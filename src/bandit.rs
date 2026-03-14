use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single arm in the multi-armed bandit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arm {
    /// Short identifier, e.g. "algorithm".
    pub name: String,
    /// Prompt text describing this strategy to the agent.
    pub description: String,
    /// Number of times this arm has been selected.
    pub pulls: usize,
    /// Number of times this arm led to a "keep" outcome.
    pub rewards: usize,
}

impl Arm {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            pulls: 0,
            rewards: 0,
        }
    }

    /// Empirical success rate (mean reward).
    fn mean_reward(&self) -> f64 {
        if self.pulls == 0 {
            return 0.0;
        }
        self.rewards as f64 / self.pulls as f64
    }
}

/// Persistent state for the UCB1 multi-armed bandit.
#[derive(Debug, Serialize, Deserialize)]
pub struct BanditState {
    pub arms: Vec<Arm>,
    /// UCB1 exploration constant (default: sqrt(2) ≈ 1.41).
    pub exploration_c: f64,
    /// Total pulls across all arms (cached for convenience).
    pub total_pulls: usize,
}

impl BanditState {
    /// Create a new bandit with the default set of strategy arms.
    pub fn new(exploration_c: f64) -> Self {
        let arms = vec![
            Arm::new(
                "algorithm",
                "Try a fundamentally different algorithm or approach for the core computation. \
                 Consider entirely different algorithmic families, mathematical reformulations, \
                 or approaches from other domains that solve the same underlying problem.",
            ),
            Arm::new(
                "data-structure",
                "Change how data is stored or accessed. Consider different containers \
                 (hash maps vs. B-trees vs. arrays), different representations \
                 (sparse vs. dense, SOA vs. AOS), or indexed/sorted structures that \
                 enable faster lookups.",
            ),
            Arm::new(
                "micro-optimization",
                "Apply targeted low-level optimizations to the hot path. Reduce allocations, \
                 avoid unnecessary copies, eliminate redundant computation, optimize branching, \
                 use cheaper operations, or add caching/memoization for repeated work.",
            ),
            Arm::new(
                "parallelism",
                "Add or improve concurrency, SIMD, batching, or parallel processing. \
                 Consider whether work can be split across threads, whether loops can be \
                 vectorized, or whether operations can be batched to amortize overhead.",
            ),
            Arm::new(
                "memory-layout",
                "Optimize memory access patterns for better cache utilization. Consider \
                 cache-friendly data layouts, reducing pointer indirection, improving spatial \
                 locality, prefetching, or reducing the working set size.",
            ),
            Arm::new(
                "rewrite",
                "Rewrite a significant section of the code from scratch using a completely \
                 different approach. Don't try to evolve the existing code — start fresh \
                 with a new design for the critical section.",
            ),
        ];

        Self {
            arms,
            exploration_c,
            total_pulls: 0,
        }
    }

    /// Select the arm with the highest UCB1 score.
    ///
    /// UCB1: score = mean_reward + c * sqrt(ln(total_pulls) / pulls_i)
    ///
    /// Arms with zero pulls get infinite score (explored first).
    pub fn select_arm(&self) -> &Arm {
        // If any arm has never been pulled, pick the first unpulled one.
        if let Some(arm) = self.arms.iter().find(|a| a.pulls == 0) {
            return arm;
        }

        let ln_total = (self.total_pulls as f64).ln();

        let mut best_score = f64::NEG_INFINITY;
        let mut best_arm = &self.arms[0];

        for arm in &self.arms {
            let exploitation = arm.mean_reward();
            let exploration = self.exploration_c * (ln_total / arm.pulls as f64).sqrt();
            let score = exploitation + exploration;

            if score > best_score {
                best_score = score;
                best_arm = arm;
            }
        }

        best_arm
    }

    /// Record the outcome of pulling an arm.
    ///
    /// `kept` is true if the experiment was kept (reward = 1), false otherwise (reward = 0).
    pub fn update(&mut self, arm_name: &str, kept: bool) {
        if let Some(arm) = self.arms.iter_mut().find(|a| a.name == arm_name) {
            arm.pulls += 1;
            if kept {
                arm.rewards += 1;
            }
        }
        self.total_pulls += 1;
    }

