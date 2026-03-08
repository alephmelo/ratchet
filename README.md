# ratchet

A CLI that generates autonomous optimization instructions for AI coding agents.

You write a `ratchet.yaml` describing what to optimize. Ratchet generates a `program.md` -- a complete set of instructions that any AI coding agent (Claude Code, Codex, etc.) can follow to run an autonomous edit-run-measure-ratchet loop: edit code, run the experiment, measure the metric, keep if better, revert if worse, repeat forever.

Ratchet is a **prompt generator**, not a runtime. It encodes the [autoresearch](https://github.com/karpathy/autoresearch) pattern into a reusable framework.

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# binary at target/release/ratchet
```

## Quickstart

### 1. Write a config

Create `ratchet.yaml` in your project:

```yaml
name: "sort-benchmark"

editable:
  - sort.py

readonly:
  - benchmark.py

run: "python3 benchmark.py"

metric:
  name: throughput
  grep: "^throughput:"
  direction: maximize

constraints:
  - name: correctness
    grep: "^correctness:"
    fail_below: 100.0

timeout: 30

baseline:
  throughput: 85.0
  correctness: 100.0

context: |
  This is a pure-Python sorting benchmark. You can only edit sort.py.
  The function my_sort(arr) must return a sorted list in ascending order.
  The starting implementation is bubble sort -- there is enormous room
  for improvement.
```

### 2. Validate

```bash
ratchet check
```

```
Config OK: sort-benchmark
  editable:    ["sort.py"]
  readonly:    ["benchmark.py"]
  run:         python3 benchmark.py
  metric:      throughput (maximize)
  constraints: 1
  timeout:     30s
```

### 3. Generate

```bash
ratchet init
```

This creates `program.md` -- hand it to your AI coding agent.

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
| `constraints[].name` | yes | Constraint metric name |
| `constraints[].grep` | yes | Grep pattern for this constraint |
| `constraints[].warn_above` | no | Soft upper limit (warn) |
| `constraints[].warn_below` | no | Soft lower limit (warn) |
| `constraints[].fail_above` | no | Hard upper limit (revert on violation) |
| `constraints[].fail_below` | no | Hard lower limit (revert on violation) |
| `timeout` | no | Max seconds per run (default: 600) |
| `baseline` | no | Known baseline values (avoids re-running) |
| `context` | no | Free-text domain hints for the agent |

## Commands

```
ratchet init [--config ratchet.yaml] [--output program.md]
```

Validate config and generate `program.md`.

```
ratchet check [--config ratchet.yaml]
```

Validate config without generating anything.

```
ratchet results [--config ratchet.yaml] [--results results.tsv]
```

Display experiment scoreboard from `results.tsv`. Shows each experiment's metric value, improvement vs baseline, keep/discard/crash status, and a summary with the best result.

```
  sort-benchmark — throughput ^ (higher is better)

  commit       throughput  vs base    status   description
  ----------------------------------------------------------------------------
  baseline          85.00           + keep     bubble sort baseline
  af694fe        33089.98  389x     + keep     use built-in sorted()
  8214d37        33854.79  398x     + keep     in-place list.sort() avoids allocation
  22f00d2         6153.36  72x      - discard  counting sort -- pure Python loops too slow
  f37d0c1        49704.88  585x     + keep     int16 -- values fit in 16 bits
  ----------------------------------------------------------------------------

  experiments: 5  (kept: 4, discarded: 1, crashed: 0)
  best:        throughput = 49704.88  (585x vs baseline)  [f37d0c1]
  baseline:    throughput = 85.00
```

## The pattern

Ratchet encodes a universal optimization loop with four components:

1. **Immutable evaluation harness** -- the `run` command and `readonly` files. Never modified.
2. **Mutable code** -- the `editable` files. The agent's search space.
3. **Scalar metric + direction** -- a single number to optimize, extracted via grep.
4. **Git-based ratchet** -- commit improvements, revert regressions. The metric only moves in one direction.

The generated `program.md` instructs the agent to:
- Create a branch, read the code, record the baseline
- Loop forever: edit code, commit, run, measure, keep or revert
- Log every attempt to `results.tsv`
- Never stop until interrupted

## Examples

### sort-benchmark

A self-contained e2e test. An agent optimizes a bubble sort for throughput. Includes a sample run with results (85 -> 49,704 arrays/sec, 585x improvement). See [`examples/sort-benchmark/`](examples/sort-benchmark/).

```bash
cd examples/sort-benchmark
ratchet init       # generate program.md for your agent
ratchet results    # view the experiment scoreboard
```

### autoresearch

Recreates the original [autoresearch](https://github.com/karpathy/autoresearch) experiment as a ratchet config. See [`examples/autoresearch.yaml`](examples/autoresearch.yaml).

### api-optimizer

Maximize API throughput (requests/sec) with p99 latency and error rate constraints. See [`examples/api-optimizer.yaml`](examples/api-optimizer.yaml).

## License

MIT
