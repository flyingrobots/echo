# SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Pure-Python BLAKE3 hash (default 32-byte output, no keying), written from the BLAKE3 specification."""

from __future__ import annotations

_IV = (
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
    0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
)
_PERM = (2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8)
_CHUNK_START = 1
_CHUNK_END = 2
_PARENT = 4
_ROOT = 8
_BLOCK_BYTES = 64
_CHUNK_BYTES = 1024
_MASK = 0xFFFFFFFF


def _rotr(value: int, shift: int) -> int:
    return ((value >> shift) | (value << (32 - shift))) & _MASK


def _g(state: list[int], a: int, b: int, c: int, d: int, mx: int, my: int) -> None:
    state[a] = (state[a] + state[b] + mx) & _MASK
    state[d] = _rotr(state[d] ^ state[a], 16)
    state[c] = (state[c] + state[d]) & _MASK
    state[b] = _rotr(state[b] ^ state[c], 12)
    state[a] = (state[a] + state[b] + my) & _MASK
    state[d] = _rotr(state[d] ^ state[a], 8)
    state[c] = (state[c] + state[d]) & _MASK
    state[b] = _rotr(state[b] ^ state[c], 7)


def _round(state: list[int], m: list[int]) -> None:
    _g(state, 0, 4, 8, 12, m[0], m[1])
    _g(state, 1, 5, 9, 13, m[2], m[3])
    _g(state, 2, 6, 10, 14, m[4], m[5])
    _g(state, 3, 7, 11, 15, m[6], m[7])
    _g(state, 0, 5, 10, 15, m[8], m[9])
    _g(state, 1, 6, 11, 12, m[10], m[11])
    _g(state, 2, 7, 8, 13, m[12], m[13])
    _g(state, 3, 4, 9, 14, m[14], m[15])


def _compress(cv: tuple[int, ...], block: bytes, counter: int, block_len: int, flags: int) -> tuple[int, ...]:
    m = [int.from_bytes(block[i * 4:i * 4 + 4], "little") for i in range(16)]
    state = [
        *cv,
        _IV[0], _IV[1], _IV[2], _IV[3],
        counter & _MASK, (counter >> 32) & _MASK, block_len, flags,
    ]
    for round_index in range(7):
        _round(state, m)
        if round_index < 6:
            m = [m[p] for p in _PERM]
    return tuple(state[i] ^ state[i + 8] for i in range(8))


def _chunk_cv(chunk: bytes, counter: int, *, is_root: bool) -> tuple[int, ...]:
    cv: tuple[int, ...] = _IV
    blocks = [chunk[i:i + _BLOCK_BYTES] for i in range(0, len(chunk), _BLOCK_BYTES)] or [b""]
    for index, block in enumerate(blocks):
        flags = 0
        if index == 0:
            flags |= _CHUNK_START
        if index == len(blocks) - 1:
            flags |= _CHUNK_END
            if is_root:
                flags |= _ROOT
        cv = _compress(cv, block.ljust(_BLOCK_BYTES, b"\0"), counter, len(block), flags)
    return cv


def _parent_cv(left: tuple[int, ...], right: tuple[int, ...], *, is_root: bool) -> tuple[int, ...]:
    block = b"".join(word.to_bytes(4, "little") for word in (*left, *right))
    return _compress(_IV, block, 0, _BLOCK_BYTES, _PARENT | (_ROOT if is_root else 0))


def _subtree_cv(data: bytes, first_chunk: int, *, is_root: bool) -> tuple[int, ...]:
    chunk_count = max(1, -(-len(data) // _CHUNK_BYTES))
    if chunk_count == 1:
        return _chunk_cv(data, first_chunk, is_root=is_root)
    left_chunks = 1 << ((chunk_count - 1).bit_length() - 1)
    split = left_chunks * _CHUNK_BYTES
    left = _subtree_cv(data[:split], first_chunk, is_root=False)
    right = _subtree_cv(data[split:], first_chunk + left_chunks, is_root=False)
    return _parent_cv(left, right, is_root=is_root)


def blake3(data: bytes) -> bytes:
    """Return the 32-byte BLAKE3 digest of ``data``."""
    root = _subtree_cv(data, 0, is_root=True)
    return b"".join(word.to_bytes(4, "little") for word in root)
