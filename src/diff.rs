use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::config::{Config, Direction};

/// Find the best commit from results.tsv.
fn find_best_commit(config: &Config, tsv_path: &Path) -> Result<String> {
    let contents = std::fs::read_to_string(tsv_path)
        .with_context(|| format!("reading {}", tsv_path.display()))?;

    let mut lines = contents.lines();
    let _header = lines.next().context("results.tsv is empty")?;

    let num_metrics = config.primary_metrics().len();
    let num_constraints = config.constraints.len();
    let mut best_commit: Option<String> = None;
    let mut best_value: Option<f64> = None;

    let first_metric = config.first_metric();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        let min_cols = 1 + num_metrics + num_constraints + 2;
        if cols.len() < min_cols {
            continue;
        }

        let status_idx = 1 + num_metrics + num_constraints;
        let status = cols[status_idx].trim();
        if status != "keep" {
            continue;
        }

        let commit = cols[0].trim();
        if commit == "baseline" {
            continue;
        }

        let value: f64 = match cols[1].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let is_better = match best_value {
            None => true,
            Some(current_best) => match first_metric.direction {
                Direction::Maximize => value > current_best,
                Direction::Minimize => value < current_best,
            },
        };

        if is_better {
            best_value = Some(value);
            best_commit = Some(commit.to_string());
        }
    }

    match best_commit {
        Some(c) => Ok(c),
        None => bail!("no kept experiments found in results.tsv (only baseline)"),
    }
}

/// Run git diff for the editable files.
fn run_git_diff(config: &Config, from: &str, to: &str) -> Result<()> {
    let mut args = vec!["diff", "--color=always", from, to, "--"];
    let editable_refs: Vec<&str> = config.editable.iter().map(|s| s.as_str()).collect();
    args.extend(editable_refs);

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("failed to run git diff")?;

    let diff = String::from_utf8_lossy(&output.stdout);
    if diff.trim().is_empty() {
        println!("  no changes in editable files between {} and {}", from, to);
    } else {
        print!("{}", diff);
    }

    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Only show stderr if it's an actual error, not just color codes
        if !stderr.trim().is_empty() {
            eprintln!("{}", stderr);
        }
    }

    Ok(())
}

/// Get the merge-base or first commit on the branch.
fn find_base_commit() -> Result<String> {
    // Try to find merge-base with main/master
    for base_branch in &["main", "master"] {
        let output = Command::new("git")
            .args(["merge-base", base_branch, "HEAD"])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !hash.is_empty() {
                    return Ok(hash);
                }
            }
        }
    }

    // Fallback: use the root commit
    let output = Command::new("git")
        .args(["rev-list", "--max-parents=0", "HEAD"])
        .output()
        .context("failed to find root commit")?;

    let hash = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if hash.is_empty() {
        bail!("could not determine base commit");
    }

    Ok(hash)
}

/// Show diff of editable files.
pub fn show_diff(config: &Config, commit: Option<&str>, best: bool, tsv_path: &Path) -> Result<()> {
    if best {
        // Find best commit from results.tsv and diff it against its parent
        let best_commit = find_best_commit(config, tsv_path)?;
        println!("  diff at best result [{}]:", best_commit);
        println!();
        let parent = format!("{}~1", best_commit);
        run_git_diff(config, &parent, &best_commit)?;
    } else if let Some(commit) = commit {
        // Diff specific commit against its parent
        println!("  diff at [{}]:", commit);
        println!();
        let parent = format!("{}~1", commit);
        run_git_diff(config, &parent, commit)?;
    } else {
        // Diff HEAD vs base branch (full experiment diff)
        let base = find_base_commit()?;
        let short_base = &base[..7.min(base.len())];
        println!("  diff {}..HEAD (full experiment):", short_base);
        println!();
        run_git_diff(config, &base, "HEAD")?;
    }

    Ok(())
}
