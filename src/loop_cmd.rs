use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::{Config, Direction};

/// A parsed row from results.tsv for building iteration prompts.
struct HistoryRow {
    num: usize,
    commit: String,
    metric: f64,
    status: String,
    description: String,
}

/// Parsed metric from benchmark output.
struct MetricResult {
    name: String,
    value: f64,
}

/// Extract a value from a line matching a grep pattern like "^name:".
fn try_parse_metric(line: &str, grep: &str) -> Option<f64> {
    let prefix = grep.strip_prefix('^').unwrap_or(grep);
    if !line.starts_with(prefix) {
        return None;
    }
    let rest = line[prefix.len()..].trim();
    rest.parse::<f64>()
        .ok()
        .or_else(|| rest.split_whitespace().next()?.parse::<f64>().ok())
}

/// Read results.tsv and return parsed history rows.
fn read_history(tsv_path: &Path, num_constraints: usize) -> Result<Vec<HistoryRow>> {
    if !tsv_path.exists() {
        return Ok(Vec::new());
    }

    let contents =
        fs::read_to_string(tsv_path).with_context(|| format!("reading {}", tsv_path.display()))?;
    let mut lines = contents.lines();
    let _header = match lines.next() {
        Some(h) => h,
        None => return Ok(Vec::new()),
    };

    let mut rows = Vec::new();
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 4 + num_constraints {
            continue;
        }

        let metric: f64 = cols[1].trim().parse().unwrap_or(0.0);
        let status_idx = 2 + num_constraints;
        let desc_idx = 3 + num_constraints;

        rows.push(HistoryRow {
            num: i + 1,
            commit: cols[0].trim().to_string(),
            metric,
            status: cols[status_idx].trim().to_string(),
            description: if desc_idx < cols.len() {
                cols[desc_idx..].join("\t").trim().to_string()
            } else {
                String::new()
            },
        });
    }

    Ok(rows)
}

