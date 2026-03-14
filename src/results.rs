use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::config::{format_metric, Config, Direction};

/// A single row from results.tsv.
struct Run {
    commit: String,
    metric_values: Vec<f64>,
    #[allow(dead_code)]
    constraint_values: Vec<f64>,
    strategy: String,
    status: String,
    description: String,
}

impl Run {
    fn first_metric(&self) -> f64 {
        self.metric_values.first().copied().unwrap_or(0.0)
    }
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
    // Support both old format (without strategy column) and new format (with strategy column).
    let has_strategy_col = actual_cols.iter().any(|c| c.trim() == "strategy");
    let expected_len = if has_strategy_col {
        expected_cols.len()
    } else {
        expected_cols.len() - 1 // old format lacks strategy column
    };
    if actual_cols.len() != expected_len {
        bail!(
            "header has {} columns, expected {} (from config)",
            actual_cols.len(),
            expected_len
        );
    }

    // Parse rows
    let primary = config.primary_metrics();
    let num_metrics = primary.len();
    let num_constraints = config.constraints.len();
    let mut runs: Vec<Run> = Vec::new();

    for (line_num, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        let min_cols = if has_strategy_col {
            1 + num_metrics + num_constraints + 3 // commit + metrics + constraints + strategy + status + description
        } else {
            1 + num_metrics + num_constraints + 2
        };
        if cols.len() < min_cols {
            bail!(
                "line {} has {} columns, expected at least {}",
                line_num + 2,
                cols.len(),
                min_cols
            );
        }

        let mut metric_values = Vec::with_capacity(num_metrics);
        for i in 0..num_metrics {
            let v: f64 = cols[1 + i]
                .trim()
                .parse()
                .with_context(|| format!("parsing metric {} on line {}", i, line_num + 2))?;
            metric_values.push(v);
        }

        let mut constraint_values = Vec::new();
        for i in 0..num_constraints {
            let v: f64 = cols[1 + num_metrics + i]
                .trim()
                .parse()
                .with_context(|| format!("parsing constraint {} on line {}", i, line_num + 2))?;
            constraint_values.push(v);
        }

        let (strategy, status_idx, desc_idx) = if has_strategy_col {
            let strat_idx = 1 + num_metrics + num_constraints;
            let stat_idx = strat_idx + 1;
            let d_idx = stat_idx + 1;
            (cols[strat_idx].trim().to_string(), stat_idx, d_idx)
        } else {
            let stat_idx = 1 + num_metrics + num_constraints;
            let d_idx = stat_idx + 1;
            (String::new(), stat_idx, d_idx)
        };

        runs.push(Run {
            commit: cols[0].trim().to_string(),
            metric_values,
            constraint_values,
            strategy,
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

    let is_multi = config.is_multi_metric();
    let first_metric = config.first_metric();
    let direction = first_metric.direction;

    // Stats
    let total = runs.len();
    let kept = runs.iter().filter(|r| r.status == "keep").count();
    let discarded = runs.iter().filter(|r| r.status == "discard").count();
    let crashed = runs.iter().filter(|r| r.status == "crash").count();

    let kept_runs: Vec<&Run> = runs.iter().filter(|r| r.status == "keep").collect();

    // Find baseline (first row)
    let baseline_values: Vec<f64> = runs[0].metric_values.clone();

    // Header
    println!();
    if is_multi {
        let metric_desc: Vec<String> = primary
            .iter()
            .map(|m| {
                let sym = match m.direction {
                    Direction::Maximize => "^",
                    Direction::Minimize => "v",
                };
                format!("{} {}", m.name, sym)
            })
            .collect();
        println!("  {} — Pareto: {} ", config.name, metric_desc.join(", "));
    } else {
        let dir_symbol = match direction {
            Direction::Maximize => "^",
            Direction::Minimize => "v",
        };
        println!(
            "  {} — {} {} ({}) ",
            config.name,
            first_metric.name,
            dir_symbol,
            match direction {
                Direction::Maximize => "higher is better",
                Direction::Minimize => "lower is better",
            }
        );
    }
    println!();

    // Check if any runs have a strategy set (non-empty, not "-")
    let show_strategy = runs
        .iter()
        .any(|r| !r.strategy.is_empty() && r.strategy != "-");

    // Scoreboard header
    if is_multi {
        let mut header_parts = vec![format!("  {:<10}", "commit")];
        for m in &primary {
            header_parts.push(format!("{:>14}", m.name));
        }
        if show_strategy {
            header_parts.push(format!("  {:<16}", "strategy"));
        }
        header_parts.push(format!(
            "  {:<8} {:>1} {:<8} {}",
            "vs base", "", "status", "description"
        ));
        println!("{}", header_parts.join(""));
    } else {
        if show_strategy {
            println!(
                "  {:<10} {:>12}  {:<8} {:>1} {:<8} {:<16} {}",
                "commit", first_metric.name, "vs base", "", "status", "strategy", "description"
            );
        } else {
            println!(
                "  {:<10} {:>12}  {:<8} {:>1} {:<8} {}",
                "commit", first_metric.name, "vs base", "", "status", "description"
            );
        }
    }
    let separator_len = if is_multi {
        76 + 14 * (num_metrics - 1) + if show_strategy { 18 } else { 0 }
    } else {
        76 + if show_strategy { 18 } else { 0 }
    };
    println!("  {}", "-".repeat(separator_len));

    for run in &runs {
        // Show improvement relative to baseline (based on first metric)
        let first_val = run.first_metric();
        let baseline_first = baseline_values[0];
        let ratio = if baseline_first != 0.0 {
            first_val / baseline_first
        } else {
            1.0
        };

        let delta_str = if run.commit == "baseline" {
            String::new()
        } else if ratio >= 10.0 || ratio <= 0.1 {
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

        if is_multi {
            let mut parts = vec![format!("  {:<10}", run.commit)];
            for val in &run.metric_values {
                parts.push(format!("{:>14}", format_metric(*val)));
            }
            if show_strategy {
                let strat_display = if run.strategy.is_empty() || run.strategy == "-" {
                    "-".to_string()
                } else {
                    run.strategy.clone()
                };
                parts.push(format!("  {:<16}", strat_display));
            }
            parts.push(format!(
                "  {:<8} {} {:<8} {}",
                delta_str, status_marker, run.status, run.description
            ));
            println!("{}", parts.join(""));
        } else {
            if show_strategy {
                let strat_display = if run.strategy.is_empty() || run.strategy == "-" {
                    "-".to_string()
                } else {
                    run.strategy.clone()
                };
                println!(
                    "  {:<10} {:>12}  {:<8} {} {:<8} {:<16} {}",
                    run.commit,
                    format_metric(first_val),
                    delta_str,
                    status_marker,
                    run.status,
                    strat_display,
                    run.description
                );
            } else {
                println!(
                    "  {:<10} {:>12}  {:<8} {} {:<8} {}",
                    run.commit,
                    format_metric(first_val),
                    delta_str,
                    status_marker,
                    run.status,
                    run.description
                );
            }
        }
    }

    println!("  {}", "-".repeat(separator_len));

    // Summary
    println!();
    println!(
        "  experiments: {}  (kept: {}, discarded: {}, crashed: {})",
        total, kept, discarded, crashed
    );

    if is_multi {
        // Show best for each metric individually
        for (i, m) in primary.iter().enumerate() {
            let best = match m.direction {
                Direction::Maximize => kept_runs.iter().max_by(|a, b| {
                    a.metric_values[i]
                        .partial_cmp(&b.metric_values[i])
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
                Direction::Minimize => kept_runs.iter().min_by(|a, b| {
                    a.metric_values[i]
                        .partial_cmp(&b.metric_values[i])
                        .unwrap_or(std::cmp::Ordering::Equal)
                }),
            };
            if let Some(best) = best {
                let baseline_val = baseline_values[i];
                let improvement_str = if baseline_val != 0.0 {
                    let r = best.metric_values[i] / baseline_val;
                    if r >= 10.0 || r <= 0.1 {
                        format!("{:.0}x vs baseline", r)
                    } else {
                        let pct = (r - 1.0) * 100.0;
                        format!("{:+.1}% vs baseline", pct)
                    }
                } else {
                    String::new()
                };
                println!(
                    "  best {}: {}  ({})  [{}]",
                    m.name,
                    format_metric(best.metric_values[i]),
                    improvement_str,
                    best.commit
                );
            }
        }
        for (i, m) in primary.iter().enumerate() {
            println!(
                "  baseline:  {} = {}",
                m.name,
                format_metric(baseline_values[i])
            );
        }
    } else {
        let best = match direction {
            Direction::Maximize => kept_runs.iter().max_by(|a, b| {
                a.first_metric()
                    .partial_cmp(&b.first_metric())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            Direction::Minimize => kept_runs.iter().min_by(|a, b| {
                a.first_metric()
                    .partial_cmp(&b.first_metric())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        };

        if let Some(best) = best {
            let baseline_val = baseline_values[0];
            let ratio = if baseline_val != 0.0 {
                best.first_metric() / baseline_val
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
                "  best:        {} = {}  ({})  [{}]",
                first_metric.name,
                format_metric(best.first_metric()),
                improvement_str,
                best.commit
            );
            println!(
                "  baseline:    {} = {}",
                first_metric.name,
                format_metric(baseline_values[0])
            );
        }
    }

    println!();

    Ok(())
}
