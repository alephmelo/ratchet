use anyhow::{Context, Result};
use std::path::Path;

use crate::config::{Config, Direction};

/// A data point for the chart.
struct DataPoint {
    label: String,
    value: f64,
    status: String,
}

/// Parse results.tsv into data points.
fn parse_data(config: &Config, tsv_path: &Path) -> Result<Vec<DataPoint>> {
    let contents = std::fs::read_to_string(tsv_path)
        .with_context(|| format!("reading {}", tsv_path.display()))?;

    let mut lines = contents.lines();
    let _header = lines.next().context("results.tsv is empty")?;

    let num_metrics = config.primary_metrics().len();
    let num_constraints = config.constraints.len();
    let mut points = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        let min_cols = 1 + num_metrics + num_constraints + 2;
        if cols.len() < min_cols {
            continue;
        }

        let commit = cols[0].trim();
        let value: f64 = cols[1].trim().parse().unwrap_or(0.0);
        let status_idx = 1 + num_metrics + num_constraints;

        points.push(DataPoint {
            label: if commit == "baseline" {
                "base".to_string()
            } else if commit.len() > 7 {
                commit[..7].to_string()
            } else {
                commit.to_string()
            },
            value,
            status: cols[status_idx].trim().to_string(),
        });
    }

    Ok(points)
}

/// Render a horizontal bar chart in the terminal.
pub fn show_plot(config: &Config, tsv_path: &Path) -> Result<()> {
    let points = parse_data(config, tsv_path)?;

    if points.is_empty() {
        println!("  No data to plot.");
        return Ok(());
    }

    let first_metric = config.first_metric();
    let dir_symbol = match first_metric.direction {
        Direction::Maximize => "^",
        Direction::Minimize => "v",
    };

    println!();
    println!(
        "  {} — {} {} ({})",
        config.name,
        first_metric.name,
        dir_symbol,
        match first_metric.direction {
            Direction::Maximize => "higher is better",
            Direction::Minimize => "lower is better",
        }
    );
    println!();

    // Find range
    let max_val = points
        .iter()
        .map(|p| p.value)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(1.0);

    let min_val = points
        .iter()
        .map(|p| p.value)
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    // Chart dimensions
    let bar_max_width: usize = 50;
    let label_width: usize = 8;

    // Use 0 as the base for the bars (not min_val) so bars are proportional
    let scale_max = if max_val > 0.0 { max_val } else { 1.0 };

    // Find the best value
    let best_val = match first_metric.direction {
        Direction::Maximize => max_val,
        Direction::Minimize => min_val,
    };

    for point in points.iter() {
        // Scale bar width proportionally
        let bar_width = if scale_max > 0.0 {
            ((point.value / scale_max) * bar_max_width as f64) as usize
        } else {
            0
        };
        let bar_width = bar_width.max(1); // at least 1 char

        // Choose bar character based on status
        let (bar_char, marker) = match point.status.as_str() {
            "keep" => {
                if (point.value - best_val).abs() < f64::EPSILON * 100.0 {
                    ('█', " *") // best result
                } else {
                    ('█', "")
                }
            }
            "discard" => ('░', ""),
            "crash" => ('░', " !"),
            _ => ('▒', ""),
        };

        let bar: String = std::iter::repeat(bar_char).take(bar_width).collect();

        // Color-code via status marker
        let status_indicator = match point.status.as_str() {
            "keep" => "+",
            "discard" => "-",
            "crash" => "!",
            _ => "?",
        };

        println!(
            "  {} {:>label_w$} {} {}{:>10.2}{}",
            status_indicator,
            point.label,
            bar,
            "",
            point.value,
            marker,
            label_w = label_width,
        );
    }

    // Axis line
    println!(
        "  {}{}{}",
        " ".repeat(label_width + 3),
        "└",
        "─".repeat(bar_max_width + 12)
    );

    // Scale labels
    let mid_val = scale_max / 2.0;
    let quarter = bar_max_width / 4;

    println!(
        "  {}0{}{:.0}{}{:.0}{}{:.0}",
        " ".repeat(label_width + 4),
        " ".repeat(quarter.saturating_sub(1)),
        mid_val / 2.0,
        " ".repeat(quarter.saturating_sub(format!("{:.0}", mid_val / 2.0).len())),
        mid_val,
        " ".repeat(quarter.saturating_sub(format!("{:.0}", mid_val).len())),
        scale_max,
    );

    // Summary
    println!();

    let baseline_val = points.first().map(|p| p.value).unwrap_or(0.0);
    let kept_count = points.iter().filter(|p| p.status == "keep").count();
    let total = points.len();

    // Best kept value
    let best_kept = points
        .iter()
        .filter(|p| p.status == "keep")
        .map(|p| p.value)
        .max_by(|a, b| match first_metric.direction {
            Direction::Maximize => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            Direction::Minimize => b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal),
        });

    if let Some(best) = best_kept {
        let improvement = if baseline_val != 0.0 {
            let ratio = best / baseline_val;
            if ratio >= 10.0 || ratio <= 0.1 {
                format!("{:.0}x", ratio)
            } else {
                let pct = (ratio - 1.0) * 100.0;
                format!("{:+.1}%", pct)
            }
        } else {
            String::new()
        };

        println!(
            "  █ kept ({}/{})  ░ discarded/crashed  * best: {:.2} ({})",
            kept_count, total, best, improvement
        );
    } else {
        println!("  █ kept ({}/{})  ░ discarded/crashed", kept_count, total);
    }

    println!();

    Ok(())
}
