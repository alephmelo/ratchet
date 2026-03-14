use crate::config::{Config, Direction};
use std::path::Path;

/// Generate the instruction text dynamically from the current ratchet feature set.
/// If a ratchet.yaml already exists, it is loaded and included as context.
pub fn print_instructions(config_path: &Path) {
    let existing_config = Config::from_file(config_path).ok();

    let mut out = String::new();

    // ── Header ──────────────────────────────────────────────────
    out.push_str("# Ratchet — Setup Instructions for AI Agent\n\n");
    out.push_str(
        "You are helping a developer set up **ratchet**, a tool that points an AI agent at code, \
         tells it what number to improve, and lets it run autonomously in a loop. \
         Your job is to explore this repository, understand what it does, identify meaningful \
         metrics to optimise, and produce a valid `ratchet.yaml` config file.\n\n",
    );

    // ── What ratchet does ───────────────────────────────────────
    out.push_str("## What ratchet does\n\n");
    out.push_str(
        "Ratchet runs an optimisation loop:\n\
         1. An AI agent edits source files (the \"editable\" files).\n\
         2. A benchmark command runs and prints metrics to stdout.\n\
         3. Ratchet parses the metrics, decides whether to keep or revert, and repeats.\n\n\
         It supports single-metric optimisation, multi-metric Pareto optimisation, \
         hard/soft constraints, automatic experiment branching, rollback-to-best on failure, \
         and strategy hints that adapt based on experiment history.\n\n",
    );

    // ── Two modes ───────────────────────────────────────────────
    out.push_str("## Two operating modes\n\n");
    out.push_str(
        "- `ratchet init` — generates a `program.md` prompt that you hand to an AI agent \
         (e.g. Claude Code). The agent runs the full loop autonomously.\n\
         - `ratchet loop` — ratchet itself orchestrates the loop, spawning the agent each \
         iteration with a per-iteration prompt. This mode auto-creates a `ratchet/{tag}` \
         experiment branch (e.g. `ratchet/mar10`, `ratchet/mar10-2`) when run from \
         main/master.\n\n",
    );

    // ── Config schema ───────────────────────────────────────────
    out.push_str("## ratchet.yaml — full config schema\n\n");
    out.push_str("```yaml\n");
    out.push_str(
        "# Required fields\n\
         name: \"my-project\"              # Human-readable experiment name\n\
         editable:                         # Files the agent is allowed to modify\n\
         \x20 - src/solver.py\n\
         readonly:                         # Files the agent can read but must NOT edit (optional)\n\
         \x20 - benchmark.py\n\
         run: \"python3 benchmark.py\"      # Shell command that runs the benchmark\n\n",
    );
    out.push_str(
        "# Metric — single metric mode (use ONE of `metric` or `metrics`, not both)\n\
         metric:\n\
         \x20 name: throughput               # Metric name (must match what the benchmark prints)\n\
         \x20 grep: \"^throughput:\"           # Prefix pattern to find the metric line in stdout\n\
         \x20 direction: maximize            # \"maximize\" or \"minimize\"\n\n",
    );
    out.push_str(
        "# Metrics — multi-metric Pareto mode (alternative to `metric`)\n\
         # When using multiple metrics, ratchet keeps results that are on the Pareto frontier:\n\
         # a result is kept if no previous result is better in ALL metrics simultaneously.\n\
         metrics:\n\
         \x20 - name: requests_per_sec\n\
         \x20   grep: \"^requests_per_sec:\"\n\
         \x20   direction: maximize\n\
         \x20 - name: latency_p99\n\
         \x20   grep: \"^latency_p99:\"\n\
         \x20   direction: minimize\n\n",
    );
    out.push_str(
        "# Constraints — guard rails that reject or warn about bad results (optional)\n\
         constraints:\n\
         \x20 - name: correctness\n\
         \x20   grep: \"^correctness:\"\n\
         \x20   fail_below: 100.0            # Hard limit: revert if violated\n\
         \x20   # fail_above: 50.0           # Hard upper limit\n\
         \x20   # warn_below: 95.0           # Soft limit: warning only\n\
         \x20   # warn_above: 1000.0         # Soft upper limit\n\n",
    );
    out.push_str(&format!(
        "# Optional fields\n\
         timeout: 600                       # Max seconds for the benchmark command (default: {})\n\
         context: |                         # Free-text domain hints given to the agent\n\
         \x20 Pure Python only, no numpy.\n\
         \x20 Focus on algorithmic improvements.\n\
         baseline:                          # Known baseline values (avoids wasting a run)\n\
         \x20 throughput: 85.0\n\
         agent: 'opencode run < {{prompt}}'  # Agent command ({{prompt}} = prompt file path)\n\
         agent_timeout: 1800               # Max seconds to wait for the agent (default: {})\n\
         max_iterations: 50                # Stop after N total iterations (optional)\n\
         patience: 10                      # Stop after N iterations without improvement (optional)\n\
         bandit: true                      # Enable multi-armed bandit strategy selection (optional)\n\
         # Or with custom exploration constant:\n\
         # bandit:\n\
         #   exploration: 1.41             # UCB1 exploration constant (default: sqrt(2))\n",
        600, 1800,
    ));
    out.push_str("```\n\n");

    // ── How metric parsing works ────────────────────────────────
    out.push_str("## How metric parsing works\n\n");
    out.push_str(
        "The benchmark command must print metrics to stdout in the format:\n\
         ```\n\
         metric_name: 1234.56\n\
         ```\n\
         The `grep` field in the config is a **prefix match** (not regex). \
         For example, `grep: \"^throughput:\"` matches any line starting with `throughput:` \
         and parses the first number after the prefix. The `^` anchor is optional and stripped.\n\n\
         The benchmark can print anything else to stdout — ratchet only looks for lines \
         matching the configured prefixes.\n\n",
    );

    // ── CLI commands ────────────────────────────────────────────
    out.push_str("## Available CLI commands\n\n");
    out.push_str(
        "| Command | Purpose |\n\
         |---|---|\n\
         | `ratchet check` | Validate ratchet.yaml without running anything |\n\
         | `ratchet run` | Run the benchmark once and display parsed metrics |\n\
         | `ratchet init` | Generate program.md for agent-driven mode |\n\
         | `ratchet loop` | Run the autonomous optimisation loop |\n\
         | `ratchet results` | Show experiment results from results.tsv |\n\
         | `ratchet plot` | Plot metric progression in the terminal |\n\
         | `ratchet diff` | Show diff of editable files (supports `--best`, `--commit`) |\n\
         | `ratchet instruct` | Print these setup instructions |\n\n",
    );

    // ── What the agent should do ────────────────────────────────
    out.push_str("## Your task\n\n");
    out.push_str(
        "1. **Explore this repository.** Understand the codebase: what it does, what language(s), \
         what the entry points are, what can be benchmarked.\n\n\
         2. **Identify metrics.** Look for things that can be measured numerically: \
         throughput, latency, accuracy, loss, memory usage, file size, compression ratio, \
         build time, test execution time, etc. The best metrics are ones where improvement \
         is meaningful and the benchmark is deterministic (or low-variance).\n\n\
         3. **Identify editable vs readonly files.** The agent should only edit files that \
         contain the implementation logic. Benchmark scripts, test harnesses, and data files \
         should be readonly.\n\n\
         4. **Write a benchmark script** (if one doesn't exist) that:\n\
         \x20  - Runs the workload\n\
         \x20  - Prints metrics as `metric_name: value` lines to stdout\n\
         \x20  - Exits 0 on success, non-zero on failure\n\
         \x20  - Runs in a reasonable time (under the timeout)\n\n\
         5. **Write the `ratchet.yaml`** file with the config schema shown above.\n\n\
         6. **Validate** by running `ratchet check` and `ratchet run` to confirm metrics \
         are parsed correctly.\n\n",
    );

    // ── Guidelines ──────────────────────────────────────────────
    out.push_str("## Guidelines for choosing good metrics\n\n");
    out.push_str(
        "- **Deterministic is better.** If the benchmark has high variance, the loop will \
         waste iterations on noise. Use fixed seeds, warm-up runs, or multiple iterations \
         with averaging.\n\
         - **Fast is better.** Each loop iteration runs the full benchmark. A 10-second \
         benchmark means 100 iterations take ~17 minutes. A 5-minute benchmark means \
         100 iterations take ~8 hours.\n\
         - **Correctness constraints are critical.** If the code can produce wrong results \
         that happen to be \"faster\", add a constraint (e.g. `fail_below: 100.0` on \
         correctness) to prevent that.\n\
         - **Use `minimize` for things like latency, loss, error rate, memory.** \
         Use `maximize` for things like throughput, accuracy, compression ratio.\n\
         - **Multi-metric Pareto** is useful when there's a natural tradeoff \
         (e.g. throughput vs latency, accuracy vs speed). Use `metrics:` (plural) for this.\n\n",
    );

    // ── Existing config context ─────────────────────────────────
    if let Some(cfg) = &existing_config {
        out.push_str("## Existing ratchet.yaml detected\n\n");
        out.push_str(&format!(
            "A `ratchet.yaml` already exists in this directory with the following config:\n\n\
             - **name:** {}\n\
             - **editable:** {}\n",
            cfg.name,
            cfg.editable.join(", "),
        ));
        if !cfg.readonly.is_empty() {
            out.push_str(&format!("- **readonly:** {}\n", cfg.readonly.join(", ")));
        }
        out.push_str(&format!("- **run:** `{}`\n", cfg.run));
        for m in cfg.primary_metrics() {
            let dir = match m.direction {
                Direction::Maximize => "maximize",
                Direction::Minimize => "minimize",
            };
            out.push_str(&format!(
                "- **metric:** {} (grep: `{}`, {})\n",
                m.name, m.grep, dir
            ));
        }
        if cfg.is_multi_metric() {
            out.push_str("- **mode:** multi-metric (Pareto)\n");
        }
        if !cfg.constraints.is_empty() {
            for c in &cfg.constraints {
                let mut bounds = Vec::new();
                if let Some(v) = c.fail_above {
                    bounds.push(format!("fail_above: {}", v));
                }
                if let Some(v) = c.fail_below {
                    bounds.push(format!("fail_below: {}", v));
                }
                if let Some(v) = c.warn_above {
                    bounds.push(format!("warn_above: {}", v));
                }
                if let Some(v) = c.warn_below {
                    bounds.push(format!("warn_below: {}", v));
                }
                out.push_str(&format!(
                    "- **constraint:** {} (grep: `{}`, {})\n",
                    c.name,
                    c.grep,
                    bounds.join(", ")
                ));
            }
        }
        out.push_str(&format!("- **timeout:** {}s\n", cfg.timeout));
        if let Some(ref ctx) = cfg.context {
            out.push_str(&format!("- **context:** {}\n", ctx.trim()));
        }
        if let Some(ref agent) = cfg.agent {
            out.push_str(&format!("- **agent:** `{}`\n", agent));
        }
        out.push_str(
            "\nYou can review and improve this config, or use it as-is. \
             Run `ratchet check` to validate and `ratchet run` to test it.\n\n",
        );
    } else {
        out.push_str("## No ratchet.yaml found\n\n");
        out.push_str(
            "There is no `ratchet.yaml` in the current directory. \
             Explore the codebase and create one following the schema above.\n\n",
        );
    }

    // ── Example configs ─────────────────────────────────────────
    out.push_str("## Example: single metric with constraint\n\n");
    out.push_str(
        "```yaml\n\
         name: sort-benchmark\n\
         editable:\n\
         \x20 - sort.py\n\
         readonly:\n\
         \x20 - benchmark.py\n\
         run: \"python3 benchmark.py\"\n\
         metric:\n\
         \x20 name: throughput\n\
         \x20 grep: \"^throughput:\"\n\
         \x20 direction: maximize\n\
         constraints:\n\
         \x20 - name: correctness\n\
         \x20   grep: \"^correctness:\"\n\
         \x20   fail_below: 100.0\n\
         timeout: 30\n\
         context: |\n\
         \x20 Pure Python sorting. Baseline is bubble sort.\n\
         baseline:\n\
         \x20 throughput: 85.0\n\
         ```\n\n",
    );

    out.push_str("## Example: multi-metric Pareto\n\n");
    out.push_str(
        "```yaml\n\
         name: http-handler\n\
         editable:\n\
         \x20 - handler.py\n\
         readonly:\n\
         \x20 - benchmark.py\n\
         run: \"python3 benchmark.py\"\n\
         metrics:\n\
         \x20 - name: requests_per_sec\n\
         \x20   grep: \"^requests_per_sec:\"\n\
         \x20   direction: maximize\n\
         \x20 - name: latency_p99\n\
         \x20   grep: \"^latency_p99:\"\n\
         \x20   direction: minimize\n\
         constraints:\n\
         \x20 - name: correctness\n\
         \x20   grep: \"^correctness:\"\n\
         \x20   fail_below: 100.0\n\
         timeout: 60\n\
         max_iterations: 20\n\
         patience: 5\n\
         ```\n\n",
    );

    print!("{}", out);
}
