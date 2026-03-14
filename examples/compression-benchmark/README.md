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

## Example results

10 iterations with `gpt-5.3-codex`, starting from a naive RLE baseline at 8.50 MB/s:

| # | throughput | vs baseline | strategy | status |
|---|-----------|-------------|----------|--------|
| baseline | 8.50 | — | — | keep |
| 1 | 249.12 | 29x | algorithm | keep |
| 2 | 245.00 | — | data-structure | discard |
| 3 | 241.85 | — | micro-optimization | discard |
| 4 | 250.65 | +0.6% | parallelism | keep |
| 5 | 0.00 | — | memory-layout | crash |
| 6 | 256.49 | +2.3% | rewrite | keep |
| 7 | 284.91 | +11% | algorithm | keep |
| 8 | 295.66 | +3.8% | parallelism | keep |
| 9 | 0.00 | — | rewrite | crash |
| 10 | 143.19 | — | algorithm | discard |

**Final: 295.66 MB/s** — a **34.8x** improvement over the baseline.

### Bandit arm performance

| Arm | Pulls | Wins | Win rate |
|-----|-------|------|----------|
| algorithm | 3 | 2 | 67% |
| parallelism | 2 | 2 | 100% |
| rewrite | 2 | 1 | 50% |
| data-structure | 1 | 0 | 0% |
| micro-optimization | 1 | 0 | 0% |
| memory-layout | 1 | 0 | 0% |

The bandit explored all 6 arms in the first 6 iterations (UCB1 prioritizes
untried arms), then exploited the top performers: `algorithm` and `parallelism`
produced the biggest gains.

## Why bandit works well here

All six bandit arms map naturally to compression:

- **algorithm** — switch from RLE to LZ77, Huffman, hybrid schemes
- **data-structure** — hash tables for match finding, tries for dictionary coding
- **micro-optimization** — memoryview, struct packing, bytearray pre-allocation
- **parallelism** — chunked compression with multiprocessing
- **memory-layout** — buffer reuse, cache-friendly access patterns
- **rewrite** — complete reimplementation with a different approach
