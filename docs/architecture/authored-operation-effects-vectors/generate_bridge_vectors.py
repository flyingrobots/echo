# SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
"""Independent byte vectors for the bridge Action encodings of authored-operation-effects.md.

Every expected value is computed here from the layouts written in the design
document, with a standard-library-only implementation that shares no code with
warp-core. Run: ``python3 -I generate_bridge_vectors.py > bridge-vectors.json``.
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from blake3_reference import blake3  # noqa: E402  (path set on the line above)

DOC = "docs/architecture/authored-operation-effects.md"
OFFICIAL_BLAKE3 = (
    (0, "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"),
    (1024, "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7"),
    (1025, "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444"),
    (2048, "e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a"),
)
Z32 = bytes(32)


def check_blake3() -> None:
    """Refuse to emit vectors unless BLAKE3 matches the official test vectors."""
    for length, expected in OFFICIAL_BLAKE3:
        if blake3(bytes(i % 251 for i in range(length))).hex() != expected:
            raise SystemExit(f"blake3_reference fails the official vector for length {length}")


def label_id(label: str) -> bytes:
    """A 32-byte vector input named by a label (an input, not an expected value)."""
    return blake3(b"bridge-vector-input:" + label.encode())


def u8(value: int) -> bytes:
    return struct.pack("<B", value)


def u32(value: int) -> bytes:
    return struct.pack("<I", value)


def u64(value: int) -> bytes:
    return struct.pack("<Q", value)


def rng(start: int, length: int) -> bytes:
    """WSC `Range` row: start then length, little-endian u64 each."""
    return u64(start) + u64(length)


def sized(row: bytes, size: int) -> bytes:
    if len(row) != size:
        raise SystemExit(f"row is {len(row)} bytes, layout says {size}")
    return row


def slot_row(cls: int, warp: bytes, local: bytes, *, edge_type: bytes = Z32, owner: int = 0, plane: int = 0) -> bytes:
    return sized(u8(cls) + u8(owner) + u8(plane) + bytes(5) + warp + local + edge_type, 104)


def read_row(slot: bytes, outcome: int, *, bucket: int = 0, read_type: bytes = Z32, subject: bytes = Z32,
             pairs: bytes = bytes(16), bucket_len: int = 0) -> bytes:
    return sized(slot + u8(outcome) + u8(bucket) + bytes(6) + read_type + subject + pairs + u64(bucket_len), 200)


def effect_row(kind: int, key_warp: bytes, key_local: bytes, *, type_id: bytes = Z32, atom_type: bytes = Z32,
               atom: bytes = bytes(16), expected: bytes = Z32, from_local: bytes = Z32, to_local: bytes = Z32,
               expected_to: bytes = Z32) -> bytes:
    body = u8(kind) + bytes(7) + key_warp + key_local + type_id + atom_type + atom + expected
    return sized(body + from_local + to_local + expected_to, 280)


def outcome_row(outcome: int, code: int, authored: bytes = Z32) -> bytes:
    return sized(u8(outcome) + u8(code) + bytes(6) + authored, 40)


def bridge_row(fields: dict[str, object]) -> bytes:
    """BridgeEntryRow, 496 bytes; fields absent from ``fields`` are zero."""
    def h(name: str) -> bytes:
        value = fields.get(name, Z32)
        if not isinstance(value, bytes):
            raise TypeError(name)
        return value

    def n(name: str) -> int:
        value = fields.get(name, 0)
        if not isinstance(value, int):
            raise TypeError(name)
        return value
    tags = bytes(n(t) for t in ("posture", "patch", "validity", "evidence", "outcome", "code")) + bytes(2)
    digests = b"".join(h(d) for d in ("proposal_id", "target_set", "write_set", "patch_digest", "read_set",
                                       "origin_basis", "dependency_set", "compared_values"))
    tail = bytes(112) + h("authored_id") + u64(0) + b"".join(u64(n(b)) for b in
                                                             ("steps", "read_bytes", "write_bytes", "allocated_bytes"))
    result = fields.get("result", bytes(16))
    if not isinstance(result, bytes):
        raise TypeError("result")
    return sized(tags + digests + tail + h("result_identity") + result, 496)


def set_digest(domain: bytes, rows: list[bytes]) -> bytes:
    return blake3(domain + u64(len(rows)) + b"".join(rows))


def read_set_digest(rows: list[bytes], pairs: list[bytes]) -> bytes:
    return blake3(b"echo.bridge.read-set.v2\0" + u64(len(rows)) + b"".join(rows) + u64(len(pairs)) + b"".join(pairs))


def lp(data: bytes) -> bytes:
    return u64(len(data)) + data


def pad8(data: bytes) -> bytes:
    return data + bytes(-len(data) % 8)


def hx(data: bytes) -> str:
    return data.hex()


SRC_DOC = f"computed by this generator from the layout in {DOC}"
SRC_RECEIPT = ("computed by this generator from the receipt digest recipe of warp-core receipt.rs "
               "compute_tick_receipt_digest at 53c78e8a, re-implemented here; not from the production encoder")
SRC_V1 = ("computed by this generator from the :v1 recipes of echo_operation.rs action_batch_rewrites_digest and "
          "action_batch_composition_digest_from_parts at 53c78e8a, re-implemented here")
SRC_SCOPE = "computed by this generator from engine_impl.rs scope_hash (v0) at 53c78e8a and Decision 1 of " + DOC


def slot_key(row: bytes) -> tuple[int, bytes, bytes, bytes, int, int]:
    """Sort key of a SlotRow: class, warp_id, local_id, edge_type, owner, plane."""
    return (row[0], row[8:40], row[40:72], row[72:104], row[1], row[2])


class Vectors:
    """Accumulates named vectors with their inputs and the source of each expected value."""

    def __init__(self) -> None:
        self.items: list[dict[str, object]] = []

    def add(self, name: str, inputs: dict[str, str], expected: bytes, source: str, *, digest: bool = False) -> bytes:
        entry: dict[str, object] = {"name": name, "inputs": inputs, "source": source}
        if digest:
            entry["expected_digest_hex"] = hx(expected)
        else:
            entry["expected_bytes_hex"] = hx(expected)
            entry["expected_len"] = len(expected)
            entry["expected_blake3_hex"] = hx(blake3(expected))
        self.items.append(entry)
        return expected


def scope_vectors(v: Vectors, rule_id: bytes, root_warp: bytes, invocation: bytes) -> bytes:
    scope = v.add("synthetic_scope_node_key", {"root_warp_id": hx(root_warp), "invocation_id": hx(invocation)},
                  root_warp + invocation, SRC_SCOPE)
    return v.add("synthetic_scope_hash", {"rule_id": hx(rule_id), "scope": hx(scope)},
                 blake3(rule_id + scope), SRC_SCOPE, digest=True)


def value_vectors(v: Vectors, warp: bytes, ids: dict[str, bytes], atom: bytes) -> dict[str, bytes]:
    out = {
        "node": blake3(b"echo.bridge.node-value.v1\0" + ids["node_type"]),
        "atom": blake3(b"echo.bridge.attachment-value.v1\0" + u8(0) + ids["atom_type"] + lp(atom)),
        "portal": blake3(b"echo.bridge.attachment-value.v1\0" + u8(1) + warp),
        "edge": blake3(b"echo.bridge.edge-value.v1\0" + ids["edge_type"] + ids["a"] + ids["c"]),
    }
    for name, value in out.items():
        v.add(f"value_digest_{name}", {"atom_bytes_hex": hx(atom)}, value, SRC_DOC, digest=True)
    return out


def main() -> None:
    check_blake3()
    v = Vectors()
    names = ("root_warp", "invocation", "rule_id", "node_type", "atom_type", "edge_type", "a", "c", "edge",
             "basis", "submission", "ingress", "program", "worldline", "head", "commit", "fixed_prep",
             "fixed_patch", "fixed_result", "fixed_basis", "fixed_footprint", "fixed_rule", "fixed_scope_hash",
             "fixed_submission", "committed_patch")
    ids = {name: label_id(name) for name in names}
    w = ids["root_warp"]
    scope_h = scope_vectors(v, ids["rule_id"], w, ids["invocation"])
    atom, result = b"version 8", b"\xa1\x62ok\xf5"
    vals = value_vectors(v, w, ids, atom)

    pairs = [v.add("edge_pair_row", {}, sized(ids["edge"] + ids["c"], 64), SRC_DOC)]
    rows = sorted([
        read_row(slot_row(0, w, ids["a"]), 0, bucket=1, read_type=ids["node_type"], subject=vals["node"],
                 bucket_len=1),
        read_row(slot_row(1, w, ids["a"]), 0, read_type=ids["atom_type"], subject=vals["atom"]),
        read_row(slot_row(3, w, ids["a"], edge_type=ids["edge_type"]), 2, pairs=rng(0, 1)),
    ], key=lambda r: (slot_key(r[:104]), r[112:144]))
    for index, row in enumerate(rows):
        v.add(f"read_row_{index}", {}, row, SRC_DOC)
    read_set = v.add("read_set_digest", {}, read_set_digest(rows, pairs), SRC_DOC, digest=True)
    v.add("read_set_digest_empty", {}, read_set_digest([], []), SRC_DOC, digest=True)

    effects = [
        effect_row(2, w, ids["a"], atom_type=ids["atom_type"], atom=rng(0, len(atom)), expected=vals["atom"]),
        effect_row(3, w, ids["edge"], type_id=ids["edge_type"], from_local=ids["a"], to_local=ids["c"]),
    ]
    for index, row in enumerate(effects):
        v.add(f"effect_row_{index}", {}, row, SRC_DOC)
    blob = pad8(atom) + result
    header = (ids["rule_id"] + ids["invocation"] + ids["basis"] + read_set + u64(7) + u64(352) + u64(192) + u64(48)
              + rng(len(pad8(atom)), len(result)) + u64(len(effects)) + u64(len(blob)))
    image = v.add("proposal_image_EBPRP001", {"costs": "steps 7, read 352, write 192, allocated 48"},
                  b"EBPRP001" + sized(header, 192) + b"".join(effects) + blob, SRC_DOC)
    proposal = v.add("proposal_identity", {}, blake3(b"echo.bridge.effect-proposal.v1\0" + image), SRC_DOC,
                     digest=True)

    targets = sorted([slot_row(1, w, ids["a"]), slot_row(2, w, ids["edge"])], key=slot_key)
    writes = sorted(targets + [slot_row(3, w, ids["a"], edge_type=ids["edge_type"])], key=slot_key)
    target_set = v.add("target_set_digest", {}, set_digest(b"echo.bridge.target-set.v1\0", targets), SRC_DOC,
                       digest=True)
    write_set = v.add("write_set_digest", {}, set_digest(b"echo.bridge.write-set.v1\0", writes), SRC_DOC,
                      digest=True)
    authored = v.add("authored_id", {"code": "BasisNotCanonical", "detail_hex": "f6"},
                     blake3(b"echo.bridge.authored-obstruction.v1\0" + ids["program"] + lp(b"BasisNotCanonical")
                            + lp(b"\xf6")), SRC_DOC, digest=True)
    result_id = v.add("result_identity", {"output_type": "link.result/v1", "result_hex": hx(result)},
                      blake3(b"echo.bridge.result.v1\0" + ids["program"] + lp(b"link.result/v1") + lp(result)),
                      SRC_DOC, digest=True)
    emit_rows_and_records(v, ids, {"scope_h": scope_h, "proposal": proposal, "target_set": target_set,
                                   "write_set": write_set, "read_set": read_set, "authored": authored,
                                   "result_id": result_id, "result": result})
    inputs = {"rule": "each named input is BLAKE3 over 'bridge-vector-input:' and its name",
              "values": {name: hx(value) for name, value in ids.items()}}
    json.dump({"generator": "generate_bridge_vectors.py", "document": DOC, "inputs": inputs,
               "vectors": v.items}, sys.stdout,
              indent=1)
    sys.stdout.write("\n")


def emit_rows_and_records(v: Vectors, ids: dict[str, bytes], ctx: dict[str, object]) -> None:
    """BridgeEntryRow per posture, OutcomeRow, EOACT004, EPRV0004, receipt v4/v5 and batch digests."""
    def b(name: str) -> bytes:
        value = ctx[name]
        if not isinstance(value, bytes):
            raise TypeError(name)
        return value
    cost = {"steps": 9, "read_bytes": 512, "write_bytes": 192, "allocated_bytes": 48}
    proposed = {"posture": 2, "proposal_id": b("proposal"), "target_set": b("target_set"),
                "read_set": b("read_set"), **cost}
    derived = {**proposed, "patch": 1, "write_set": b("write_set"), "patch_digest": ids["committed_patch"]}
    empty = b("read_set_empty") if "read_set_empty" in ctx else None
    rows = {
        "D_basis_changed": {"posture": 1, "outcome": 1, "code": 0,
                            "read_set": empty or read_set_digest([], [])},
        "G_authored": {"posture": 1, "outcome": 2, "authored_id": b("authored"), "read_set": b("read_set"),
                       **cost},
        "K_dangling_reference": {**proposed, "outcome": 1, "code": 10},
        "L_footprint_mismatch": {**derived, "outcome": 1, "code": 11},
        "N_conflict": dict(derived),
        "O_committed": {**derived, "result_identity": b("result_id"), "result": rng(0, len(b("result")))},
    }
    built = {name: v.add(f"bridge_entry_row_{name}", {}, bridge_row(fields), SRC_DOC)
             for name, fields in rows.items()}
    not_bridge = v.add("bridge_entry_row_not_bridge", {}, bridge_row({}), SRC_DOC)
    outcome_for = {name: outcome_row(f.get("outcome", 0), f.get("code", 0), f.get("authored_id", Z32))
                   for name, f in rows.items()}
    for name, row in outcome_for.items():
        v.add(f"outcome_row_{name}", {}, row, SRC_DOC)
    receipt_v5 = emit_tick(v, ids, b("scope_h"), not_bridge, built["O_committed"], b("result"))
    coordinate = sized(ids["worldline"] + ids["head"] + u64(5) + u64(12) + ids["commit"] + receipt_v5 + u64(1), 152)
    v.add("decision_coordinate", {"worldline_tick_after": "5", "commit_global_tick": "12", "member_index": "1"},
          coordinate, SRC_DOC)
    prefix = b"EOACT004" + ids["submission"] + ids["ingress"]
    v.add("EOACT004_committed", {}, prefix + u8(1) + outcome_for["O_committed"] + built["O_committed"] + coordinate
          + lp(b("result")), SRC_DOC)
    v.add("EOACT004_obstructed_dangling_reference", {}, prefix + u8(2) + outcome_for["K_dangling_reference"]
          + built["K_dangling_reference"] + coordinate, SRC_DOC)
    v.add("EOACT004_conflict", {"blockers": "[0]"}, prefix + u8(3) + outcome_for["N_conflict"] + built["N_conflict"]
          + coordinate + u64(1) + u32(0), SRC_DOC)
    emit_batch_digests(v, ids, b("proposal"), b("result_id"), b("write_set"))


def receipt_preimage(version: int, entries: list[tuple[bytes, bytes, bytes, int]],
                     attribution: list[tuple[bytes, list[int]]]) -> bytes:
    out = struct.pack("<H", version) + u64(len(entries))
    for rule_id, scope_h, scope, disposition in entries:
        out += rule_id + scope_h + scope + u8(disposition)
    out += u64(len(attribution))
    for submission, members in attribution:
        out += submission + u8(1) + u64(len(members)) + b"".join(u32(m) for m in members)
    return out


def emit_tick(v: Vectors, ids: dict[str, bytes], scope_h: bytes, fixed_row: bytes, bridge: bytes,
              result: bytes) -> bytes:
    """A two-entry bridge Tick: entry 0 a fixed applied Action, entry 1 a committed bridge Action."""
    w = ids["root_warp"]
    fixed_scope = w + ids["a"]
    entries = [(ids["fixed_rule"], ids["fixed_scope_hash"], fixed_scope, 1),
               (ids["rule_id"], scope_h, w + ids["invocation"], 1)]
    attribution = sorted([(ids["fixed_submission"], [0]), (ids["submission"], [1])])
    v4 = receipt_preimage(4, entries, attribution)
    v.add("receipt_digest_v4_same_entries", {"note": "legacy recipe over the same entries"}, blake3(v4),
          SRC_RECEIPT, digest=True)
    table = fixed_row + bridge
    blob = result
    eprv0003_payload = label_id("eprv0003-payload") + b"opaque-EPRV0003-payload"
    record = pad8(b"EPRV0004" + eprv0003_payload) + table + lp(blob)
    v.add("EPRV0004_record", {"eprv0003_payload_hex": hx(eprv0003_payload),
                              "note": "the EPRV0003 payload is an opaque input; phase 2 copies it byte for byte"},
          record, SRC_DOC)
    v5 = receipt_preimage(5, entries, attribution) + table + lp(blob)
    v.add("receipt_digest_v5_preimage", {}, v5, SRC_DOC + "; v4 part per " + SRC_RECEIPT)
    return v.add("receipt_digest_v5", {}, blake3(v5), SRC_DOC, digest=True)


def emit_batch_digests(v: Vectors, ids: dict[str, bytes], proposal: bytes, result_id: bytes,
                       write_set: bytes) -> None:
    fixed = [ids["fixed_prep"], ids["fixed_patch"], ids["fixed_result"], ids["fixed_basis"], ids["fixed_footprint"]]
    bridge = [proposal, ids["committed_patch"], result_id, ids["basis"], write_set]
    patch = ids["committed_patch"]
    v.add("rewrites_v1_fixed_only", {}, blake3(b"echo:operation-action-batch-rewrites:v1\0" + u64(1) + fixed[0]),
          SRC_V1, digest=True)
    v.add("composition_v1_fixed_only", {}, blake3(b"echo:operation-action-batch-composition:v1\0" + patch + u64(1)
                                                  + b"".join(fixed)), SRC_V1, digest=True)
    v.add("rewrites_v2_mixed", {}, blake3(b"echo:operation-action-batch-rewrites:v2\0" + u64(2) + u8(1) + fixed[0]
                                          + u8(2) + bridge[0]), SRC_DOC, digest=True)
    v.add("composition_v2_mixed", {}, blake3(b"echo:operation-action-batch-composition:v2\0" + patch + u64(2)
                                             + u8(1) + b"".join(fixed) + u8(2) + b"".join(bridge)), SRC_DOC,
          digest=True)
    v.add("rewrites_v2_bridge_tick_fixed_member_only", {"note": "bridge obstructed; only the fixed member applied"},
          blake3(b"echo:operation-action-batch-rewrites:v2\0" + u64(1) + u8(1) + fixed[0]), SRC_DOC, digest=True)
    v.add("composition_v2_bridge_tick_fixed_member_only", {},
          blake3(b"echo:operation-action-batch-composition:v2\0" + patch + u64(1) + u8(1) + b"".join(fixed)),
          SRC_DOC, digest=True)


if __name__ == "__main__":
    main()
