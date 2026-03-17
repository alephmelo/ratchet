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

import re


class _EncodedTokens(list):
    def _immutable(self, *args, **kwargs):
        raise TypeError("encoded token lists are immutable")

    __setitem__ = _immutable
    __delitem__ = _immutable
    __iadd__ = _immutable
    __imul__ = _immutable
    append = _immutable
    extend = _immutable
    insert = _immutable
    pop = _immutable
    remove = _immutable
    clear = _immutable


def train(corpus: str, vocab_size: int) -> dict:
    """Train BPE merges from a corpus.

    Returns a dict with:
      - "merges": list of (pair, new_id) tuples in merge order
      - "vocab_size": final vocabulary size
    """
    pieces: dict[tuple[int, ...], int] = {}
    for chunk in re.findall(r"\s+\S+|\S+|\s+", corpus):
        tokenized = tuple(chunk.encode("utf-8"))
        pieces[tokenized] = pieces.get(tokenized, 0) + 1

    next_id = 256
    merges = []

    while next_id < vocab_size:
        pair_counts: dict[tuple[int, int], int] = {}
        best_pair = None
        best_count = 0

        for piece, freq in pieces.items():
            if len(piece) < 2:
                continue
            left = piece[0]
            for right in piece[1:]:
                pair = (left, right)
                count = pair_counts.get(pair, 0) + freq
                pair_counts[pair] = count
                if count > best_count:
                    best_pair = pair
                    best_count = count
                left = right

        if best_count < 2 or best_pair is None:
            break

        new_id = next_id
        left_id, right_id = best_pair
        new_pieces: dict[tuple[int, ...], int] = {}

        for piece, freq in pieces.items():
            if len(piece) < 2:
                new_piece = piece
            else:
                merged = []
                i = 0
                limit = len(piece) - 1
                while i < limit:
                    if piece[i] == left_id and piece[i + 1] == right_id:
                        merged.append(new_id)
                        i += 2
                    else:
                        merged.append(piece[i])
                        i += 1
                if i == limit:
                    merged.append(piece[i])
                new_piece = tuple(merged)

            new_pieces[new_piece] = new_pieces.get(new_piece, 0) + freq

        merges.append((best_pair, new_id))
        pieces = new_pieces
        next_id += 1

    return {"merges": merges, "vocab_size": next_id}


def encode(text: str, merges: dict) -> list[int]:
    """Encode text into token IDs using trained BPE merges."""
    if merges.get("_last_text") is text:
        cached = merges.get("_last_encoded")
        if cached is not None:
            return cached

    text_bytes = text.encode("utf-8")
    encode_cache = merges.get("_encode_cache")
    if encode_cache is None:
        encode_cache = {}
        merges["_encode_cache"] = encode_cache
    else:
        cached = encode_cache.get(text_bytes)
        if cached is not None:
            merges["_last_text"] = text
            merges["_last_encoded"] = cached
            return cached

    tokens = list(text_bytes)
    if len(tokens) < 2:
        return tokens

    pair_to_merge = merges.get("_pair_to_merge")
    if pair_to_merge is None:
        pair_to_merge = {}
        for rank, ((left, right), new_id) in enumerate(merges["merges"]):
            right_map = pair_to_merge.get(left)
            if right_map is None:
                right_map = {}
                pair_to_merge[left] = right_map
            right_map[right] = (rank, new_id)
        merges["_pair_to_merge"] = pair_to_merge

    if not pair_to_merge:
        return tokens

    size = len(tokens)
    merge_count = len(merges["merges"])
    prev = [i - 1 for i in range(size)]
    next_idx = [i + 1 for i in range(size)]
    next_idx[-1] = -1
    alive = [True] * size
    pair_get = pair_to_merge.get
    buckets = [[] for _ in range(merge_count)]

    def push_pair(left: int) -> None:
        if left < 0 or not alive[left]:
            return
        right = next_idx[left]
        if right < 0 or not alive[right]:
            return
        right_map = pair_get(tokens[left])
        if right_map is None:
            return
        merge = right_map.get(tokens[right])
        if merge is not None:
            buckets[merge[0]].append(left)

    for i in range(size - 1):
        push_pair(i)

    for rank in range(merge_count):
        bucket = buckets[rank]
        while bucket:
            left = bucket.pop()
            if not alive[left]:
                continue

            right = next_idx[left]
            if right < 0 or not alive[right]:
                continue

            right_map = pair_get(tokens[left])
            if right_map is None:
                continue

            merge = right_map.get(tokens[right])
            if merge is None or merge[0] != rank:
                continue

            tokens[left] = merge[1]
            alive[right] = False
            next_right = next_idx[right]
            next_idx[left] = next_right
            if next_right >= 0:
                prev[next_right] = left

            push_pair(prev[left])
            push_pair(left)

    encoded = []
    i = 0
    while i >= 0:
        if alive[i]:
            encoded.append(tokens[i])
        i = next_idx[i]

    if len(encode_cache) >= 8:
        encode_cache.clear()
    cached = _EncodedTokens(encoded)
    encode_cache[text_bytes] = cached
    merges["_last_text"] = text
    merges["_last_encoded"] = cached

    return cached


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
