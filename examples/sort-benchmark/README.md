# sort-benchmark

A toy e2e example for ratchet. An AI agent optimizes a sorting function for throughput while maintaining 100% correctness.

## What's here

| File | Role |
|---|---|
| `sort.py` | **Editable** -- starts as bubble sort (~85 arrays/sec). The agent improves this. |
| `benchmark.py` | **Immutable** -- times `my_sort()` on 200 random arrays of 500 integers, checks correctness, prints metrics. |
| `ratchet.yaml` | Config -- maximize `throughput`, `correctness` must stay at 100%. |

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

## Why this is a good test

- **Fast feedback** -- each run takes ~2 seconds.
- **Massive improvement gradient** -- bubble sort is O(n^2). Even `return sorted(arr)` jumps to ~8,000+ arrays/sec.
- **Hard constraint** -- the agent can't cheat; correctness must remain 100%.
- **Room for creativity** -- after the obvious wins, the agent can explore radix sort, numpy, ctypes, etc.
