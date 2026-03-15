"""
Byte-Pair Encoding (BPE) tokenizer.

Exports:
    train(corpus: str, vocab_size: int) -> dict
    encode(text: str, merges: dict) -> list[int]
    decode(token_ids: list[int], merges: dict) -> str

The baseline is a naive BPE implementation that:
  - Trains by repeatedly scanning the full token list for the most frequent pair
  - Encodes by applying merges one-at-a-time with linear scans
  - Has no caching, no indexing, no pre-compiled merge tables

This is deliberately simple and slow — there is enormous room for improvement
in training speed, encoding speed, and vocabulary quality.

Rules:
  - train(), encode(), and decode() must remain the public API.
  - decode(encode(text, m), m) must equal text for all inputs (lossless).
  - Only the Python stdlib is allowed (no pip packages).
"""


def train(corpus: str, vocab_size: int) -> dict:
    """Train BPE merges from a corpus.

    Returns a dict with:
      - "merges": list of (pair, new_id) tuples in merge order
      - "vocab_size": final vocabulary size
    """
    # Start with byte-level vocabulary (0-255)
    tokens = list(corpus.encode("utf-8"))
    next_id = 256
    merges = []

    while next_id < vocab_size:
        # Count all adjacent pairs — O(n) per iteration
        pair_counts: dict[tuple[int, int], int] = {}
        for i in range(len(tokens) - 1):
            pair = (tokens[i], tokens[i + 1])
            pair_counts[pair] = pair_counts.get(pair, 0) + 1

        if not pair_counts:
            break

        # Find the most frequent pair
        best_pair = max(pair_counts, key=pair_counts.__getitem__)
        if pair_counts[best_pair] < 2:
            break  # No pair occurs more than once

        # Replace all occurrences of best_pair with next_id — O(n)
        new_tokens = []
        i = 0
        while i < len(tokens):
            if i < len(tokens) - 1 and (tokens[i], tokens[i + 1]) == best_pair:
                new_tokens.append(next_id)
                i += 2
            else:
                new_tokens.append(tokens[i])
                i += 1

        merges.append((best_pair, next_id))
        tokens = new_tokens
        next_id += 1

    return {"merges": merges, "vocab_size": next_id}


def encode(text: str, merges: dict) -> list[int]:
    """Encode text into token IDs using trained BPE merges."""
    tokens = list(text.encode("utf-8"))

    # Apply each merge in order — O(merges * n) total
    for (a, b), new_id in merges["merges"]:
        new_tokens = []
        i = 0
        while i < len(tokens):
            if i < len(tokens) - 1 and tokens[i] == a and tokens[i + 1] == b:
                new_tokens.append(new_id)
                i += 2
            else:
                new_tokens.append(tokens[i])
                i += 1
        tokens = new_tokens

    return tokens


def decode(token_ids: list[int], merges: dict) -> str:
    """Decode token IDs back to text."""
    # Build reverse mapping: new_id -> (a, b)
    reverse = {}
    for (a, b), new_id in merges["merges"]:
        reverse[new_id] = (a, b)

    # Expand each token to bytes
    def expand(token_id: int) -> list[int]:
        if token_id < 256:
            return [token_id]
        if token_id in reverse:
            a, b = reverse[token_id]
            return expand(a) + expand(b)
        raise ValueError(f"Unknown token ID: {token_id}")

    byte_list = []
    for tid in token_ids:
        byte_list.extend(expand(tid))

    return bytes(byte_list).decode("utf-8")
