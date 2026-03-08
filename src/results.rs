use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::config::{Config, Direction};

/// A single row from results.tsv.
struct Run {
    commit: String,
    metric_value: f64,
    #[allow(dead_code)]
    constraint_values: Vec<f64>,
    status: String,
    description: String,
}

/// Parse results.tsv and print a formatted summary.
pub fn show_results(config: &Config, tsv_path: &Path) -> Result<()> {
    let contents = std::fs::read_to_string(tsv_path)
        .with_context(|| format!("reading {}", tsv_path.display()))?;

    let mut lines = contents.lines();
    let header = lines.next().context("results.tsv is empty")?;

    // Validate header matches config
    let expected_cols = config.tsv_columns();
    let actual_cols: Vec<&str> = header.split('\t').collect();
    if actual_cols.len() != expected_cols.len() {
        bail!(
            "header has {} columns, expected {} (from config)",
            actual_cols.len(),
            expected_cols.len()
        );
    }

    // Parse rows
    let num_constraints = config.constraints.len();
    let mut runs: Vec<Run> = Vec::new();

    for (line_num, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 4 + num_constraints {
            bail!(
                "line {} has {} columns, expected at least {}",
                line_num + 2,
                cols.len(),
                4 + num_constraints
            );
        }

        let metric_value: f64 = cols[1]
            .trim()
            .parse()
            .with_context(|| format!("parsing metric on line {}", line_num + 2))?;

        let mut constraint_values = Vec::new();
        for i in 0..num_constraints {
            let v: f64 = cols[2 + i]
                .trim()
                .parse()
                .with_context(|| format!("parsing constraint {} on line {}", i, line_num + 2))?;
            constraint_values.push(v);
        }

        let status_idx = 2 + num_constraints;
        let desc_idx = 3 + num_constraints;

        runs.push(Run {
            commit: cols[0].trim().to_string(),
            metric_value,
            constraint_values,
            status: cols[status_idx].trim().to_string(),
            description: if desc_idx < cols.len() {
                cols[desc_idx..].join("\t").trim().to_string()
            } else {
                String::new()
            },
        });
    }

    if runs.is_empty() {
        println!("No results yet.");
        return Ok(());
    }

    let direction = config.metric.direction;
    let dir_symbol = match direction {
        Direction::Maximize => "^",
        Direction::Minimize => "v",
    };

    // Stats
    let total = runs.len();
    let kept = runs.iter().filter(|r| r.status == "keep").count();
    let discarded = runs.iter().filter(|r| r.status == "discard").count();
    let crashed = runs.iter().filter(|r| r.status == "crash").count();

    let kept_runs: Vec<&Run> = runs.iter().filter(|r| r.status == "keep").collect();

    let best = match direction {
        Direction::Maximize => kept_runs.iter().max_by(|a, b| {
            a.metric_value
                .partial_cmp(&b.metric_value)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        Direction::Minimize => kept_runs.iter().min_by(|a, b| {
            a.metric_value
                .partial_cmp(&b.metric_value)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    };

    // Find baseline (first row)
    let baseline_value = runs[0].metric_value;

    // Header
    println!();
    println!(
        "  {} — {} {} ({}) ",
        config.name,
        config.metric.name,
        dir_symbol,
        match direction {
            Direction::Maximize => "higher is better",
            Direction::Minimize => "lower is better",
        }
    );
    println!();

    // Scoreboard
    println!(
        "  {:<10} {:>12}  {:<8} {:>1} {:<8} {}",
        "commit", config.metric.name, "vs base", "", "status", "description"
    );
    println!("  {}", "-".repeat(76));

    for run in &runs {
        // Show improvement relative to baseline
        let ratio = if baseline_value != 0.0 {
            run.metric_value / baseline_value
        } else {
            1.0
        };

        let delta_str = if run.commit == "baseline" {
            String::new()
        } else if ratio >= 10.0 || ratio <= 0.1 {
            // Use multiplier for large changes
            format!("{:.0}x", ratio)
        } else {
            let pct = (ratio - 1.0) * 100.0;
            format!("{:+.1}%", pct)
        };

        let status_marker = match run.status.as_str() {
            "keep" => "+",
            "discard" => "-",
            "crash" => "!",
            _ => "?",
        };

        println!(
            "  {:<10} {:>12.2}  {:<8} {} {:<8} {}",
            run.commit, run.metric_value, delta_str, status_marker, run.status, run.description
        );
    }

    println!("  {}", "-".repeat(76));

    // Summary
    println!();
    println!(
        "  experiments: {}  (kept: {}, discarded: {}, crashed: {})",
        total, kept, discarded, crashed
    );

    if let Some(best) = best {
        let ratio = if baseline_value != 0.0 {
            best.metric_value / baseline_value
        } else {
            1.0
        };
        let improvement_str = if ratio >= 10.0 || ratio <= 0.1 {
            format!("{:.0}x vs baseline", ratio)
        } else {
            let pct = (ratio - 1.0) * 100.0;
            format!("{:+.1}% vs baseline", pct)
        };
        println!(
            "  best:        {} = {:.2}  ({})  [{}]",
            config.metric.name, best.metric_value, improvement_str, best.commit
        );
        println!(
            "  baseline:    {} = {:.2}",
            config.metric.name, baseline_value
        );
    }

    println!();

    Ok(())
}
