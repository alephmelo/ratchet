# ratchet

Point an AI agent at your code, tell it what number to improve, and let it run forever.

Ratchet generates a `program.md` from a simple YAML config. You hand that file to any AI coding agent (Claude Code, Codex, OpenCode, etc.) and it runs an autonomous loop: edit code, run experiment, measure metric, keep if better, revert if worse, repeat.

Inspired by Karpathy's [autoresearch](https://github.com/karpathy/autoresearch).

## How it works

You need three things:

1. **Code to optimize** -- one or more files the agent can edit
2. **A benchmark** -- a command that runs and prints a number
3. **A direction** -- maximize or minimize that number

Ratchet does the rest. It generates detailed instructions that tell the agent how to run the loop, track results in a TSV, use git to commit improvements and revert failures, and never stop until you interrupt it.

## Apply it to your project

Any project where you can measure a single number works. Write a `ratchet.yaml`:

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
  f37d0c1        49704.88  585x     + keep     int16 -- values fit in 16 bits
  ----------------------------------------------------------------------------

  experiments: 4  (kept: 3, discarded: 1, crashed: 0)
  best:        throughput = 49704.88  (585x vs baseline)  [f37d0c1]
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

All commands accept `--config <path>` (default: `ratchet.yaml`).

## Config reference

| Field | Required | Description |
|---|---|---|
| `name` | yes | Experiment name |
| `editable` | yes | Files the agent can modify |
| `readonly` | no | Files the agent must not touch (shown for context) |
| `run` | yes | Command to run the experiment |
| `metric.name` | yes | Primary metric name |
| `metric.grep` | yes | Grep pattern to extract the metric from stdout |
| `metric.direction` | yes | `maximize` or `minimize` |
| `constraints` | no | Secondary metrics with thresholds |
| `timeout` | no | Max seconds per run (default: 600) |
| `baseline` | no | Known baseline values (avoids re-running) |
| `context` | no | Free-text domain hints for the agent |

## Examples

- **[sort-benchmark](examples/sort-benchmark/)** -- bubble sort to numpy in 7 experiments (85 -> 49,704 arrays/sec, 585x). A good e2e test.
- **[autoresearch](examples/autoresearch.yaml)** -- Karpathy's GPT pretraining optimization as a ratchet config.
- **[api-optimizer](examples/api-optimizer.yaml)** -- maximize API throughput with p99 latency and error rate constraints.

## License

MIT