    /// Load bandit state from a JSON file, or create a new default state if the file doesn't exist.
    pub fn load(path: &Path, exploration_c: f64) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new(exploration_c));
        }

        let contents =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut state: BanditState = serde_json::from_str(&contents)
            .with_context(|| format!("parsing {}", path.display()))?;

        // Override exploration constant from config (allows tuning between runs).
        state.exploration_c = exploration_c;

        // Ensure all default arms exist (in case we add new ones in a future version).
        let default = Self::new(exploration_c);
        for default_arm in &default.arms {
            if !state.arms.iter().any(|a| a.name == default_arm.name) {
                state.arms.push(default_arm.clone());
            }
        }

        Ok(state)
    }

    /// Save bandit state to a JSON file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("serializing bandit state")?;
        std::fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

/// Default UCB1 exploration constant: sqrt(2).
pub const DEFAULT_EXPLORATION_C: f64 = 1.41421356;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bandit_has_six_arms() {
        let bandit = BanditState::new(DEFAULT_EXPLORATION_C);
        assert_eq!(bandit.arms.len(), 6);
        assert_eq!(bandit.total_pulls, 0);
    }

    #[test]
    fn test_unpulled_arms_selected_first() {
        let bandit = BanditState::new(DEFAULT_EXPLORATION_C);
        // First selection should be the first arm (all unpulled).
        let arm = bandit.select_arm();
        assert_eq!(arm.name, "algorithm");
    }

    #[test]
    fn test_round_robin_unpulled() {
        let mut bandit = BanditState::new(DEFAULT_EXPLORATION_C);

        // Pulling each arm once should round-robin through all of them.
        let mut selected: Vec<String> = Vec::new();
        for _ in 0..6 {
            let arm = bandit.select_arm();
            selected.push(arm.name.clone());
            bandit.update(&arm.name.clone(), false);
        }

        assert_eq!(selected.len(), 6);
        // All six distinct arms should have been selected.
        let mut unique = selected.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn test_exploitation_favors_high_reward() {
        let mut bandit = BanditState::new(0.0); // Zero exploration = pure exploitation.

        // Pull all arms once with no reward, except "algorithm" which gets a reward.
        for arm in &bandit.arms.clone() {
            bandit.update(&arm.name, arm.name == "algorithm");
        }

        // With zero exploration constant, should always pick the arm with highest mean.
        let arm = bandit.select_arm();
        assert_eq!(arm.name, "algorithm");
    }

    #[test]
    fn test_update_increments() {
        let mut bandit = BanditState::new(DEFAULT_EXPLORATION_C);

        bandit.update("algorithm", true);
        bandit.update("algorithm", false);
        bandit.update("algorithm", true);

        let arm = bandit.arms.iter().find(|a| a.name == "algorithm").unwrap();
        assert_eq!(arm.pulls, 3);
        assert_eq!(arm.rewards, 2);
        assert_eq!(bandit.total_pulls, 3);
    }

    #[test]
    fn test_mean_reward() {
        let mut arm = Arm::new("test", "test arm");
        assert_eq!(arm.mean_reward(), 0.0);

        arm.pulls = 4;
        arm.rewards = 3;
        assert!((arm.mean_reward() - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_save_and_load() {
        let dir = std::env::temp_dir().join("ratchet_bandit_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("bandit.json");

        let mut bandit = BanditState::new(DEFAULT_EXPLORATION_C);
        bandit.update("algorithm", true);
        bandit.update("rewrite", false);
        bandit.save(&path).unwrap();

        let loaded = BanditState::load(&path, DEFAULT_EXPLORATION_C).unwrap();
        assert_eq!(loaded.total_pulls, 2);
        let alg = loaded.arms.iter().find(|a| a.name == "algorithm").unwrap();
        assert_eq!(alg.pulls, 1);
        assert_eq!(alg.rewards, 1);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
