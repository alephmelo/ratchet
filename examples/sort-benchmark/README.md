# sort-benchmark

A toy e2e example for ratchet. An AI agent optimizes a sorting function for throughput while maintaining 100% correctness.

## What's here

| File | Role |
|---|---|
| `sort.py` | **Editable** -- starts as bubble sort (~85 arrays/sec). The agent improves this. |
| `benchmark.py` | **Immutable** -- times `my_sort()` on 200 random arrays of 500 integers, checks correctness, prints metrics. |
| `ratchet.yaml` | Config -- maximize `throughput`, `correctness` must stay at 100%. |
| `results.tsv` | Experiment log from a sample run. |

## Running

Generate the agent instructions:

```bash
ratchet init
```

This creates `program.md`. Hand it to an AI coding agent (Claude Code, Codex, etc.) pointed at this directory.

You can also run the benchmark directly to see the baseline:

```bash
python3 benchmark.py
```

```
throughput: 85.09
correctness: 100.0
elapsed_sec: 2.3505
num_arrays: 200
array_size: 500
```

## Sample results

After running the optimization loop, view the scoreboard with `ratchet results`:

```
  sort-benchmark — throughput ^ (higher is better)

  commit       throughput  vs base    status   description
  ----------------------------------------------------------------------------
  baseline          85.00           + keep     bubble sort baseline
  af694fe        33089.98  389x     + keep     use built-in sorted()
  8214d37        33854.79  398x     + keep     in-place list.sort() avoids allocation
  22f00d2         6153.36  72x      - discard  counting sort -- pure Python loops too slow vs C timsort
  f6b82ca        35599.33  419x     + keep     numpy introsort on contiguous int64 array
  9b9552e        45964.69  541x     + keep     int32 instead of int64 for better cache utilization
  f37d0c1        49704.88  585x     + keep     int16 -- values fit in 16 bits
  8aaad3b        44018.12  518x     - discard  np.sort stable returns new array -- extra allocation hurts
  a54fe56        71396.70  840x     + keep     C extension counting sort with numpy fallback
  ----------------------------------------------------------------------------

  experiments: 9  (kept: 7, discarded: 2, crashed: 0)
  best:        throughput = 71396.70  (840x vs baseline)  [a54fe56]
  baseline:    throughput = 85.00
```

The agent went from bubble sort (85 arrays/sec) to a C extension counting sort (71,396 arrays/sec) -- an **840x improvement** in 8 experiments.

## Why this is a good test

- **Fast feedback** -- each run takes ~2 seconds.
- **Massive improvement gradient** -- bubble sort is O(n^2). Even `return sorted(arr)` jumps to ~8,000+ arrays/sec.
- **Hard constraint** -- the agent can't cheat; correctness must remain 100%.
- **Room for creativity** -- after the obvious wins, the agent can explore radix sort, numpy, C extensions, etc.
