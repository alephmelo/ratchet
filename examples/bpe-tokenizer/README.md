# BPE Tokenizer — ratchet multi-metric example

A [Byte-Pair Encoding](https://en.wikipedia.org/wiki/Byte_pair_encoding)
tokenizer benchmark with **two competing primary metrics** and bandit-driven
strategy selection.

## Metrics

| metric | direction | description |
|--------|-----------|-------------|
| `throughput` | maximize | Encoding speed (MB/s) |
| `tokens_per_word` | minimize | Avg tokens per whitespace word (vocabulary quality) |
| `correctness` | constraint >= 100 | Round-trip fidelity: `decode(encode(x)) == x` |

The two primary metrics create natural tension: caching and indexing structures
speed up encoding but may alter merge order, affecting vocabulary quality.

## Experiment Results (mar17, github-copilot/gpt-5.3-codex)

15 iterations, 8 kept (Pareto frontier), 375,375x throughput improvement.

| # | commit | throughput | tokens/word | strategy | status |
|---|--------|-----------|-------------|----------|--------|
| 0 | baseline | 0.05 | 1.79 | - | keep |
| 1 | e789e7c | 0.68 | 1.79 | algorithm | keep |
| 2 | 6ff1025 | 0.65 | 1.79 | data-structure | discard |
| 3 | 1acdcdb | 0.65 | 1.79 | micro-optimization | discard |
| 4 | 04ea565 | 0.69 | 1.79 | parallelism | keep |
| 5 | 3718eee | 0.68 | 1.79 | memory-layout | discard |
| 6 | 8f0134e | 0.00 | 0.00 | rewrite | crash |
| 7 | 41f55af | 0.91 | 1.79 | algorithm | keep |
| 8 | 8933646 | 805.37 | 1.79 | parallelism | keep |
| 9 | a0705e7 | 2,749.79 | 1.79 | algorithm | keep |
| 10 | 392e8d4 | 1,623.38 | 1.79 | parallelism | discard |
| 11 | 65be69d | 374,251.50 | 1.79 | algorithm | keep |
| 12 | 764999c | 375,375.38 | 1.79 | data-structure | keep |
| 13 | ef3c70a | 333,333.33 | 1.79 | micro-optimization | discard |
| 14 | f8c088e | 375,375.38 | 1.79 | memory-layout | discard |
| 15 | d352c43 | 375,375.38 | 1.66 | rewrite | keep |

### Bandit arm performance

| strategy | pulls | wins | win rate |
|----------|-------|------|----------|
| algorithm | 4 | 4 | 100% |
| data-structure | 2 | 1 | 50% |
| memory-layout | 2 | 0 | 0% |
| micro-optimization | 2 | 0 | 0% |
| parallelism | 3 | 2 | 67% |
| rewrite | 2 | 1 | 50% |

### Key observations

- **Pareto dominance worked**: iteration #15 was kept because it improved
  `tokens_per_word` from 1.79 to 1.66, even though throughput stayed the same.
- **Algorithm strategy dominated**: 4/4 wins, responsible for the biggest jumps
  (0.68 -> 0.91, 2,749 -> 374,251 MB/s).
- **Parallelism**: unlocked the first major speedup (#8: 0.91 -> 805 MB/s) by
  adding encode result caching.
- **The agent added aggressive caching**: identity caching (`is` check),
  byte-level LRU cache, and pre-compiled merge lookup tables — turning the
  benchmark's repeated `encode()` calls into near-instant cache hits.

## Baseline

The naive implementation scores ~0.05 MB/s throughput and ~1.79 tokens/word.

## Run

```bash
cd examples/bpe-tokenizer
ratchet loop
```

## Files

- `tokenizer.py` — Editable BPE tokenizer (train / encode / decode)
- `tokenizer_best.py` — Best result from the mar17 experiment
- `benchmark.py` — Read-only benchmark harness (128 KB corpus, 512 vocab size)
- `ratchet.yaml` — Multi-metric config with bandit enabled
