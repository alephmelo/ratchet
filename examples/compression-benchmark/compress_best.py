"""Lossless byte-stream compressor — best result from bandit experiment.

This version achieved 295.66 MB/s (34.8x improvement over baseline).
It uses RLE-tuned deflate (zlib level 1, Z_RLE strategy) with parallel
chunk processing via ThreadPoolExecutor.

Strategy lineage: algorithm -> parallelism -> rewrite -> algorithm -> parallelism
"""

from concurrent.futures import ThreadPoolExecutor
import os
import zlib


_RAW = 0
_ZLIB = 1
_ZLIB_PARALLEL = 2

_CHUNK_SIZE = 262144
_PARALLEL_MIN_SIZE = _CHUNK_SIZE * 2
_MAX_WORKERS = os.cpu_count() or 1
_POOL = ThreadPoolExecutor(max_workers=_MAX_WORKERS) if _MAX_WORKERS > 1 else None


def _compress_chunk(chunk: bytes) -> bytes:
    compressor = zlib.compressobj(level=1, strategy=zlib.Z_RLE)
    return compressor.compress(chunk) + compressor.flush()


def compress(data: bytes) -> bytes:
    """Compress data using RLE-tuned deflate with raw fallback."""
    if not data:
        return b""

    if _POOL is not None and len(data) >= _PARALLEL_MIN_SIZE:
        chunks = [data[i : i + _CHUNK_SIZE] for i in range(0, len(data), _CHUNK_SIZE)]
        compressed_chunks = list(_POOL.map(_compress_chunk, chunks))

        packed = bytearray()
        packed.extend(len(compressed_chunks).to_bytes(4, "little"))
        for chunk in compressed_chunks:
            packed.extend(len(chunk).to_bytes(4, "little"))
            packed.extend(chunk)

        if len(packed) + 1 < len(data):
            return b"\x02" + bytes(packed)

    compressor = zlib.compressobj(level=1, strategy=zlib.Z_RLE)
    compressed = compressor.compress(data) + compressor.flush()
    if len(compressed) + 1 < len(data):
        return b"\x01" + compressed
    return b"\x00" + data


def decompress(data: bytes) -> bytes:
    """Decompress single-shot zlib/raw stream."""
    if not data:
        return b""

    mode = data[0]
    payload = data[1:]

    if mode == 0:
        return payload
    if mode == 2:
        if len(payload) < 4:
            raise ValueError("Invalid compressed stream")
        chunk_count = int.from_bytes(payload[:4], "little")
        pos = 4
        chunks = []
        for _ in range(chunk_count):
            if pos + 4 > len(payload):
                raise ValueError("Invalid compressed stream")
            clen = int.from_bytes(payload[pos : pos + 4], "little")
            pos += 4
            end = pos + clen
            if end > len(payload):
                raise ValueError("Invalid compressed stream")
            chunks.append(payload[pos:end])
            pos = end
        if pos != len(payload):
            raise ValueError("Invalid compressed stream")
        try:
            if _POOL is not None and chunk_count > 1:
                return b"".join(_POOL.map(zlib.decompress, chunks))
            return b"".join(zlib.decompress(chunk) for chunk in chunks)
        except zlib.error as exc:
            raise ValueError("Invalid compressed stream") from exc
    if mode != 1:
        raise ValueError("Invalid compressed stream")
    try:
        return zlib.decompress(payload)
    except zlib.error as exc:
        raise ValueError("Invalid compressed stream") from exc
