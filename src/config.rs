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
    pub metric: Metric,
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
}

#[derive(Debug, Deserialize)]
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

        if self.metric.name.trim().is_empty() {
            bail!("'metric.name' must not be empty");
        }

        if self.metric.grep.trim().is_empty() {
            bail!("'metric.grep' must not be empty");
        }

        if self.timeout == 0 {
            bail!("'timeout' must be greater than 0");
        }

        // Constraint names must not collide with metric name
        for c in &self.constraints {
            if c.name == self.metric.name {
                bail!(
                    "constraint name '{}' collides with the primary metric name",
                    c.name
                );
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

        // If baseline is provided, it must include the primary metric
        if let Some(baseline) = &self.baseline {
            if !baseline.contains_key(&self.metric.name) {
                bail!(
                    "baseline must include the primary metric '{}'",
                    self.metric.name
                );
            }
        }

        Ok(())
    }

    /// Build the combined grep pattern for extracting all metrics from run.log.
    pub fn grep_pattern(&self) -> String {
        let mut patterns = vec![self.metric.grep.clone()];
        for c in &self.constraints {
            patterns.push(c.grep.clone());
        }
        patterns.join("\\|")
    }

    /// Build the TSV header columns.
    pub fn tsv_columns(&self) -> Vec<String> {
        let mut cols = vec!["commit".to_string(), self.metric.name.clone()];
        for c in &self.constraints {
            cols.push(c.name.clone());
        }
        cols.push("status".to_string());
        cols.push("description".to_string());
        cols
    }
}
