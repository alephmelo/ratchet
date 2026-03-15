# BPE Tokenizer — ratchet multi-metric example

A [Byte-Pair Encoding](https://en.wikipedia.org/wiki/Byte_pair_encoding)
tokenizer benchmark with **two competing primary metrics** and bandit-driven
strategy selection.

## Metrics

| metric | direction | description |
|--------|-----------|-------------|
| `throughput` | maximize | Encoding speed (MB/s) |
| `tokens_per_word` | minimize | Avg tokens per whitespace word (vocabulary quality) |
| `correctness` | constraint ≥ 100 | Round-trip fidelity: `decode(encode(x)) == x` |

The two primary metrics create natural tension: caching and indexing structures
speed up encoding but may alter merge order, affecting vocabulary quality.

## Baseline

The naive implementation scores ~0.05 MB/s throughput and ~1.79 tokens/word.

## Run

```bash
cd examples/bpe-tokenizer
ratchet loop
```

## Files

- `tokenizer.py` — Editable BPE tokenizer (train / encode / decode)
- `benchmark.py` — Read-only benchmark harness (128 KB corpus, 512 vocab size)
- `ratchet.yaml` — Multi-metric config with bandit enabled
