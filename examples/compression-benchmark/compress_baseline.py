"""
Lossless byte-stream compressor.

Exports:
    compress(data: bytes) -> bytes
    decompress(data: bytes) -> bytes

The baseline is a naive run-length encoding (RLE) that encodes each run as
(count, byte). This is deliberately simple and slow — there is enormous room
for algorithmic, data-structure, micro-optimization, and parallelism
improvements.

Rules:
  - compress() and decompress() must remain the public API.
  - decompress(compress(data)) must equal data for all inputs (lossless).
  - Only the Python stdlib is allowed (no pip packages).
"""


def compress(data: bytes) -> bytes:
    """Compress data using naive run-length encoding."""
    if not data:
        return b""

    out = bytearray()
    i = 0
    n = len(data)

    while i < n:
        byte = data[i]
        count = 1
        while i + count < n and data[i + count] == byte and count < 255:
            count += 1
        out.append(count)
        out.append(byte)
        i += count

    return bytes(out)


def decompress(data: bytes) -> bytes:
    """Decompress RLE-encoded data."""
    if not data:
        return b""

    out = bytearray()
    i = 0
    n = len(data)

    while i + 1 < n:
        count = data[i]
        byte = data[i + 1]
        out.extend(bytes([byte]) * count)
        i += 2

    return bytes(out)
