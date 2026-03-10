use anyhow::{bail, Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::{format_metric, Config, Direction};

/// A parsed row from results.tsv for building iteration prompts.
struct HistoryRow {
    num: usize,
    commit: String,
    /// Primary metric values, in the same order as config.primary_metrics().
    metric_values: Vec<f64>,
    status: String,
    description: String,
}

impl HistoryRow {
    /// Get the first (or only) primary metric value — backward compat.
    fn first_metric(&self) -> f64 {
        self.metric_values.first().copied().unwrap_or(0.0)
    }
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

/// Check if `candidate` Pareto-dominates `reference` given metric directions.
/// Returns true if candidate is at least as good in ALL metrics and strictly better in at least one.
fn pareto_dominates(candidate: &[f64], reference: &[f64], directions: &[Direction]) -> bool {
    if candidate.len() != reference.len() || candidate.len() != directions.len() {
        return false;
    }
    let mut strictly_better_in_any = false;
    for i in 0..candidate.len() {
        let better = match directions[i] {
            Direction::Maximize => candidate[i] > reference[i],
            Direction::Minimize => candidate[i] < reference[i],
        };
        let worse = match directions[i] {
            Direction::Maximize => candidate[i] < reference[i],
            Direction::Minimize => candidate[i] > reference[i],
        };
        if worse {
            return false;
        }
        if better {
            strictly_better_in_any = true;
        }
    }
    strictly_better_in_any
}

/// Check if `candidate` is better than `reference` (single-metric comparison).
fn is_single_metric_better(candidate: f64, reference: f64, direction: Direction) -> bool {
    match direction {
        Direction::Maximize => candidate > reference,
        Direction::Minimize => candidate < reference,
    }
}

/// Read results.tsv and return parsed history rows.
fn read_history(
    tsv_path: &Path,
    num_metrics: usize,
    num_constraints: usize,
) -> Result<Vec<HistoryRow>> {
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
    let min_cols = 1 + num_metrics + num_constraints + 2; // commit + metrics + constraints + status + description
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < min_cols {
            continue;
        }

        let mut metric_values = Vec::with_capacity(num_metrics);
        for j in 0..num_metrics {
            metric_values.push(cols[1 + j].trim().parse().unwrap_or(0.0));
        }

        let status_idx = 1 + num_metrics + num_constraints;
        let desc_idx = status_idx + 1;

        rows.push(HistoryRow {
            num: i + 1,
            commit: cols[0].trim().to_string(),
            metric_values,
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
    strategy_hints: &[String],
) -> Result<String> {
    let template_src = include_str!("../templates/iterate.md.j2");

    let env = minijinja::Environment::new();
    let tmpl = env
        .template_from_str(template_src)
        .context("parsing iterate.md.j2 template")?;

    // Read editable file contents
    let mut editable_files: Vec<HashMap<String, String>> = Vec::new();
    for path in &config.editable {
        let content =
            fs::read_to_string(path).unwrap_or_else(|_| format!("(file not found: {})", path));
        let mut m = HashMap::new();
        m.insert("path".to_string(), path.clone());
        m.insert("content".to_string(), content);
        editable_files.push(m);
    }

    // Build history rows for template
    let primary = config.primary_metrics();
    let baseline_metric = if let Some(first) = history.first() {
        first.first_metric()
    } else if let Some(baseline) = &config.baseline {
        *baseline.get(&config.first_metric().name).unwrap_or(&0.0)
    } else {
        0.0
    };

    let mut history_rows: Vec<HashMap<String, String>> = Vec::new();
    for row in history {
        let first_val = row.first_metric();
        let vs_baseline = if baseline_metric != 0.0 && row.commit != "baseline" {
            let ratio = first_val / baseline_metric;
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
        m.insert("num".to_string(), row.num.to_string());
        m.insert("metric".to_string(), format_metric(first_val));
        // For multi-metric, include all values
        for (i, val) in row.metric_values.iter().enumerate() {
            if i < primary.len() {
                m.insert(format!("metric_{}", primary[i].name), format_metric(*val));
            }
        }
        m.insert("vs_baseline".to_string(), vs_baseline);
        m.insert("status".to_string(), row.status.clone());
        m.insert("description".to_string(), row.description.clone());
        history_rows.push(m);
    }

    // Find best (based on first metric for display)
    let kept: Vec<&HistoryRow> = history.iter().filter(|r| r.status == "keep").collect();
    let first_dir = config.first_metric().direction;
    let best = match first_dir {
        Direction::Maximize => kept.iter().max_by(|a, b| {
            a.first_metric()
                .partial_cmp(&b.first_metric())
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        Direction::Minimize => kept.iter().min_by(|a, b| {
            a.first_metric()
                .partial_cmp(&b.first_metric())
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    };

    let best_metric = best
        .map(|b| format_metric(b.first_metric()))
        .unwrap_or("n/a".to_string());
    let best_commit = best.map(|b| b.commit.clone()).unwrap_or_default();
    let best_vs_baseline = if let Some(b) = best {
        if baseline_metric != 0.0 {
            let ratio = b.first_metric() / baseline_metric;
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

    let first_metric = config.first_metric();
    let direction_str = match first_metric.direction {
        Direction::Maximize => "maximize",
        Direction::Minimize => "minimize",
    };

    // Build metric names/directions for multi-metric display in template
    let metric_names: Vec<&str> = primary.iter().map(|m| m.name.as_str()).collect();
    let metric_directions: Vec<&str> = primary
        .iter()
        .map(|m| match m.direction {
            Direction::Maximize => "maximize",
            Direction::Minimize => "minimize",
        })
        .collect();

    let ctx = minijinja::context! {
        iteration => iteration,
        name => &config.name,
        metric_name => &first_metric.name,
        direction => direction_str,
        is_multi_metric => config.is_multi_metric(),
        metric_names => metric_names,
        metric_directions => metric_directions,
        editable => editable_files,
        readonly => &config.readonly,
        history => history_rows,
        best_metric => best_metric,
        best_vs_baseline => best_vs_baseline,
        best_commit => best_commit,
        baseline_metric => format_metric(baseline_metric),
        has_constraints => !config.constraints.is_empty(),
        constraints => &config.constraints,
        context => &config.context,
        last_attempt => last_attempt,
        strategy_hints => strategy_hints,
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
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to start benchmark command")?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let reader = BufReader::new(stdout);

    let mut output_lines: Vec<String> = Vec::new();
    let mut metrics: Vec<MetricResult> = Vec::new();

    for line in reader.lines() {
        let line = line.context("reading benchmark output")?;

        // Parse all primary metrics
        for pm in config.primary_metrics() {
            if let Some(value) = try_parse_metric(&line, &pm.grep) {
                metrics.push(MetricResult {
                    name: pm.name.clone(),
                    value,
                });
            }
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

    // Warn about any expected metrics that were not found in the output
    for pm in config.primary_metrics() {
        if !metrics.iter().any(|m| m.name == pm.name) {
            eprintln!(
                "  WARNING: metric '{}' not found in benchmark output (grep: '{}')",
                pm.name, pm.grep
            );
        }
    }
    for constraint in &config.constraints {
        if !metrics.iter().any(|m| m.name == constraint.name) {
            eprintln!(
                "  WARNING: constraint '{}' not found in benchmark output (grep: '{}')",
                constraint.name, constraint.grep
            );
        }
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
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git rev-parse failed: {}", stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Git: commit all changes to editable files.
fn git_commit(config: &Config, message: &str) -> Result<()> {
    let mut args = vec!["add"];
    let editable_refs: Vec<&str> = config.editable.iter().map(|s| s.as_str()).collect();
    args.extend(&editable_refs);

    let add_output = Command::new("git")
        .args(&args)
        .output()
        .context("git add")?;
    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        bail!("git add failed: {}", stderr.trim());
    }

    let commit_output = Command::new("git")
        .args(["commit", "-m", message])
        .output()
        .context("git commit")?;
    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        bail!("git commit failed: {}", stderr.trim());
    }

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

/// Git: revert editable files to a specific commit (or HEAD if None).
fn git_revert_editable(config: &Config, target: Option<&str>) -> Result<()> {
    let tgt = target.unwrap_or("HEAD");
    let mut args = vec!["checkout", tgt, "--"];
    let editable_refs: Vec<&str> = config.editable.iter().map(|s| s.as_str()).collect();
    args.extend(editable_refs);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("git checkout")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git checkout {} failed: {}", tgt, stderr.trim());
    }

    Ok(())
}

/// Git: get the current branch name (returns None if detached HEAD).
fn git_current_branch() -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()
        .context("git symbolic-ref")?;
    if !output.status.success() {
        // detached HEAD — not on any branch
        return Ok(None);
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        Ok(None)
    } else {
        Ok(Some(branch))
    }
}

/// Git: check whether a branch name already exists (local).
fn git_branch_exists(name: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/heads/{}", name)])
        .output()
        .context("git rev-parse --verify")?;
    Ok(output.status.success())
}

/// Git: create and switch to a new branch.
fn git_create_branch(name: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["checkout", "-b", name])
        .output()
        .context("git checkout -b")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git checkout -b {} failed: {}", name, stderr.trim());
    }
    Ok(())
}

/// Generate a date-based experiment tag like "mar10", "mar10-2", "mar10-3", etc.
/// Finds the first unused tag by checking existing branch names.
fn generate_experiment_tag() -> Result<String> {
    let now = Local::now();
    let base_tag = now.format("%b%-d").to_string().to_lowercase(); // e.g. "mar10"

    let candidate = format!("ratchet/{}", base_tag);
    if !git_branch_exists(&candidate)? {
        return Ok(base_tag);
    }

    // Try suffixes -2, -3, ... until we find one that doesn't exist
    for n in 2.. {
        let tag = format!("{}-{}", base_tag, n);
        let candidate = format!("ratchet/{}", tag);
        if !git_branch_exists(&candidate)? {
            return Ok(tag);
        }
    }

    unreachable!()
}

/// Ensure we are on an experiment branch before starting the loop.
///
/// - If already on a `ratchet/*` branch, proceed.
/// - If on `main` or `master`, auto-create a new `ratchet/{tag}` branch.
/// - Otherwise (any other non-ratchet branch), refuse to run.
fn ensure_experiment_branch() -> Result<String> {
    let branch = git_current_branch()?;

    match branch {
        Some(ref name) if name.starts_with("ratchet/") => {
            // Already on an experiment branch — good to go
            Ok(name.clone())
        }
        Some(ref name) if name == "main" || name == "master" => {
            // On main/master — auto-create an experiment branch
            let tag = generate_experiment_tag()?;
            let branch_name = format!("ratchet/{}", tag);
            println!("  creating experiment branch: {}", branch_name);
            git_create_branch(&branch_name)?;
            Ok(branch_name)
        }
        Some(name) => {
            bail!(
                "refusing to run on branch '{}'. \
                 Switch to main/master (to auto-create an experiment branch) \
                 or to an existing ratchet/* branch.",
                name
            );
        }
        None => {
            bail!(
                "HEAD is detached. \
                 Switch to main/master (to auto-create an experiment branch) \
                 or to an existing ratchet/* branch."
            );
        }
    }
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
    let mut row = vec![commit.to_string()];

    for pm in config.primary_metrics() {
        let value = metrics
            .iter()
            .find(|m| m.name == pm.name)
            .map(|m| m.value)
            .unwrap_or(0.0);
        row.push(format_metric(value));
    }

    for constraint in &config.constraints {
        let value = metrics
            .iter()
            .find(|m| m.name == constraint.name)
            .map(|m| m.value)
            .unwrap_or(0.0);
        row.push(format_metric(value));
    }

    row.push(status.to_string());
    row.push(description.to_string());

    writeln!(file, "{}", row.join("\t"))?;

    Ok(())
}

/// Spawn the agent to edit code. Returns the agent's description of what it did.
fn spawn_agent(agent_cmd: &str, prompt_path: &Path, agent_timeout: u64) -> Result<String> {
    // Replace {prompt} placeholder with actual path
    let cmd = agent_cmd.replace("{prompt}", &prompt_path.display().to_string());

    // Inherit stdout and stderr so the user can see agent output in real-time.
    // We capture the description from the last few lines of stdout instead.
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn agent")?;

    let timeout = Duration::from_secs(agent_timeout);
    let start = Instant::now();

    let status = loop {
        match child.try_wait().context("checking agent status")? {
            Some(status) => break status,
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // reap the process
                    bail!(
                        "agent timed out after {}s (agent_timeout: {}s)",
                        start.elapsed().as_secs(),
                        agent_timeout
                    );
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    };

    if !status.success() {
        bail!("agent exited with code {}", status.code().unwrap_or(-1));
    }

    // Since we inherited stdout, we can't capture the description from it.
    // Use git diff --stat to summarize what changed instead.
    let diff_output = Command::new("git")
        .args(["diff", "--stat", "HEAD"])
        .output()
        .context("git diff --stat")?;

    let diff_stat = String::from_utf8_lossy(&diff_output.stdout)
        .trim()
        .to_string();
    let description = diff_stat
        .lines()
        .last()
        .unwrap_or("agent edit")
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
        let mut baseline_metrics: Vec<MetricResult> = Vec::new();
        for pm in config.primary_metrics() {
            let value = *baseline.get(&pm.name).unwrap_or(&0.0);
            baseline_metrics.push(MetricResult {
                name: pm.name.clone(),
                value,
            });
        }
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
        "  [{:>3}] {} {:<10} {:>12}  {:<8} {} {:<8} {} ({:.1}s)",
        iteration,
        status_marker,
        commit,
        format_metric(primary),
        vs_baseline,
        status_marker,
        status,
        description,
        elapsed.as_secs_f64()
    );
}

/// Analyze experiment history and generate strategy hints for the agent.
fn build_strategy_hints(config: &Config, history: &[HistoryRow]) -> Vec<String> {
    let mut hints = Vec::new();

    if history.len() < 2 {
        return hints;
    }

    // Separate kept vs discarded/crashed
    let kept: Vec<&HistoryRow> = history.iter().filter(|r| r.status == "keep").collect();
    let failed: Vec<&HistoryRow> = history
        .iter()
        .filter(|r| r.status == "discard" || r.status == "crash")
        .collect();

    // Detect plateau: last N kept results within 5% of each other (first metric)
    if kept.len() >= 3 {
        let last_kept: Vec<f64> = kept
            .iter()
            .rev()
            .take(3)
            .map(|r| r.first_metric())
            .collect();
        let max_k = last_kept.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_k = last_kept.iter().cloned().fold(f64::INFINITY, f64::min);
        if max_k > 0.0 && (max_k - min_k) / max_k < 0.05 {
            hints.push(
                "The metric has plateaued — recent improvements are marginal. Consider a fundamentally different approach rather than incremental tuning.".to_string()
            );
        }
    }

    // Recent failure streak
    let recent: Vec<&HistoryRow> = history.iter().rev().take(5).collect();
    let recent_failures = recent.iter().filter(|r| r.status != "keep").count();
    if recent_failures >= 3 {
        hints.push(format!(
            "{} of the last {} attempts were discarded or crashed. The current approach may be hitting diminishing returns. Try a completely different strategy.",
            recent_failures,
            recent.len().min(5)
        ));
    }

    // What worked — find the biggest jumps (based on first metric)
    if kept.len() >= 2 {
        let mut biggest_jump_desc = String::new();
        let mut biggest_jump = 0.0_f64;
        let first_dir = config.first_metric().direction;
        for i in 1..kept.len() {
            let jump = match first_dir {
                Direction::Maximize => kept[i].first_metric() - kept[i - 1].first_metric(),
                Direction::Minimize => kept[i - 1].first_metric() - kept[i].first_metric(),
            };
            if jump > biggest_jump {
                biggest_jump = jump;
                biggest_jump_desc = kept[i].description.clone();
            }
        }
        if !biggest_jump_desc.is_empty() && biggest_jump > 0.0 {
            hints.push(format!(
                "Biggest improvement so far: \"{}\". Consider variations on that theme.",
                biggest_jump_desc
            ));
        }
    }

    // What failed — list recent crash/discard descriptions to avoid
    if !failed.is_empty() {
        let recent_failed: Vec<&str> = failed
            .iter()
            .rev()
            .take(3)
            .map(|r| r.description.as_str())
            .collect();
        hints.push(format!(
            "Recently failed approaches (avoid repeating): {}",
            recent_failed
                .iter()
                .map(|d| format!("\"{}\"", d))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Suggest escalation if many iterations done
    let total_non_baseline = history.iter().filter(|r| r.commit != "baseline").count();
    if total_non_baseline >= 10 && failed.len() > kept.len() {
        hints.push(
            "Most attempts have failed. Consider more radical changes: different algorithms, data structures, or even rewriting the core logic from scratch.".to_string()
        );
    }

    hints
}

/// The main loop.
pub fn run_loop(
    config: &Config,
    agent_cmd: &str,
    tsv_path: &Path,
    max_iterations: Option<usize>,
    patience: Option<usize>,
) -> Result<()> {
    // Ensure we're on an experiment branch (auto-creates one from main/master)
    let branch = ensure_experiment_branch()?;

    println!();
    let primary = config.primary_metrics();
    if config.is_multi_metric() {
        let metric_strs: Vec<String> = primary
            .iter()
            .map(|m| {
                format!(
                    "{} ({})",
                    m.name,
                    match m.direction {
                        Direction::Maximize => "maximize",
                        Direction::Minimize => "minimize",
                    }
                )
            })
            .collect();
        println!(
            "  ratchet loop — {} — Pareto: {}",
            config.name,
            metric_strs.join(", ")
        );
    } else {
        let first = config.first_metric();
        println!(
            "  ratchet loop — {} — {} ({})",
            config.name,
            first.name,
            match first.direction {
                Direction::Maximize => "maximize",
                Direction::Minimize => "minimize",
            }
        );
    }
    println!("  branch: {}", branch);
    println!("  agent: {}", agent_cmd);
    println!("  agent timeout: {}s", config.agent_timeout);
    if let Some(max) = max_iterations {
        println!("  max iterations: {}", max);
    }
    if let Some(p) = patience {
        println!(
            "  patience: {} (stop after {} iterations without improvement)",
            p, p
        );
    }
    println!();

    // Initialize results.tsv with baseline if needed
    init_results_tsv(config, tsv_path)?;

    // If no baseline run exists yet, run the benchmark once to establish it
    let num_metrics = config.primary_metrics().len();
    let history = read_history(tsv_path, num_metrics, config.constraints.len())?;
    if history.is_empty() {
        println!("  establishing baseline...");
        let (metrics, elapsed, success) = run_benchmark(config)?;
        if !success || metrics.is_empty() {
            bail!("baseline run failed — fix the benchmark before looping");
        }

        let first_metric_name = &config.first_metric().name;
        let first_val = metrics
            .iter()
            .find(|m| m.name == *first_metric_name)
            .map(|m| m.value)
            .unwrap_or(0.0);

        append_result(config, tsv_path, "baseline", &metrics, "keep", "baseline")?;
        println!(
            "  baseline: {} = {} ({:.1}s)",
            first_metric_name,
            format_metric(first_val),
            elapsed.as_secs_f64()
        );
        println!();
    }

    // Prompt file path (reused each iteration)
    let prompt_path = PathBuf::from(".ratchet-prompt.md");

    // Set up graceful shutdown on Ctrl-C
    let shutdown = Arc::new(AtomicBool::new(false));
    {
        let shutdown = shutdown.clone();
        ctrlc::set_handler(move || {
            eprintln!("\n  caught interrupt, finishing current operation and shutting down...");
            shutdown.store(true, Ordering::SeqCst);
        })
        .context("failed to set Ctrl-C handler")?;
    }

    let mut iteration = 0;
    let mut since_improvement = 0_usize;

    // Track the best commit hash for rollback-to-best
    let mut best_commit_hash: Option<String> = {
        let history = read_history(tsv_path, num_metrics, config.constraints.len())?;
        let kept: Vec<&HistoryRow> = history.iter().filter(|r| r.status == "keep").collect();
        let first_dir = config.first_metric().direction;
        let best = match first_dir {
            Direction::Maximize => kept.iter().max_by(|a, b| {
                a.first_metric()
                    .partial_cmp(&b.first_metric())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            Direction::Minimize => kept.iter().min_by(|a, b| {
                a.first_metric()
                    .partial_cmp(&b.first_metric())
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        };
        best.and_then(|b| {
            if b.commit == "baseline" {
                None
            } else {
                Some(b.commit.clone())
            }
        })
    };

    loop {
        iteration += 1;

        // Check for shutdown signal
        if shutdown.load(Ordering::SeqCst) {
            println!();
            println!("  interrupted, shutting down gracefully");
            // Revert any uncommitted changes to best known state
            if git_has_changes(config).unwrap_or(false) {
                let _ = git_revert_editable(config, best_commit_hash.as_deref());
            }
            break;
        }

        if let Some(max) = max_iterations {
            if iteration > max {
                println!();
                println!("  reached max iterations ({}), stopping", max);
                break;
            }
        }

        if let Some(p) = patience {
            if since_improvement >= p {
                println!();
                println!(
                    "  no improvement in {} iterations, stopping (patience exhausted)",
                    p
                );
                break;
            }
        }

        // 1. Read current history
        let history = read_history(tsv_path, num_metrics, config.constraints.len())?;
        let baseline_metric = history.first().map(|r| r.first_metric()).unwrap_or(0.0);

        // 2. Build strategy hints and iteration prompt
        let hints = build_strategy_hints(config, &history);
        let prompt = build_iteration_prompt(config, iteration, &history, &hints)?;
        fs::write(&prompt_path, &prompt).context("writing iteration prompt")?;

        // 3. Spawn agent
        println!("  [{:>3}] spawning agent...", iteration);
        let iter_start = Instant::now();

        let description = match spawn_agent(agent_cmd, &prompt_path, config.agent_timeout) {
            Ok(desc) => desc,
            Err(e) => {
                eprintln!("  [{:>3}] agent error: {}", iteration, e);
                since_improvement += 1;
                continue;
            }
        };

        // 4. Check if agent made changes
        let has_changes = git_has_changes(config)?;
        if !has_changes {
            println!("  [{:>3}] agent made no changes, skipping", iteration);
            since_improvement += 1;
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
            // Crash — revert to best known state
            git_revert_editable(config, best_commit_hash.as_deref())?;
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
            since_improvement += 1;
            continue;
        }

        let primary_val = metrics
            .iter()
            .find(|m| m.name == config.first_metric().name)
            .map(|m| m.value)
            .unwrap_or(0.0);

        // Build candidate metric values in the same order as config.primary_metrics()
        let candidate_values: Vec<f64> = config
            .primary_metrics()
            .iter()
            .map(|pm| {
                metrics
                    .iter()
                    .find(|m| m.name == pm.name)
                    .map(|m| m.value)
                    .unwrap_or(0.0)
            })
            .collect();

        // 7. Check constraints
        let violations = check_constraints(config, &metrics);
        if !violations.is_empty() {
            // Constraint violation — revert to best known state
            git_revert_editable(config, best_commit_hash.as_deref())?;
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
                primary_val,
                baseline_metric,
                "crash",
                &violation_desc,
                total_elapsed,
            );
            since_improvement += 1;
            continue;
        }

        // 8. Compare with current best using Pareto dominance (or single-metric)
        let kept_history: Vec<&HistoryRow> =
            history.iter().filter(|r| r.status == "keep").collect();

        let directions: Vec<Direction> = config
            .primary_metrics()
            .iter()
            .map(|m| m.direction)
            .collect();

        let is_better = if kept_history.is_empty() {
            true // No prior results, anything is better
        } else if config.is_multi_metric() {
            // Pareto: keep if candidate dominates at least one kept result
            // and is not dominated by any kept result
            let dominated_by_any = kept_history
                .iter()
                .any(|r| pareto_dominates(&r.metric_values, &candidate_values, &directions));
            // Keep if not dominated by any kept result (i.e., it's a new Pareto frontier member)
            !dominated_by_any
        } else {
            // Single metric: simple comparison
            let first_dir = config.first_metric().direction;
            let current_best = match first_dir {
                Direction::Maximize => kept_history
                    .iter()
                    .map(|r| r.first_metric())
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)),
                Direction::Minimize => kept_history
                    .iter()
                    .map(|r| r.first_metric())
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)),
            };
            match current_best {
                Some(best) => is_single_metric_better(primary_val, best, first_dir),
                None => true,
            }
        };

        if is_better {
            // Keep! Update best commit tracking.
            best_commit_hash = Some(commit_hash.clone());
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
                primary_val,
                baseline_metric,
                "keep",
                &description,
                total_elapsed,
            );
            since_improvement = 0;
        } else {
            // Discard — revert to best known state
            git_revert_editable(config, best_commit_hash.as_deref())?;
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
                primary_val,
                baseline_metric,
                "discard",
                &description,
                total_elapsed,
            );
            since_improvement += 1;
        }
    }

    // Clean up prompt file
    let _ = fs::remove_file(&prompt_path);

    println!();
    Ok(())
}
