"""Custom LZ1 compression/decompression.

Stream format:
  - Groups of 8 operations, preceded by 1 control byte
  - Control byte bits (MSB first):
    0 = literal: 1 byte follows
    1 = match: 3 bytes follow (u16 LE offset + u8 encoded_length)
  - Offset: 1-65535 (16-bit)
  - encoded_length = actual_length - 3 (0-255, so length = 3-258)
"""

import struct

MIN_MATCH = 3
MAX_MATCH = 258
WINDOW_SIZE = 65536
HASH_BYTES = 4


def _hash_key(data: bytes, pos: int, n: int) -> bytes:
    """Return HASH_BYTES bytes starting at pos, or None if out of range."""
    if pos + HASH_BYTES > n:
        return None
    return data[pos:pos + HASH_BYTES]


def compress(data: bytes) -> bytes:
    n = len(data)
    if n == 0:
        return b''

    pos_map = {}
    out = bytearray()
    i = 0

    while i < n:
        items = []

        for _ in range(8):
            if i >= n:
                break

            match_advance = 1
            payload = bytes([data[i]])

            if i + HASH_BYTES <= n:
                key = data[i:i + HASH_BYTES]
                candidate = pos_map.get(key)
                if candidate is not None:
                    dist = i - candidate
                    if dist <= WINDOW_SIZE:
                        max_len = min(MAX_MATCH, n - i)
                        ml = HASH_BYTES
                        while ml < max_len and data[candidate + ml] == data[i + ml]:
                            ml += 1
                        if ml >= MIN_MATCH:
                            lazy_better = False
                            if ml < MAX_MATCH and i + 1 + HASH_BYTES <= n:
                                key2 = data[i + 1:i + 1 + HASH_BYTES]
                                c2 = pos_map.get(key2)
                                if c2 is not None and (i + 1) - c2 <= WINDOW_SIZE:
                                    max2 = min(MAX_MATCH, n - (i + 1))
                                    ml2 = HASH_BYTES
                                    while ml2 < max2 and data[c2 + ml2] == data[i + 1 + ml2]:
                                        ml2 += 1
                                    if ml2 > ml + 1:
                                        lazy_better = True

                            if not lazy_better and dist <= 65535:
                                payload = struct.pack('<H', dist) + bytes([ml - 3])
                                match_advance = ml

            # Update hash table for every input byte consumed
            for j in range(match_advance):
                if i + j >= HASH_BYTES:
                    key = data[i + j - HASH_BYTES:i + j]
                    pos_map[key] = i + j - HASH_BYTES

            items.append((match_advance > 1, payload))
            i += match_advance

        ctrl = 0
        payload = bytearray()
        for is_ref, pld in items:
            ctrl <<= 1
            if is_ref:
                ctrl |= 1
            payload += pld

        ctrl <<= 8 - len(items)
        out.append(ctrl & 0xFF)
        out += payload

    return bytes(out)


def decompress(data: bytes, uncompressed_size: int) -> bytes:
    out = bytearray()
    pos = 0
    while pos < len(data) and len(out) < uncompressed_size:
        ctrl = data[pos]
        pos += 1
        for _ in range(8):
            if pos >= len(data) or len(out) >= uncompressed_size:
                break
            if ctrl & 0x80:
                if pos + 3 > len(data):
                    return bytes(out)
                offset = data[pos] | (data[pos + 1] << 8)
                enc_len = data[pos + 2]
                pos += 3
                length = enc_len + 3
                if offset == 0 or offset > len(out):
                    return bytes(out)
                src = len(out) - offset
                for _ in range(length):
                    out.append(out[src])
                    src += 1
            else:
                out.append(data[pos])
                pos += 1
            ctrl <<= 1
    return bytes(out)
