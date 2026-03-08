# ratchet

Point an AI agent at your code, tell it what number to improve, and let it run forever.

Ratchet generates a `program.md` from a simple YAML config. You hand that file to any AI coding agent (Claude Code, Codex, OpenCode, etc.) and it runs an autonomous loop: edit code, run experiment, measure metric, keep if better, revert if worse, repeat.

Or use `ratchet loop` to let ratchet orchestrate everything — it spawns the agent, runs the benchmark, evaluates results, and handles git automatically.

Inspired by Karpathy's [autoresearch](https://github.com/karpathy/autoresearch).

## How it works

You need three things:

1. **Code to optimize** -- one or more files the agent can edit
2. **A benchmark** -- a command that runs and prints a number
3. **A direction** -- maximize or minimize that number

Ratchet does the rest. It generates detailed instructions that tell the agent how to run the loop, track results in a TSV, use git to commit improvements and revert failures, and never stop until you interrupt it.

## Apply it to your project

Any project where you can measure a number works. Write a `ratchet.yaml`:

```yaml
name: "my-project"

editable:
  - src/model.py          # files the agent can change

readonly:
  - benchmark.py          # files it should read but not touch

run: "python benchmark.py"

metric:
  name: accuracy
  grep: "^accuracy:"       # how to extract the number from stdout
  direction: maximize       # or minimize

timeout: 300
```

Then:

```bash
ratchet init               # generates program.md
# hand program.md to your AI agent
```

Or let ratchet run the whole loop:

```bash
ratchet loop --agent "opencode run -m github-copilot/claude-sonnet-4.6 < {prompt}"
```

That's it. The agent takes over from there.

### Ideas for what to optimize

| Project | Metric | Direction |
|---|---|---|
| ML training script | val_loss | minimize |
| API server | requests/sec | maximize |
| Compiler pass | binary size | minimize |
| Game AI | win rate | maximize |
| Image pipeline | processing time | minimize |
| Search engine | relevance score | maximize |
| Sorting algorithm | throughput | maximize |

### Multiple metrics (Pareto optimization)

If you have multiple metrics to optimize simultaneously, use `metrics` (plural) instead of `metric`. Ratchet uses Pareto dominance: a result is kept if no previous kept result is better in ALL metrics at once.

```yaml
name: "api-optimizer"

editable:
  - src/server.py

run: "python benchmark.py"

metrics:
  - name: throughput
    grep: "^throughput:"
    direction: maximize
  - name: p99_latency
    grep: "^p99_latency:"
    direction: minimize

timeout: 120
```

This lets the agent explore trade-offs — a result that improves throughput at the cost of slightly higher latency can still be kept if no existing result dominates it across both metrics. Single `metric:` configs continue to work as before.

### Adding constraints

If you need to keep a secondary metric in check (e.g. "make it faster but don't break correctness"), add constraints:

```yaml
constraints:
  - name: correctness
    grep: "^correctness:"
    fail_below: 100.0       # hard limit -- revert if violated

  - name: memory_mb
    grep: "^memory_mb:"
    warn_above: 512.0       # soft limit -- warn but don't revert
```

### Providing baselines

If you already know the current numbers, include them so the agent doesn't waste a run measuring them:

```yaml
baseline:
  accuracy: 0.847
  correctness: 100.0
```

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# binary at target/release/ratchet
```

## Commands

**`ratchet init`** -- generate `program.md` from `ratchet.yaml`

**`ratchet check`** -- validate config without generating

**`ratchet results`** -- display experiment scoreboard

```
  sort-benchmark — throughput ^ (higher is better)

  commit       throughput  vs base    status   description
  ----------------------------------------------------------------------------
  baseline          85.00           + keep     bubble sort baseline
  af694fe        33089.98  389x     + keep     use built-in sorted()
  22f00d2         6153.36  72x      - discard  counting sort -- pure Python loops too slow
  a54fe56        71396.70  840x     + keep     C extension counting sort with numpy fallback
  ----------------------------------------------------------------------------

  experiments: 4  (kept: 3, discarded: 1, crashed: 0)
  best:        throughput = 71396.70  (840x vs baseline)  [a54fe56]
  baseline:    throughput = 85.00
```

**`ratchet run`** -- execute the benchmark command, parse metrics, check constraints, and compare against baseline

```
  running: python3 benchmark.py

  results:

    throughput               47107.78 ^
    correctness                100.00

  elapsed: 0.19s

  vs baseline: 85.00 -> 47107.78 (554x) BETTER
