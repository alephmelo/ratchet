use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::{Config, Direction};

/// Parsed metric from benchmark output.
struct MetricResult {
    name: String,
    value: f64,
}

/// Extract a value from a line matching "name: value" given a grep pattern like "^name:".
fn try_parse_metric(line: &str, _name: &str, grep: &str) -> Option<f64> {
    // The grep patterns are like "^throughput:" — we just check if the line
    // starts with the prefix (strip the ^ anchor if present).
    let prefix = grep.strip_prefix('^').unwrap_or(grep);
    if !line.starts_with(prefix) {
        return None;
    }
    // Extract the value after the prefix
    let rest = line[prefix.len()..].trim();
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

    // Run the command via shell
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&config.run)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start benchmark command")?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let reader = BufReader::new(stdout);

    // Collect all output lines and parse metrics as they come
    let mut output_lines: Vec<String> = Vec::new();
    let mut metrics: Vec<MetricResult> = Vec::new();

    for line in reader.lines() {
        let line = line.context("reading benchmark output")?;

        // Check primary metric
        if let Some(value) = try_parse_metric(&line, &config.metric.name, &config.metric.grep) {
            metrics.push(MetricResult {
                name: config.metric.name.clone(),
                value,
            });
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
        println!(
            "    metric: {} (grep: {})",
            config.metric.name, config.metric.grep
        );
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

    // Find primary metric
    let primary = metrics.iter().find(|m| m.name == config.metric.name);

    println!("  results:");
    println!();

    for m in &metrics {
        let is_primary = m.name == config.metric.name;
        let marker = if is_primary {
            match config.metric.direction {
                Direction::Maximize => " ^",
                Direction::Minimize => " v",
            }
        } else {
            ""
        };
        println!("    {:<20} {:>12.2}{}", m.name, m.value, marker);
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
                        "{} = {:.2} (fail_above: {})",
                        m.name, m.value, limit
                    ));
                }
            }
            if let Some(limit) = constraint.fail_below {
                if m.value < limit {
                    violations.push(format!(
                        "{} = {:.2} (fail_below: {})",
                        m.name, m.value, limit
                    ));
                }
            }
            if let Some(limit) = constraint.warn_above {
                if m.value > limit {
                    warnings.push(format!(
                        "{} = {:.2} (warn_above: {})",
                        m.name, m.value, limit
                    ));
                }
            }
            if let Some(limit) = constraint.warn_below {
                if m.value < limit {
                    warnings.push(format!(
                        "{} = {:.2} (warn_below: {})",
                        m.name, m.value, limit
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
        if let Some(&baseline_val) = baseline.get(&config.metric.name) {
            let ratio = if baseline_val != 0.0 {
                primary.value / baseline_val
            } else {
                1.0
            };

            let is_better = match config.metric.direction {
                Direction::Maximize => primary.value > baseline_val,
                Direction::Minimize => primary.value < baseline_val,
            };

            println!();
            if ratio >= 10.0 || ratio <= 0.1 {
                println!(
                    "  vs baseline: {:.2} -> {:.2} ({:.0}x) {}",
                    baseline_val,
                    primary.value,
                    ratio,
                    if is_better { "BETTER" } else { "WORSE" }
                );
            } else {
                let pct = (ratio - 1.0) * 100.0;
                println!(
                    "  vs baseline: {:.2} -> {:.2} ({:+.1}%) {}",
                    baseline_val,
                    primary.value,
                    pct,
                    if is_better { "BETTER" } else { "WORSE" }
                );
            }
        }
    }

    println!();

    Ok(())
}
