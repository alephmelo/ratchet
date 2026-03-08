use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub editable: Vec<String>,
    #[serde(default)]
    pub readonly: Vec<String>,
    pub run: String,
    /// Single primary metric (backward-compatible).
    #[serde(default)]
    pub metric: Option<Metric>,
    /// Multiple primary metrics (Pareto optimization). Takes precedence over `metric`.
    #[serde(default)]
    pub metrics: Option<Vec<Metric>>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub baseline: Option<HashMap<String, f64>>,
    /// Agent command template. Use {prompt} as placeholder for the prompt file path.
    /// Example: "claude --print {prompt}" or "opencode -p {prompt}"
    #[serde(default)]
    pub agent: Option<String>,
    /// Maximum number of iterations for `ratchet loop`.
    #[serde(default)]
    pub max_iterations: Option<usize>,
    /// Stop after N consecutive iterations without improvement.
    #[serde(default)]
    pub patience: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Metric {
    pub name: String,
    pub grep: String,
    pub direction: Direction,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Maximize,
    Minimize,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct Constraint {
    pub name: String,
    pub grep: String,
    #[serde(default)]
    pub warn_above: Option<f64>,
    #[serde(default)]
    pub warn_below: Option<f64>,
    #[serde(default)]
    pub fail_above: Option<f64>,
    #[serde(default)]
    pub fail_below: Option<f64>,
}

fn default_timeout() -> u64 {
    600
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let config: Config = serde_yaml::from_str(&contents)
            .with_context(|| format!("parsing {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    /// Returns the list of primary metrics. Supports both singular `metric` and plural `metrics`.
    pub fn primary_metrics(&self) -> Vec<&Metric> {
        if let Some(ref metrics) = self.metrics {
            metrics.iter().collect()
        } else if let Some(ref metric) = self.metric {
            vec![metric]
        } else {
            vec![]
        }
    }

    /// Whether this config uses multi-metric (Pareto) mode.
    pub fn is_multi_metric(&self) -> bool {
        self.metrics.as_ref().map(|m| m.len() > 1).unwrap_or(false)
    }

    /// The first (or only) primary metric — used for backward-compatible display.
    pub fn first_metric(&self) -> &Metric {
        self.primary_metrics()
            .first()
            .expect("at least one metric must be defined")
    }

    pub fn validate(&self) -> Result<()> {
        if self.editable.is_empty() {
            bail!("'editable' must list at least one file");
        }

        if self.name.trim().is_empty() {
            bail!("'name' must not be empty");
        }

        if self.run.trim().is_empty() {
            bail!("'run' must not be empty");
        }

        // Must have either metric or metrics
        let primary = self.primary_metrics();
        if primary.is_empty() {
            bail!("must define either 'metric' or 'metrics' with at least one entry");
        }

        // Cannot have both metric and metrics
        if self.metric.is_some() && self.metrics.is_some() {
            bail!("cannot define both 'metric' and 'metrics' — use one or the other");
        }

        for m in &primary {
            if m.name.trim().is_empty() {
                bail!("metric 'name' must not be empty");
            }
            if m.grep.trim().is_empty() {
                bail!("metric 'grep' must not be empty");
            }
        }

        // Check for duplicate metric names
        let mut seen = std::collections::HashSet::new();
        for m in &primary {
            if !seen.insert(&m.name) {
                bail!("duplicate metric name '{}'", m.name);
            }
        }

        if self.timeout == 0 {
            bail!("'timeout' must be greater than 0");
        }

        // Constraint names must not collide with metric names
        for c in &self.constraints {
            for m in &primary {
                if c.name == m.name {
                    bail!(
                        "constraint name '{}' collides with a primary metric name",
                        c.name
                    );
                }
            }
            if c.name.trim().is_empty() {
                bail!("constraint 'name' must not be empty");
            }
            if c.grep.trim().is_empty() {
                bail!("constraint 'grep' must not be empty");
            }
            if c.warn_above.is_none()
                && c.warn_below.is_none()
                && c.fail_above.is_none()
                && c.fail_below.is_none()
            {
                bail!(
                    "constraint '{}' must have at least one of: warn_above, warn_below, fail_above, fail_below",
                    c.name
                );
            }
        }

        // If baseline is provided, it must include all primary metrics
        if let Some(baseline) = &self.baseline {
            for m in &primary {
                if !baseline.contains_key(&m.name) {
                    bail!("baseline must include the primary metric '{}'", m.name);
                }
            }
        }

        Ok(())
    }

    /// Build the combined grep pattern for extracting all metrics from run.log.
    pub fn grep_pattern(&self) -> String {
        let mut patterns: Vec<String> = self
            .primary_metrics()
            .iter()
            .map(|m| m.grep.clone())
            .collect();
        for c in &self.constraints {
            patterns.push(c.grep.clone());
        }
        patterns.join("\\|")
    }

    /// Build the TSV header columns.
    pub fn tsv_columns(&self) -> Vec<String> {
        let mut cols = vec!["commit".to_string()];
        for m in self.primary_metrics() {
            cols.push(m.name.clone());
        }
        for c in &self.constraints {
            cols.push(c.name.clone());
        }
        cols.push("status".to_string());
        cols.push("description".to_string());
        cols
    }
}

/// Format a metric value with adaptive precision.
/// Uses enough decimal places to show at least 3 significant digits,
/// but always at least 2 decimal places.
pub fn format_metric(value: f64) -> String {
    if value == 0.0 {
        return "0.00".to_string();
    }
    let abs = value.abs();
    if abs >= 1.0 {
        // For values >= 1, two decimal places is fine
        format!("{:.2}", value)
    } else {
        // For values < 1, we need more precision.
        // Find how many leading zeros after the decimal point, then show 3 sig digits.
        let digits_after_dot = if abs > 0.0 {
            let log = -abs.log10().floor() as usize;
            // e.g. 0.001 → log10 = -3 → 3 leading zeros → need 3 + 2 = 5 digits
            // 0.29 → log10 = -0.537 → 0 leading zeros → need 0 + 2 = 2 digits
            log + 2
        } else {
            2
        };
        let precision = digits_after_dot.max(2);
        format!("{:.prec$}", value, prec = precision)
    }
}
