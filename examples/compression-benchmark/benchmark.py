#!/usr/bin/env python3
"""
Compression benchmark for ratchet.

Generates reproducible test data with mixed patterns (repetitive runs,
structured text, pseudo-random bytes) and measures:
  - throughput: compression speed in MB/s
  - ratio: compression ratio (original_size / compressed_size), higher = better
  - correctness: 100.0 if decompress(compress(data)) == data for all inputs, else 0.0

The benchmark runs multiple rounds and reports the median throughput to
reduce noise.
"""

import random
import time

from compress import compress, decompress

# ---------------------------------------------------------------------------
# Test data generation (deterministic)
# ---------------------------------------------------------------------------

SEED = 42
TOTAL_SIZE = 2 * 1024 * 1024  # 2 MB total test corpus
ROUNDS = 5


def _generate_corpus(seed: int, size: int) -> list[bytes]:
    """Generate a list of byte buffers totalling approximately `size` bytes."""
    rng = random.Random(seed)
    corpus = []
    remaining = size

    while remaining > 0:
        # Weighted distribution: mostly compressible data so the naive RLE
        # baseline barely passes ratio > 1.0, with some hard-to-compress
        # chunks to reward smarter algorithms.
        r = rng.random()
        if r < 0.45:
            kind = "runs"
        elif r < 0.75:
            kind = "text"
        elif r < 0.88:
            kind = "mixed"
        else:
            kind = "random"
        chunk_size = min(rng.randint(4096, 65536), remaining)

        if kind == "runs":
            # Long runs of repeated bytes — RLE-friendly
            buf = bytearray()
            while len(buf) < chunk_size:
                byte = rng.randint(0, 255)
                length = rng.randint(10, 300)
                buf.extend(bytes([byte]) * length)
            corpus.append(bytes(buf[:chunk_size]))

        elif kind == "text":
            # Simulated structured text (ASCII with repeated words)
            words = [
                "the",
                "quick",
                "brown",
                "fox",
                "jumps",
                "over",
                "lazy",
                "dog",
                "lorem",
                "ipsum",
                "dolor",
                "sit",
                "amet",
                "data",
                "compress",
                "algorithm",
                "benchmark",
                "optimize",
                "buffer",
            ]
            buf = bytearray()
            while len(buf) < chunk_size:
                line_words = [rng.choice(words) for _ in range(rng.randint(5, 15))]
                line = " ".join(line_words) + "\n"
                buf.extend(line.encode())
            corpus.append(bytes(buf[:chunk_size]))

        elif kind == "random":
            # Pseudo-random bytes — hard to compress
            corpus.append(bytes(rng.getrandbits(8) for _ in range(chunk_size)))

        else:
            # Mixed: alternating compressible and random segments
            buf = bytearray()
            while len(buf) < chunk_size:
                if rng.random() < 0.5:
                    byte = rng.randint(0, 255)
                    buf.extend(bytes([byte]) * rng.randint(20, 100))
                else:
                    seg = rng.randint(16, 64)
                    buf.extend(bytes(rng.getrandbits(8) for _ in range(seg)))
            corpus.append(bytes(buf[:chunk_size]))

        remaining -= chunk_size

    return corpus


# ---------------------------------------------------------------------------
# Benchmark
# ---------------------------------------------------------------------------


def main() -> None:
    corpus = _generate_corpus(SEED, TOTAL_SIZE)
    total_bytes = sum(len(b) for b in corpus)

    # --- Correctness check ---
    correct = True
    for i, original in enumerate(corpus):
        compressed = compress(original)
        restored = decompress(compressed)
        if restored != original:
            correct = False
            break

    correctness = 100.0 if correct else 0.0

    # --- Compression ratio (single pass) ---
    compressed_total = 0
    for buf in corpus:
        compressed_total += len(compress(buf))

    ratio = total_bytes / compressed_total if compressed_total > 0 else 0.0

    # --- Throughput (median of ROUNDS) ---
    timings: list[float] = []
    for _ in range(ROUNDS):
        t0 = time.perf_counter()
        for buf in corpus:
            compress(buf)
        elapsed = time.perf_counter() - t0
        timings.append(elapsed)

    timings.sort()
    median_elapsed = timings[len(timings) // 2]
    throughput_mbs = (
        (total_bytes / (1024 * 1024)) / median_elapsed if median_elapsed > 0 else 0.0
    )

    print(f"throughput: {throughput_mbs:.2f}")
    print(f"ratio: {ratio:.3f}")
    print(f"correctness: {correctness:.1f}")


if __name__ == "__main__":
    main()
