#!/usr/bin/env python3
"""
BPE tokenizer benchmark for ratchet.

Generates a reproducible text corpus (~500 KB of mixed English-like text)
and measures:
  - throughput:   encoding speed in MB/s (higher is better)
  - tokens_per_word: average number of tokens per whitespace-delimited word
                     (lower is better — measures vocabulary quality)
  - correctness:  100.0 if decode(encode(text)) == text for all inputs, else 0.0

The benchmark trains BPE on a portion of the corpus, then encodes/decodes
the full corpus. Training time is NOT included in throughput (only encoding).
"""

import random
import time

from tokenizer import train, encode, decode

# ---------------------------------------------------------------------------
# Corpus generation (deterministic)
# ---------------------------------------------------------------------------

SEED = 42
CORPUS_SIZE = 128 * 1024  # 128 KB text corpus
VOCAB_SIZE = 512  # target vocabulary size (256 byte tokens + 256 merges)
ROUNDS = 3  # timing rounds for throughput measurement

# Word pools designed to create realistic frequency distributions
# so that BPE can learn meaningful merges
COMMON_WORDS = [
    "the",
    "of",
    "and",
    "to",
    "in",
    "a",
    "is",
    "that",
    "for",
    "it",
    "was",
    "on",
    "are",
    "as",
    "with",
    "his",
    "they",
    "be",
    "at",
    "one",
    "have",
    "this",
    "from",
    "by",
    "not",
    "but",
    "what",
    "all",
    "were",
    "we",
]

TECHNICAL_WORDS = [
    "function",
    "algorithm",
    "optimization",
    "parameters",
    "implementation",
    "transformer",
    "attention",
    "embedding",
    "gradient",
    "backpropagation",
    "tokenizer",
    "vocabulary",
    "encoding",
    "compression",
    "neural",
    "inference",
    "training",
    "dataset",
    "benchmark",
    "performance",
    "architecture",
    "convolution",
    "activation",
    "normalization",
    "dropout",
    "regularization",
    "hyperparameter",
    "convergence",
    "overfitting",
    "batch",
]

RARE_WORDS = [
    "syzygy",
    "quixotic",
    "sesquipedalian",
    "pneumonoultramicroscopicsilicovolcanoconiosis",
    "antidisestablishmentarianism",
    "floccinaucinihilipilification",
    "supercalifragilisticexpialidocious",
    "pseudopseudohypoparathyroidism",
    "hippopotomonstrosesquippedaliophobia",
    "thyroparathyroidectomized",
]

PUNCTUATION = [".", ",", ";", ":", "!", "?", "-", "(", ")", '"']


def _generate_corpus(seed: int, size: int) -> str:
    """Generate a text corpus with realistic word frequency distribution."""
    rng = random.Random(seed)
    paragraphs = []
    current_size = 0

    while current_size < size:
        # Generate a paragraph
        sentences = []
        num_sentences = rng.randint(3, 8)

        for _ in range(num_sentences):
            words = []
            num_words = rng.randint(5, 25)

            for _ in range(num_words):
                r = rng.random()
                if r < 0.60:
                    word = rng.choice(COMMON_WORDS)
                elif r < 0.90:
                    word = rng.choice(TECHNICAL_WORDS)
                elif r < 0.97:
                    # Repeat with suffix to create compound words
                    base = rng.choice(TECHNICAL_WORDS)
                    suffix = rng.choice(
                        ["ing", "ed", "er", "tion", "ness", "ment", "ize"]
                    )
                    word = base + suffix
                else:
                    word = rng.choice(RARE_WORDS)

                # Occasional capitalization
                if rng.random() < 0.05:
                    word = word.capitalize()

                words.append(word)

                # Occasional punctuation within sentence
                if rng.random() < 0.1:
                    words.append(rng.choice(PUNCTUATION[:5]))

            sentence = " ".join(words)
            # Capitalize first letter
            sentence = sentence[0].upper() + sentence[1:]
            # End punctuation
            sentence += rng.choice([".", ".", ".", "!", "?"])
            sentences.append(sentence)

        paragraph = " ".join(sentences)
        paragraphs.append(paragraph)
        current_size += len(paragraph) + 2  # +2 for \n\n

    return "\n\n".join(paragraphs)[:size]


# ---------------------------------------------------------------------------
# Benchmark
# ---------------------------------------------------------------------------


def main() -> None:
    corpus = _generate_corpus(SEED, CORPUS_SIZE)
    total_bytes = len(corpus.encode("utf-8"))

    # --- Train BPE (time not counted for throughput) ---
    merges = train(corpus, VOCAB_SIZE)

    # --- Correctness check ---
    # Test on several slices of the corpus
    test_slices = [
        corpus[:1000],
        corpus[1000:5000],
        corpus[5000:20000],
        corpus,
    ]

    correct = True
    for text in test_slices:
        tokens = encode(text, merges)
        restored = decode(tokens, merges)
        if restored != text:
            correct = False
            break

    correctness = 100.0 if correct else 0.0

    # --- Tokens per word (vocabulary efficiency) ---
    # Encode the full corpus and count tokens
    all_tokens = encode(corpus, merges)
    num_tokens = len(all_tokens)
    num_words = len(corpus.split())
    tokens_per_word = num_tokens / num_words if num_words > 0 else 0.0

    # --- Throughput (median of ROUNDS, encoding only) ---
    timings: list[float] = []
    for _ in range(ROUNDS):
        t0 = time.perf_counter()
        encode(corpus, merges)
        elapsed = time.perf_counter() - t0
        timings.append(elapsed)

    timings.sort()
    median_elapsed = timings[len(timings) // 2]
    throughput_mbs = (
        (total_bytes / (1024 * 1024)) / median_elapsed if median_elapsed > 0 else 0.0
    )

    print(f"throughput: {throughput_mbs:.2f}")
    print(f"tokens_per_word: {tokens_per_word:.3f}")
    print(f"correctness: {correctness:.1f}")


if __name__ == "__main__":
    main()