```

**`ratchet diff`** -- show git diff of editable files

- `ratchet diff` -- full experiment diff (merge-base to HEAD)
- `ratchet diff --commit <hash>` -- diff a specific commit against its parent
- `ratchet diff --best` -- diff the best-scoring commit from `results.tsv`

```
  diff at best result [f37d0c1]:

  -    a = np.array(arr, dtype=np.int32)
  +    a = np.array(arr, dtype=np.int16)
```

**`ratchet loop`** -- run the autonomous optimization loop

Ratchet controls the iteration: spawn agent to edit code, run benchmark, evaluate, keep or revert, repeat. The agent only sees a focused per-iteration prompt with the current code, history, and what to try.

```bash
# Specify agent on the command line
ratchet loop --agent "opencode run -m github-copilot/claude-sonnet-4.6 < {prompt}"

# Or set it in ratchet.yaml
# agent: 'opencode run -m github-copilot/claude-sonnet-4.6 < {prompt}'
ratchet loop

# Limit iterations
ratchet loop -n 20

# Stop after 5 consecutive iterations without improvement
ratchet loop -p 5

# Combine both: at most 50 iterations, stop early if stuck
ratchet loop -n 50 -p 5
```

The `{prompt}` placeholder is replaced with the path to a generated prompt file. The agent should read it, edit the editable files, and exit. Ratchet handles everything else (git, benchmark, evaluation, logging).

The loop includes two automatic behaviors:

- **Rollback-to-best**: When a result is discarded or crashes, ratchet reverts editable files to the best-scoring commit (not just the previous one). This means the agent always works from the best known state.
- **Strategy hints**: After a few iterations, ratchet analyzes experiment history and injects hints into the agent's prompt — plateau detection ("recent improvements are marginal, try something different"), failure streak warnings, the biggest improvement so far ("more of this"), and recently failed approaches to avoid.

**`ratchet plot`** -- visualize metric progression

```
  sort-benchmark — throughput ^ (higher is better)

  +     base █      85.00
  +  af694fe ███████████████████████   33089.98
  +  8214d37 ████████████████████████   33854.79
  -  22f00d2 ████    6153.36
  +  f6b82ca █████████████████████████   35599.33
  +  9b9552e ████████████████████████████████   45964.69
  +  f37d0c1 ███████████████████████████████████   49704.88
  -  8aaad3b ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   44018.12
  +  a54fe56 ██████████████████████████████████████████████████   71396.70 *

  █ kept (7/9)  ░ discarded/crashed  * best: 71396.70 (840x)
```

All commands accept `--config <path>` (default: `ratchet.yaml`).

## Config reference

| Field | Required | Description |
|---|---|---|
| `name` | yes | Experiment name |
| `editable` | yes | Files the agent can modify |
| `readonly` | no | Files the agent must not touch (shown for context) |
| `run` | yes | Command to run the experiment |
| `metric` | yes* | Single primary metric (`name`, `grep`, `direction`) |
| `metrics` | yes* | Multiple primary metrics for Pareto optimization (list of `name`, `grep`, `direction`) |
| `constraints` | no | Secondary metrics with thresholds |
| `timeout` | no | Max seconds per run (default: 600) |
| `baseline` | no | Known baseline values (avoids re-running) |
| `context` | no | Free-text domain hints for the agent |
| `agent` | no | Agent command for `ratchet loop`. Use `{prompt}` as placeholder. |
| `max_iterations` | no | Maximum iterations for `ratchet loop` (overridden by `-n`). |
| `patience` | no | Stop after N iterations without improvement (overridden by `-p`). |

*Use either `metric` (single) or `metrics` (multiple), not both.

## Examples

- **[sort-benchmark](examples/sort-benchmark/)** -- bubble sort to C extension counting sort in 8 experiments (85 -> 71,396 arrays/sec, 840x). A good e2e test.
- **[knn-benchmark](examples/knn-benchmark/)** -- brute-force KNN to KD-tree with variance-reordered dims in 5 experiments (380 -> 6,698 queries/sec, 18x). Pure Python, zero dependencies.
- **[autoresearch](examples/autoresearch.yaml)** -- Karpathy's GPT pretraining optimization as a ratchet config.
- **[api-optimizer](examples/api-optimizer.yaml)** -- maximize API throughput with p99 latency and error rate constraints.

## License

MIT
