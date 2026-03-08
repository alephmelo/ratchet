use anyhow::{Context, Result};
use std::path::Path;

use crate::config::{format_metric, Config, Direction};

/// A data point for the chart.
struct DataPoint {
    label: String,
    /// One value per primary metric (in order).
    values: Vec<f64>,
    status: String,
}

/// Parse results.tsv into data points with all metric values.
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
        let mut values = Vec::with_capacity(num_metrics);
        for i in 0..num_metrics {
            let v: f64 = cols[1 + i].trim().parse().unwrap_or(0.0);
            values.push(v);
        }
        let status_idx = 1 + num_metrics + num_constraints;

        points.push(DataPoint {
            label: if commit == "baseline" {
                "base".to_string()
            } else if commit.len() > 7 {
                commit[..7].to_string()
            } else {
                commit.to_string()
            },
            values,
            status: cols[status_idx].trim().to_string(),
        });
    }

    Ok(points)
}

/// Render a horizontal bar chart for one metric.
fn render_chart(
    points: &[DataPoint],
    metric_idx: usize,
    metric_name: &str,
    direction: Direction,
    config_name: &str,
) {
    let dir_symbol = match direction {
        Direction::Maximize => "^",
        Direction::Minimize => "v",
    };

    println!(
        "  {} — {} {} ({})",
        config_name,
        metric_name,
        dir_symbol,
        match direction {
            Direction::Maximize => "higher is better",
            Direction::Minimize => "lower is better",
        }
    );
    println!();

    // Find range
    let max_val = points
        .iter()
        .map(|p| p.values[metric_idx])
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(1.0);

    let min_val = points
        .iter()
        .map(|p| p.values[metric_idx])
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    // Chart dimensions
    let bar_max_width: usize = 50;
    let label_width: usize = 8;

    // Use 0 as the base for the bars (not min_val) so bars are proportional
    let scale_max = if max_val > 0.0 { max_val } else { 1.0 };

    // Find the best value
    let best_val = match direction {
        Direction::Maximize => max_val,
        Direction::Minimize => min_val,
    };

    for point in points.iter() {
        let val = point.values[metric_idx];
        // Scale bar width proportionally
        let bar_width = if scale_max > 0.0 {
            ((val / scale_max) * bar_max_width as f64) as usize
        } else {
            0
        };
        let bar_width = bar_width.max(1); // at least 1 char

        // Choose bar character based on status
        let (bar_char, marker) = match point.status.as_str() {
            "keep" => {
                if (val - best_val).abs() < f64::EPSILON * 100.0 {
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
            "  {} {:>label_w$} {} {}{}{}",
            status_indicator,
            point.label,
            bar,
            "",
            format_metric(val),
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

    let max_label = format_metric(scale_max);
    println!(
        "  {}0{}{}{}{}{}{}",
        " ".repeat(label_width + 4),
        " ".repeat(quarter.saturating_sub(1)),
        format_metric(mid_val / 2.0),
        " ".repeat(quarter.saturating_sub(format_metric(mid_val / 2.0).len())),
        format_metric(mid_val),
        " ".repeat(quarter.saturating_sub(format_metric(mid_val).len())),
        max_label,
    );

    // Summary
    println!();

    let baseline_val = points.first().map(|p| p.values[metric_idx]).unwrap_or(0.0);
    let kept_count = points.iter().filter(|p| p.status == "keep").count();
    let total = points.len();

    // Best kept value
    let best_kept = points
        .iter()
        .filter(|p| p.status == "keep")
        .map(|p| p.values[metric_idx])
        .max_by(|a, b| match direction {
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
            "  █ kept ({}/{})  ░ discarded/crashed  * best: {} ({})",
            kept_count,
            total,
            format_metric(best),
            improvement
        );
    } else {
        println!("  █ kept ({}/{})  ░ discarded/crashed", kept_count, total);
    }
}

/// Render a horizontal bar chart in the terminal.
pub fn show_plot(config: &Config, tsv_path: &Path) -> Result<()> {
    let points = parse_data(config, tsv_path)?;

    if points.is_empty() {
        println!("  No data to plot.");
        return Ok(());
    }

    let primary = config.primary_metrics();

    println!();

    if primary.len() > 1 {
        // Multi-metric: render one chart per metric
        for (i, m) in primary.iter().enumerate() {
            render_chart(&points, i, &m.name, m.direction, &config.name);
            if i < primary.len() - 1 {
                println!();
            }
        }
    } else {
        // Single metric: render one chart
        let m = &primary[0];
        render_chart(&points, 0, &m.name, m.direction, &config.name);
    }

    println!();

    Ok(())
}