/// Build the iteration prompt using MiniJinja.
fn build_iteration_prompt(
    config: &Config,
    iteration: usize,
    history: &[HistoryRow],
) -> Result<String> {
    let template_src = include_str!("../templates/iterate.md.j2");

    let env = minijinja::Environment::new();
    let tmpl = env
        .template_from_str(template_src)
        .context("parsing iterate.md.j2 template")?;

    // Read editable file contents
    let mut editable_files: Vec<HashMap<&str, String>> = Vec::new();
    for path in &config.editable {
        let content =
            fs::read_to_string(path).unwrap_or_else(|_| format!("(file not found: {})", path));
        let mut m = HashMap::new();
        m.insert("path", path.clone());
        m.insert("content", content);
        editable_files.push(m);
    }

    // Build history rows for template
    let baseline_metric = if let Some(first) = history.first() {
        first.metric
    } else if let Some(baseline) = &config.baseline {
        *baseline.get(&config.metric.name).unwrap_or(&0.0)
    } else {
        0.0
    };

    let mut history_rows: Vec<HashMap<&str, String>> = Vec::new();
    for row in history {
        let vs_baseline = if baseline_metric != 0.0 && row.commit != "baseline" {
            let ratio = row.metric / baseline_metric;
            if ratio >= 10.0 || ratio <= 0.1 {
                format!("{:.0}x", ratio)
            } else {
                let pct = (ratio - 1.0) * 100.0;
                format!("{:+.1}%", pct)
            }
        } else {
            String::new()
        };

        let mut m = HashMap::new();
        m.insert("num", row.num.to_string());
        m.insert("metric", format!("{:.2}", row.metric));
        m.insert("vs_baseline", vs_baseline);
        m.insert("status", row.status.clone());
        m.insert("description", row.description.clone());
        history_rows.push(m);
    }

    // Find best
    let kept: Vec<&HistoryRow> = history.iter().filter(|r| r.status == "keep").collect();
    let best = match config.metric.direction {
        Direction::Maximize => kept.iter().max_by(|a, b| {
            a.metric
                .partial_cmp(&b.metric)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        Direction::Minimize => kept.iter().min_by(|a, b| {
            a.metric
                .partial_cmp(&b.metric)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    };

    let best_metric = best
        .map(|b| format!("{:.2}", b.metric))
        .unwrap_or("n/a".to_string());
    let best_commit = best.map(|b| b.commit.clone()).unwrap_or_default();
    let best_vs_baseline = if let Some(b) = best {
        if baseline_metric != 0.0 {
            let ratio = b.metric / baseline_metric;
            if ratio >= 10.0 || ratio <= 0.1 {
                format!("{:.0}x", ratio)
            } else {
                let pct = (ratio - 1.0) * 100.0;
                format!("{:+.1}%", pct)
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Last attempt
    let last_attempt = history.last().map(|r| {
        let mut m = HashMap::new();
        m.insert("num".to_string(), r.num.to_string());
        m.insert("description".to_string(), r.description.clone());
        m.insert("status".to_string(), r.status.clone());
        m
    });

    let direction_str = match config.metric.direction {
        Direction::Maximize => "maximize",
        Direction::Minimize => "minimize",
    };

    let ctx = minijinja::context! {
        iteration => iteration,
        name => &config.name,
        metric_name => &config.metric.name,
        direction => direction_str,
        editable => editable_files,
        readonly => &config.readonly,
        history => history_rows,
        best_metric => best_metric,
        best_vs_baseline => best_vs_baseline,
        best_commit => best_commit,
        baseline_metric => format!("{:.2}", baseline_metric),
        has_constraints => !config.constraints.is_empty(),
        constraints => &config.constraints,
        context => &config.context,
        last_attempt => last_attempt,
    };

    tmpl.render(ctx).context("rendering iteration prompt")
}

/// Run the benchmark command and parse metrics.
fn run_benchmark(config: &Config) -> Result<(Vec<MetricResult>, Duration, bool)> {
    let start = Instant::now();
    let timeout = Duration::from_secs(config.timeout);

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&config.run)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start benchmark command")?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let reader = BufReader::new(stdout);

    let mut output_lines: Vec<String> = Vec::new();
    let mut metrics: Vec<MetricResult> = Vec::new();

    for line in reader.lines() {
        let line = line.context("reading benchmark output")?;

        if let Some(value) = try_parse_metric(&line, &config.metric.grep) {
            metrics.push(MetricResult {
                name: config.metric.name.clone(),
                value,
            });
        }

        for constraint in &config.constraints {
            if let Some(value) = try_parse_metric(&line, &constraint.grep) {
                metrics.push(MetricResult {
                    name: constraint.name.clone(),
                    value,
                });
            }
        }

        output_lines.push(line);

        if start.elapsed() > timeout {
            let _ = child.kill();
            eprintln!("  TIMEOUT after {}s — killing", config.timeout);
            return Ok((metrics, start.elapsed(), false));
        }
    }

    let status = child.wait().context("waiting for benchmark to finish")?;
    let elapsed = start.elapsed();

    if !status.success() {
        eprintln!("  CRASHED (exit code: {})", status.code().unwrap_or(-1));
        // Show last 10 lines for debugging
        for line in output_lines.iter().rev().take(10).rev() {
            eprintln!("  | {}", line);
        }
        return Ok((metrics, elapsed, false));
    }

    Ok((metrics, elapsed, true))
}

/// Check whether constraints are violated. Returns list of violations.
fn check_constraints(config: &Config, metrics: &[MetricResult]) -> Vec<String> {
    let mut violations = Vec::new();
    for constraint in &config.constraints {
        if let Some(m) = metrics.iter().find(|m| m.name == constraint.name) {
            if let Some(limit) = constraint.fail_above {
                if m.value > limit {
                    violations.push(format!(
                        "{} = {:.2} > {} (fail_above)",
                        m.name, m.value, limit
                    ));
                }
            }
            if let Some(limit) = constraint.fail_below {
                if m.value < limit {
                    violations.push(format!(
                        "{} = {:.2} < {} (fail_below)",
                        m.name, m.value, limit
                    ));
                }
            }
        }
    }
    violations
}

/// Git: get short hash of HEAD.
fn git_short_hash() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .context("git rev-parse")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Git: commit all changes to editable files.
fn git_commit(config: &Config, message: &str) -> Result<()> {
    let mut args = vec!["add"];
    let editable_refs: Vec<&str> = config.editable.iter().map(|s| s.as_str()).collect();
    args.extend(&editable_refs);

    Command::new("git")
        .args(&args)
        .output()
        .context("git add")?;

    Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .context("git commit")?;

    Ok(())
}

/// Git: check if editable files have changes.
fn git_has_changes(config: &Config) -> Result<bool> {
    let mut args = vec!["diff", "--name-only", "--"];
    let editable_refs: Vec<&str> = config.editable.iter().map(|s| s.as_str()).collect();
    args.extend(editable_refs);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("git diff")?;

    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

/// Git: revert editable files to HEAD.
fn git_revert_editable(config: &Config) -> Result<()> {
    let mut args = vec!["checkout", "HEAD", "--"];
    let editable_refs: Vec<&str> = config.editable.iter().map(|s| s.as_str()).collect();
    args.extend(editable_refs);

    Command::new("git")
        .args(&args)
        .output()
        .context("git checkout")?;

    Ok(())
}

/// Append a row to results.tsv.
fn append_result(
    config: &Config,
    tsv_path: &Path,
    commit: &str,
    metrics: &[MetricResult],
    status: &str,
    description: &str,
) -> Result<()> {
    use std::io::Write;

    // Create file with header if it doesn't exist
    let needs_header = !tsv_path.exists();

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(tsv_path)
        .with_context(|| format!("opening {} for append", tsv_path.display()))?;

    if needs_header {
        let cols = config.tsv_columns();
        writeln!(file, "{}", cols.join("\t"))?;
    }

    // Build row
    let primary_value = metrics
        .iter()
        .find(|m| m.name == config.metric.name)
        .map(|m| m.value)
        .unwrap_or(0.0);

    let mut row = vec![commit.to_string(), format!("{:.2}", primary_value)];

    for constraint in &config.constraints {
        let value = metrics
            .iter()
            .find(|m| m.name == constraint.name)
            .map(|m| m.value)
            .unwrap_or(0.0);
        row.push(format!("{:.2}", value));
    }

    row.push(status.to_string());
    row.push(description.to_string());

    writeln!(file, "{}", row.join("\t"))?;

    Ok(())
}

/// Spawn the agent to edit code. Returns the agent's description of what it did.
fn spawn_agent(agent_cmd: &str, prompt_path: &Path) -> Result<String> {
    // Replace {prompt} placeholder with actual path
    let cmd = agent_cmd.replace("{prompt}", &prompt_path.display().to_string());

    let output = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .context("failed to spawn agent")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("agent exited with error: {}", stderr.trim());
    }

    // Agent's stdout is its description of what it did
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Try to extract a short description from agent output (first non-empty line)
    let description = stdout
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("(no description)")
        .trim()
        .to_string();

    // Truncate to reasonable length
    if description.len() > 120 {
        Ok(format!("{}...", &description[..117]))
    } else {
        Ok(description)
    }
}

/// Initialize results.tsv with baseline if config has baseline values.
fn init_results_tsv(config: &Config, tsv_path: &Path) -> Result<()> {
    if tsv_path.exists() {
        return Ok(());
    }

    if let Some(baseline) = &config.baseline {
        let primary_value = *baseline.get(&config.metric.name).unwrap_or(&0.0);
        let mut baseline_metrics: Vec<MetricResult> = vec![MetricResult {
            name: config.metric.name.clone(),
            value: primary_value,
        }];
        for constraint in &config.constraints {
            let value = *baseline.get(&constraint.name).unwrap_or(&0.0);
            baseline_metrics.push(MetricResult {
                name: constraint.name.clone(),
                value,
            });
        }
        append_result(
            config,
            tsv_path,
            "baseline",
            &baseline_metrics,
            "keep",
            "baseline",
        )?;
    }

    Ok(())
}

/// Display a summary line for an iteration.
fn print_iteration_summary(
    iteration: usize,
    commit: &str,
    primary: f64,
    baseline: f64,
    status: &str,
    description: &str,
    elapsed: Duration,
) {
    let vs_baseline = if baseline != 0.0 {
        let ratio = primary / baseline;
        if ratio >= 10.0 || ratio <= 0.1 {
            format!("{:.0}x", ratio)
        } else {
            let pct = (ratio - 1.0) * 100.0;
            format!("{:+.1}%", pct)
        }
    } else {
        String::new()
    };

    let status_marker = match status {
        "keep" => "+",
        "discard" => "-",
        "crash" => "!",
        _ => "?",
    };

    println!(
        "  [{:>3}] {} {:<10} {:>12.2}  {:<8} {} {:<8} {} ({:.1}s)",
        iteration,
        status_marker,
        commit,
        primary,
        vs_baseline,
        status_marker,
        status,
        description,
        elapsed.as_secs_f64()
    );
}

/// The main loop.
pub fn run_loop(
    config: &Config,
    agent_cmd: &str,
    tsv_path: &Path,
    max_iterations: Option<usize>,
) -> Result<()> {
    println!();
    println!(
        "  ratchet loop — {} — {} ({})",
        config.name,
        config.metric.name,
        match config.metric.direction {
            Direction::Maximize => "maximize",
            Direction::Minimize => "minimize",
        }
    );
    println!("  agent: {}", agent_cmd);
    println!();

    // Initialize results.tsv with baseline if needed
    init_results_tsv(config, tsv_path)?;

    // If no baseline run exists yet, run the benchmark once to establish it
    let history = read_history(tsv_path, config.constraints.len())?;
    if history.is_empty() {
        println!("  establishing baseline...");
        let (metrics, elapsed, success) = run_benchmark(config)?;
        if !success || metrics.is_empty() {
            bail!("baseline run failed — fix the benchmark before looping");
        }

        let primary = metrics
            .iter()
            .find(|m| m.name == config.metric.name)
            .map(|m| m.value)
            .unwrap_or(0.0);

        append_result(config, tsv_path, "baseline", &metrics, "keep", "baseline")?;
        println!(
            "  baseline: {} = {:.2} ({:.1}s)",
            config.metric.name,
            primary,
            elapsed.as_secs_f64()
        );
        println!();
    }

    // Prompt file path (reused each iteration)
    let prompt_path = PathBuf::from(".ratchet-prompt.md");

    let mut iteration = 0;
    loop {
        iteration += 1;

        if let Some(max) = max_iterations {
            if iteration > max {
                println!();
                println!("  reached max iterations ({}), stopping", max);
                break;
            }
        }

        // 1. Read current history
        let history = read_history(tsv_path, config.constraints.len())?;
        let baseline_metric = history.first().map(|r| r.metric).unwrap_or(0.0);

        // 2. Build iteration prompt
        let prompt = build_iteration_prompt(config, iteration, &history)?;
        fs::write(&prompt_path, &prompt).context("writing iteration prompt")?;

        // 3. Spawn agent
        println!("  [{:>3}] spawning agent...", iteration);
        let iter_start = Instant::now();

        let description = match spawn_agent(agent_cmd, &prompt_path) {
            Ok(desc) => desc,
            Err(e) => {
                eprintln!("  [{:>3}] agent error: {}", iteration, e);
                continue;
            }
        };

        // 4. Check if agent made changes
        let has_changes = git_has_changes(config)?;
        if !has_changes {
            println!("  [{:>3}] agent made no changes, skipping", iteration);
            continue;
        }

        // 5. Commit the changes
        let commit_msg = format!("ratchet #{}: {}", iteration, &description);
        git_commit(config, &commit_msg)?;
        let commit_hash = git_short_hash()?;

        // 6. Run benchmark
        let (metrics, _bench_elapsed, success) = run_benchmark(config)?;
        let total_elapsed = iter_start.elapsed();

        if !success || metrics.is_empty() {
            // Crash — revert
            git_revert_editable(config)?;
            git_commit(config, &format!("revert ratchet #{}: crash", iteration))?;
            append_result(
                config,
                tsv_path,
                &commit_hash,
                &metrics,
                "crash",
                &description,
            )?;
            print_iteration_summary(
                iteration,
                &commit_hash,
                0.0,
                baseline_metric,
                "crash",
                &description,
                total_elapsed,
            );
            continue;
        }

        let primary = metrics
            .iter()
            .find(|m| m.name == config.metric.name)
            .map(|m| m.value)
            .unwrap_or(0.0);

        // 7. Check constraints
        let violations = check_constraints(config, &metrics);
        if !violations.is_empty() {
            // Constraint violation — revert
            git_revert_editable(config)?;
            let violation_desc = format!("{} (constraint: {})", description, violations[0]);
            git_commit(
                config,
                &format!("revert ratchet #{}: constraint violation", iteration),
            )?;
            append_result(
                config,
                tsv_path,
                &commit_hash,
                &metrics,
                "crash",
                &violation_desc,
            )?;
            print_iteration_summary(
                iteration,
                &commit_hash,
                primary,
                baseline_metric,
                "crash",
                &violation_desc,
                total_elapsed,
            );
            continue;
        }

        // 8. Compare with current best
        let kept_history: Vec<&HistoryRow> =
            history.iter().filter(|r| r.status == "keep").collect();
        let current_best = match config.metric.direction {
            Direction::Maximize => kept_history
                .iter()
                .map(|r| r.metric)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)),
            Direction::Minimize => kept_history
                .iter()
                .map(|r| r.metric)
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)),
        };

        let is_better = match current_best {
            Some(best) => match config.metric.direction {
                Direction::Maximize => primary > best,
                Direction::Minimize => primary < best,
            },
            None => true, // No prior results, anything is better
        };

        if is_better {
            // Keep!
            append_result(
                config,
                tsv_path,
                &commit_hash,
                &metrics,
                "keep",
                &description,
            )?;
            print_iteration_summary(
                iteration,
                &commit_hash,
                primary,
                baseline_metric,
                "keep",
                &description,
                total_elapsed,
            );
        } else {
            // Discard — revert
            git_revert_editable(config)?;
            git_commit(
                config,
                &format!("revert ratchet #{}: no improvement", iteration),
            )?;
            append_result(
                config,
                tsv_path,
                &commit_hash,
                &metrics,
                "discard",
                &description,
            )?;
            print_iteration_summary(
                iteration,
                &commit_hash,
                primary,
                baseline_metric,
                "discard",
                &description,
                total_elapsed,
            );
        }
    }

    // Clean up prompt file
    let _ = fs::remove_file(&prompt_path);

    println!();
    Ok(())
}
