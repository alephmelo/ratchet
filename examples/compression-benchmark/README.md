# compression-benchmark

A lossless byte-stream compression benchmark for [ratchet](https://github.com/alephmelo/ratchet).

This example showcases the **multi-armed bandit** strategy selection feature
(`bandit: true`), which uses UCB1 to automatically explore and exploit
different optimization strategies across iterations.

## What it does

- **compress.py** — the editable compressor (baseline: naive RLE)
- **benchmark.py** — generates a ~2 MB mixed corpus and measures throughput (MB/s),
  compression ratio, and round-trip correctness

## Metrics

| Metric       | Direction | Constraint     |
|-------------|-----------|----------------|
| throughput  | maximize  | —              |
| ratio       | —         | fail_below 1.0 |
| correctness | —         | fail_below 100 |

## Run

```bash
# Single benchmark run
python3 benchmark.py

# Optimization loop with bandit strategy selection
ratchet loop
```

## Why bandit works well here

All six bandit arms map naturally to compression:

- **algorithm** — switch from RLE to LZ77, Huffman, hybrid schemes
- **data-structure** — hash tables for match finding, tries for dictionary coding
- **micro-optimization** — memoryview, struct packing, bytearray pre-allocation
- **parallelism** — chunked compression with multiprocessing
- **memory-layout** — buffer reuse, cache-friendly access patterns
- **rewrite** — complete reimplementation with a different approach
