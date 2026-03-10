use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::{format_metric, Config, Direction};

/// Parsed metric from benchmark output.
struct MetricResult {
    name: String,
    value: f64,
}

/// Extract a value from a line matching "name: value" given a grep pattern like "^name:".
fn try_parse_metric(line: &str, _name: &str, grep: &str) -> Option<f64> {
    // Strip the ^ anchor if present, then search for the pattern anywhere
    // in the line so log-prefixed output (e.g. Python logging with timestamps)
    // still matches.
    let prefix = grep.strip_prefix('^').unwrap_or(grep);
    let pos = line.find(prefix)?;
    // Extract the value after the matched pattern
    let rest = line[pos + prefix.len()..].trim();
    rest.parse::<f64>().ok().or_else(|| {
        // Try parsing just the first token (in case there's extra text)
        rest.split_whitespace().next()?.parse::<f64>().ok()
    })
}

/// Run the benchmark command, parse metrics, and display results.
pub fn run_benchmark(config: &Config) -> Result<()> {
    println!("  running: {}", config.run);
    println!();

    let start = Instant::now();
    let timeout = Duration::from_secs(config.timeout);

    // Run the command via shell, merging stderr into stdout so we capture
    // metrics from both streams (many programs, e.g. Python logging, write to stderr)
    let merged_cmd = format!("{} 2>&1", &config.run);
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&merged_cmd)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to start benchmark command")?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let reader = BufReader::new(stdout);

    // Collect all output lines and parse metrics as they come
    let mut output_lines: Vec<String> = Vec::new();
    let mut metrics: Vec<MetricResult> = Vec::new();

    let first_metric = config.first_metric();

    for line in reader.lines() {
        let line = line.context("reading benchmark output")?;

        // Display benchmark output live
        println!("  | {}", line);

        // Check all primary metrics
        for pm in config.primary_metrics() {
            if let Some(value) = try_parse_metric(&line, &pm.name, &pm.grep) {
                metrics.push(MetricResult {
                    name: pm.name.clone(),
                    value,
                });
            }
        }

        // Check constraints
        for constraint in &config.constraints {
            if let Some(value) = try_parse_metric(&line, &constraint.name, &constraint.grep) {
                metrics.push(MetricResult {
                    name: constraint.name.clone(),
                    value,
                });
            }
        }

        output_lines.push(line);

        // Check timeout
        if start.elapsed() > timeout {
            let _ = child.kill();
            bail!("benchmark exceeded timeout ({}s), killed", config.timeout);
        }
    }

    let status = child.wait().context("waiting for benchmark to finish")?;
    let elapsed = start.elapsed();

    if !status.success() {
        println!("  CRASHED (exit code: {})", status.code().unwrap_or(-1));
        println!();

        // Show last 20 lines of output for debugging
        let tail: Vec<&String> = output_lines.iter().rev().take(20).collect();
        for line in tail.iter().rev() {
            println!("  | {}", line);
        }
        println!();
        bail!("benchmark exited with non-zero status");
    }

    // Display results
    if metrics.is_empty() {
        println!("  WARNING: no metrics found in output");
        println!("  Make sure your benchmark prints lines matching the grep patterns:");
        for pm in config.primary_metrics() {
            println!("    metric: {} (grep: {})", pm.name, pm.grep);
        }
        for c in &config.constraints {
            println!("    constraint: {} (grep: {})", c.name, c.grep);
        }
        println!();
        println!("  raw output ({} lines):", output_lines.len());
        for line in &output_lines {
            println!("  | {}", line);
        }
        return Ok(());
    }

    // Find primary metric (first one for display)
    let primary = metrics.iter().find(|m| m.name == first_metric.name);

    println!("  results:");
    println!();

    for m in &metrics {
        let is_primary = m.name == first_metric.name;
        let marker = if is_primary {
            match first_metric.direction {
                Direction::Maximize => " ^",
                Direction::Minimize => " v",
            }
        } else {
            ""
        };
        println!(
            "    {:<20} {:>12}{}",
            m.name,
            format_metric(m.value),
            marker
        );
    }

    println!();
    println!("  elapsed: {:.2}s", elapsed.as_secs_f64());

    // Check constraints
    let mut violations = Vec::new();
    let mut warnings = Vec::new();

    for constraint in &config.constraints {
        if let Some(m) = metrics.iter().find(|m| m.name == constraint.name) {
            if let Some(limit) = constraint.fail_above {
                if m.value > limit {
                    violations.push(format!(
                        "{} = {} (fail_above: {})",
                        m.name,
                        format_metric(m.value),
                        limit
                    ));
                }
            }
            if let Some(limit) = constraint.fail_below {
                if m.value < limit {
                    violations.push(format!(
                        "{} = {} (fail_below: {})",
                        m.name,
                        format_metric(m.value),
                        limit
                    ));
                }
            }
            if let Some(limit) = constraint.warn_above {
                if m.value > limit {
                    warnings.push(format!(
                        "{} = {} (warn_above: {})",
                        m.name,
                        format_metric(m.value),
                        limit
                    ));
                }
            }
            if let Some(limit) = constraint.warn_below {
                if m.value < limit {
                    warnings.push(format!(
                        "{} = {} (warn_below: {})",
                        m.name,
                        format_metric(m.value),
                        limit
                    ));
                }
            }
        }
    }

    if !warnings.is_empty() {
        println!();
        println!("  warnings:");
        for w in &warnings {
            println!("    WARN  {}", w);
        }
    }

    if !violations.is_empty() {
        println!();
        println!("  constraint violations:");
        for v in &violations {
            println!("    FAIL  {}", v);
        }
    }

    // Compare with baseline if available
    if let (Some(primary), Some(baseline)) = (primary, &config.baseline) {
        if let Some(&baseline_val) = baseline.get(&first_metric.name) {
            let ratio = if baseline_val != 0.0 {
                primary.value / baseline_val
            } else {
                1.0
            };

            let is_better = match first_metric.direction {
                Direction::Maximize => primary.value > baseline_val,
                Direction::Minimize => primary.value < baseline_val,
            };

            println!();
            if ratio >= 10.0 || ratio <= 0.1 {
                println!(
                    "  vs baseline: {} -> {} ({:.0}x) {}",
                    format_metric(baseline_val),
                    format_metric(primary.value),
                    ratio,
                    if is_better { "BETTER" } else { "WORSE" }
                );
            } else {
                let pct = (ratio - 1.0) * 100.0;
                println!(
                    "  vs baseline: {} -> {} ({:+.1}%) {}",
                    format_metric(baseline_val),
                    format_metric(primary.value),
                    pct,
                    if is_better { "BETTER" } else { "WORSE" }
                );
            }
        }
    }

    println!();

    Ok(())
}
