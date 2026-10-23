// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Compatibility vectors for the patch digest and patch canonicalization.
//!
//! Each vector hands [`WarpTickPatchV1::new`] unsorted inputs with repeats.
//! The pinned canonical order, digest-input bytes and digests were produced
//! by the implementation that canonicalized ops through a `BTreeMap`,
//! collected slots into `BTreeSet`s and fed `blake3::Hasher` field by field
//! (main at 0faaf58a). The vectors cover every op variant, every slot kind,
//! every attachment owner and plane, non-empty and empty payloads, a payload
//! longer than one BLAKE3 chunk, and repeated keys of every op variant.
use super::*;

use bytes::Bytes;

use crate::attachment::AttachmentPlane;
use crate::footprint::OutgoingEdgeRange;
use crate::ident::{make_edge_id, make_node_id, make_type_id, make_warp_id, TypeId};

// ---- fixture: identical in the committed test and the baseline capture ----

/// One patch's inputs, as a caller hands them to [`WarpTickPatchV1::new`]:
/// unsorted, with repeats.
struct Vector {
    name: &'static str,
    policy_id: u32,
    rule_pack_id: ContentHash,
    status: TickCommitStatus,
    in_slots: Vec<SlotId>,
    out_slots: Vec<SlotId>,
    ops: Vec<WarpOp>,
}

fn root() -> WarpId {
    make_warp_id("cv/root")
}

fn warp(i: u32) -> WarpId {
    make_warp_id(&format!("cv/warp/{i}"))
}

fn node(i: u32) -> NodeKey {
    NodeKey {
        warp_id: root(),
        local_id: make_node_id(&format!("cv/node/{i}")),
    }
}

fn edge(i: u32) -> EdgeKey {
    EdgeKey {
        warp_id: root(),
        local_id: make_edge_id(&format!("cv/edge/{i}")),
    }
}

fn ty(label: &str) -> TypeId {
    make_type_id(&format!("cv/type/{label}"))
}

fn node_key(i: u32, plane: AttachmentPlane) -> AttachmentKey {
    AttachmentKey {
        owner: AttachmentOwner::Node(node(i)),
        plane,
    }
}

fn edge_key(i: u32, plane: AttachmentPlane) -> AttachmentKey {
    AttachmentKey {
        owner: AttachmentOwner::Edge(edge(i)),
        plane,
    }
}

/// An atom whose payload is `len` bytes of a fixed pattern.
fn atom(label: &str, len: usize, seed: u8) -> AttachmentValue {
    let bytes: Vec<u8> = (0..=u8::MAX)
        .cycle()
        .take(len)
        .map(|i| i.wrapping_mul(31).wrapping_add(seed))
        .collect();
    AttachmentValue::Atom(AtomPayload::new(ty(label), Bytes::from(bytes)))
}

fn range(i: u32, label: &str) -> SlotId {
    SlotId::OutgoingEdges(OutgoingEdgeRange {
        source: node(i),
        edge_type: ty(label),
    })
}

fn edge_record(id: u32, from: u32, to: u32, label: &str) -> EdgeRecord {
    EdgeRecord {
        id: edge(id).local_id,
        from: node(from).local_id,
        to: node(to).local_id,
        ty: ty(label),
    }
}

fn set(key: AttachmentKey, value: Option<AttachmentValue>) -> WarpOp {
    WarpOp::SetAttachment { key, value }
}

fn upsert_node(i: u32, label: &str) -> WarpOp {
    WarpOp::UpsertNode {
        node: node(i),
        record: NodeRecord { ty: ty(label) },
    }
}

fn upsert_edge(id: u32, from: u32, to: u32, label: &str) -> WarpOp {
    WarpOp::UpsertEdge {
        warp_id: root(),
        record: edge_record(id, from, to, label),
    }
}

fn delete_edge(from: u32, id: u32) -> WarpOp {
    WarpOp::DeleteEdge {
        warp_id: root(),
        from: node(from).local_id,
        edge_id: edge(id).local_id,
    }
}

fn open_portal(key: AttachmentKey, child: u32, child_root: &str, init: PortalInit) -> WarpOp {
    WarpOp::OpenPortal {
        key,
        child_warp: warp(child),
        child_root: make_node_id(&format!("cv/child-root/{child_root}")),
        init,
    }
}

fn upsert_instance(i: u32, root_node: &str, parent: Option<AttachmentKey>) -> WarpOp {
    WarpOp::UpsertWarpInstance {
        instance: WarpInstance {
            warp_id: warp(i),
            root_node: make_node_id(&format!("cv/child-root/{root_node}")),
            parent,
        },
    }
}

fn every_kind() -> Vector {
    use AttachmentPlane::{Alpha, Beta};
    let in_slots = vec![
        range(0, "link"),
        SlotId::Port((root(), 7)),
        SlotId::Attachment(edge_key(0, Beta)),
        SlotId::Node(node(1)),
        SlotId::Attachment(node_key(0, Alpha)),
        SlotId::Edge(edge(1)),
        SlotId::Attachment(node_key(1, Beta)),
        SlotId::Attachment(edge_key(1, Alpha)),
        SlotId::Port((warp(1), u64::MAX)),
        SlotId::Node(node(0)),
        SlotId::Edge(edge(0)),
        SlotId::Node(node(1)),
        SlotId::Port((root(), 7)),
        range(1, "other"),
        range(0, "link"),
    ];
    let out_slots = vec![
        SlotId::Attachment(edge_key(0, Beta)),
        SlotId::Node(node(0)),
        range(0, "link"),
        SlotId::Port((root(), 7)),
        SlotId::Edge(edge(1)),
        SlotId::Attachment(node_key(0, Alpha)),
        SlotId::Node(node(0)),
    ];
    let ops = vec![
        // A payload over one BLAKE3 chunk (1 KiB) on its own.
        set(edge_key(0, Beta), Some(atom("blob", 1100, 3))),
        upsert_edge(1, 0, 1, "link"),
        WarpOp::DeleteNode { node: node(2) },
        open_portal(
            node_key(0, Beta),
            1,
            "a",
            PortalInit::Empty {
                root_record: NodeRecord { ty: ty("child") },
            },
        ),
        set(node_key(1, Alpha), Some(AttachmentValue::Descend(warp(1)))),
        upsert_instance(2, "b", Some(node_key(3, Alpha))),
        delete_edge(0, 2),
        upsert_node(1, "body"),
        open_portal(edge_key(1, Alpha), 3, "c", PortalInit::RequireExisting),
        set(node_key(0, Alpha), None),
        WarpOp::DeleteWarpInstance { warp_id: warp(4) },
        upsert_instance(5, "d", None),
        set(edge_key(1, Beta), Some(atom("small", 5, 9))),
        set(node_key(2, Alpha), Some(atom("empty", 0, 0))),
    ];
    Vector {
        name: "every-kind",
        policy_id: 0x0102_0304,
        rule_pack_id: *blake3::hash(b"cv/rule-pack/every-kind").as_bytes(),
        status: TickCommitStatus::Committed,
        in_slots,
        out_slots,
        ops,
    }
}

fn last_wins() -> Vector {
    use AttachmentPlane::{Alpha, Beta};
    let ops = vec![
        upsert_node(1, "first"),
        set(node_key(0, Alpha), Some(atom("x", 3, 1))),
        upsert_edge(1, 0, 1, "link"),
        open_portal(
            node_key(0, Beta),
            1,
            "a",
            PortalInit::Empty {
                root_record: NodeRecord { ty: ty("child") },
            },
        ),
        upsert_instance(2, "a", None),
        WarpOp::DeleteNode { node: node(2) },
        delete_edge(0, 2),
        WarpOp::DeleteWarpInstance { warp_id: warp(4) },
        // Same keys again, with different values where the op carries one.
        upsert_node(1, "second"),
        set(node_key(0, Alpha), None),
        upsert_edge(1, 0, 2, "link-2"),
        open_portal(node_key(0, Beta), 3, "b", PortalInit::RequireExisting),
        upsert_instance(2, "b", Some(edge_key(0, Beta))),
        WarpOp::DeleteNode { node: node(2) },
        delete_edge(0, 2),
        WarpOp::DeleteWarpInstance { warp_id: warp(4) },
        // Same edge id from another source: a different key, so both stay.
        upsert_edge(1, 3, 2, "link-3"),
        // Different kinds on one target: different keys, so all stay.
        WarpOp::DeleteNode { node: node(1) },
        set(node_key(0, Beta), Some(atom("beside-portal", 2, 4))),
        // Third writes; the last one of each key wins.
        set(node_key(0, Alpha), Some(atom("y", 40, 2))),
        upsert_node(1, "third"),
        upsert_edge(1, 0, 3, "link-4"),
        set(edge_key(0, Beta), Some(atom("edge", 9, 5))),
        set(edge_key(0, Beta), Some(AttachmentValue::Descend(warp(6)))),
    ];
    let in_slots = vec![
        SlotId::Node(node(1)),
        SlotId::Attachment(node_key(0, Alpha)),
        SlotId::Node(node(1)),
        range(0, "link"),
        SlotId::Attachment(node_key(0, Alpha)),
        SlotId::Edge(edge(1)),
        range(0, "link"),
        SlotId::Node(node(1)),
    ];
    let out_slots = vec![
        SlotId::Attachment(node_key(0, Alpha)),
        SlotId::Node(node(1)),
        SlotId::Attachment(node_key(0, Alpha)),
        SlotId::Node(node(1)),
    ];
    Vector {
        name: "last-wins",
        policy_id: 7,
        rule_pack_id: *blake3::hash(b"cv/rule-pack/last-wins").as_bytes(),
        status: TickCommitStatus::Aborted,
        in_slots,
        out_slots,
        ops,
    }
}

fn slots_only() -> Vector {
    use AttachmentPlane::{Alpha, Beta};
    let pool = [
        SlotId::Port((warp(2), 0)),
        SlotId::Attachment(edge_key(3, Alpha)),
        range(2, "z"),
        SlotId::Edge(edge(4)),
        SlotId::Attachment(node_key(4, Beta)),
        SlotId::Node(node(5)),
        range(2, "a"),
        SlotId::Port((warp(2), 1)),
    ];
    let in_slots = (0..24).map(|i| pool[(i * 5 + 3) % pool.len()]).collect();
    let out_slots = (0..9).map(|i| pool[(i * 3 + 1) % pool.len()]).collect();
    Vector {
        name: "slots-only",
        policy_id: u32::MAX,
        rule_pack_id: [0xa5; 32],
        status: TickCommitStatus::Committed,
        in_slots,
        out_slots,
        ops: Vec::new(),
    }
}

fn small(r: u64) -> u32 {
    u32::try_from(r).unwrap_or(0)
}

/// 400 ops over a small key space, so keys repeat in a varied order, with
/// 300 slots drawn from a small pool.
fn generated() -> Vector {
    use AttachmentPlane::{Alpha, Beta};
    let mut x: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = move || {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        x >> 24
    };
    let mut ops = Vec::new();
    for _ in 0..400 {
        let r = next();
        let a = small(r % 6);
        let b = small((r >> 8) % 4);
        let plane = if (r >> 12) % 2 == 0 { Alpha } else { Beta };
        let len = usize::try_from((r >> 16) % 70).unwrap_or(0);
        let seed = (r >> 24).to_le_bytes()[0];
        ops.push(match (r >> 32) % 9 {
            0 => set(node_key(a, plane), Some(atom("g", len, seed))),
            1 => set(edge_key(b, plane), Some(atom("h", len, seed))),
            2 => set(node_key(a, plane), None),
            3 => upsert_node(a, if seed % 2 == 0 { "even" } else { "odd" }),
            4 => WarpOp::DeleteNode { node: node(a) },
            5 => upsert_edge(b, a % 2, a, "g"),
            6 => delete_edge(a % 2, b),
            7 => set(node_key(a, plane), Some(AttachmentValue::Descend(warp(b)))),
            _ => upsert_instance(b, if seed % 2 == 0 { "e" } else { "o" }, None),
        });
    }
    let mut slot = || {
        let r = next();
        let a = small(r % 6);
        match (r >> 8) % 5 {
            0 => SlotId::Node(node(a)),
            1 => SlotId::Edge(edge(a % 4)),
            2 => SlotId::Attachment(node_key(a, Alpha)),
            3 => SlotId::Port((root(), u64::from(a))),
            _ => range(a, "g"),
        }
    };
    let in_slots = (0..200).map(|_| slot()).collect();
    let out_slots = (0..100).map(|_| slot()).collect();
    Vector {
        name: "generated",
        policy_id: 42,
        rule_pack_id: *blake3::hash(b"cv/rule-pack/generated").as_bytes(),
        status: TickCommitStatus::Committed,
        in_slots,
        out_slots,
        ops,
    }
}

fn vectors() -> [Vector; 4] {
    [every_kind(), last_wins(), slots_only(), generated()]
}

fn build(v: &Vector) -> WarpTickPatchV1 {
    WarpTickPatchV1::new(
        v.policy_id,
        v.rule_pack_id,
        v.status,
        v.in_slots.clone(),
        v.out_slots.clone(),
        v.ops.clone(),
    )
}

/// Where each canonical op came from: the index of the last input op equal
/// to it. Last-wins keeps the last op of each key, so this names the survivor.
fn op_sources(v: &Vector, patch: &WarpTickPatchV1) -> Vec<usize> {
    patch
        .ops()
        .iter()
        .map(|op| {
            v.ops
                .iter()
                .rposition(|input| input == op)
                .unwrap_or(usize::MAX)
        })
        .collect()
}

/// Each canonical slot as the index of its first occurrence in the input.
fn slot_sources(input: &[SlotId], canonical: &[SlotId]) -> Vec<usize> {
    canonical
        .iter()
        .map(|slot| input.iter().position(|s| s == slot).unwrap_or(usize::MAX))
        .collect()
}

// ---- end fixture ----

// ---- pinned baseline values ----
//
// Produced by the map-based implementation this module's parent replaced:
// main at 0faaf58a, which canonicalized ops through a `BTreeMap` and fed
// `blake3::Hasher` field by field. See the commit that added this module for
// how each value was captured.

struct Expected {
    name: &'static str,
    digest: &'static str,
    input_len: usize,
    /// The full digest input, for the vectors small enough to pin in hex.
    input: Option<&'static str>,
    in_slots: &'static [usize],
    out_slots: &'static [usize],
    ops: &'static [usize],
}

const EXPECTED: [Expected; 4] = [
    Expected {
        name: "every-kind",
        digest: "595c11495cbda213c2ae7d66b118b0c29fb5dfea7b1f54b82bb2baae55361f8d",
        input_len: 3838,
        input: Some(
            "6563686f3a70617463685f6469676573743a76310002000403020118bed28c92e7d37b8bcdb1fe22fc110d0f\
             cf9f1b1d6740b29a299292db61e32d010c00000000000000017cfd0ecd52438094cebc57213a8088cdd025cc\
             04e5ebe04786094e87b8e5a1e8c7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f636b603a61\
             a4017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc\
             3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109a027cfd0ecd52438094cebc57213a8088cdd025cc04e5\
             ebe04786094e87b8e5a1e8802dbb84cf8ee62b5f6d2363258f3c3a9d694218006bd7edfad04595c22cf7c302\
             7cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8dd323370f109585c7159764b\
             9101af942710b689e5740356bbf81ddeff2c847c0301027cfd0ecd52438094cebc57213a8088cdd025cc04e5\
             ebe04786094e87b8e5a1e8c7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f636b603a61a403\
             01017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc\
             3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109a0302027cfd0ecd52438094cebc57213a8088cdd025cc\
             04e5ebe04786094e87b8e5a1e8802dbb84cf8ee62b5f6d2363258f3c3a9d694218006bd7edfad04595c22cf7\
             c30302017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8dd323370f109585c\
             7159764b9101af942710b689e5740356bbf81ddeff2c847c0402795db50eb71e7272a99f80b971caa9854542\
             ea3cc809cb60a83c13f1172984ffffffffffffffff047cfd0ecd52438094cebc57213a8088cdd025cc04e5eb\
             e04786094e87b8e5a1e80700000000000000057cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786\
             094e87b8e5a1e8c7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f636b603a61a49bc196fc65\
             cd931dd1e2ac56e25b459cdf6134d24b31312fbb3d09040f57bfd1057cfd0ecd52438094cebc57213a8088cd\
             d025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb\
             848d109aa68820c27384e15676f3c6959602423ed2da55c424c4750d1cf69c285c9105430600000000000000\
             017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3e\
             a15ae1a85936508ad2e399c25bf4b0bcbb848d109a027cfd0ecd52438094cebc57213a8088cdd025cc04e5eb\
             e04786094e87b8e5a1e8dd323370f109585c7159764b9101af942710b689e5740356bbf81ddeff2c847c0301\
             017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3e\
             a15ae1a85936508ad2e399c25bf4b0bcbb848d109a0302027cfd0ecd52438094cebc57213a8088cdd025cc04\
             e5ebe04786094e87b8e5a1e8802dbb84cf8ee62b5f6d2363258f3c3a9d694218006bd7edfad04595c22cf7c3\
             047cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e80700000000000000057cfd\
             0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1\
             a85936508ad2e399c25bf4b0bcbb848d109aa68820c27384e15676f3c6959602423ed2da55c424c4750d1cf6\
             9c285c9105430e000000000000000801027cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e\
             87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109a02795db50eb71e\
             7272a99f80b971caa9854542ea3cc809cb60a83c13f1172984d72f7063a2bf3406562fac2af6ba1c1497d3e4\
             839aa95f780f1890045b5af08e01b05a0e3c4f2ba9eae3701b9844368830e329c827811fc86d77937e902753\
             539e0802017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8dd323370f10958\
             5c7159764b9101af942710b689e5740356bbf81ddeff2c847c33f6a8278dd1a5c9f38cb39a5abd058f837b33\
             6a6b9236bfa9266ee05e84ff3343a59cc753d34e675afafe6fcb3c9e95262d65754302c9ec96a6e5141de16c\
             380001118d5a736470be5afee95736817acdc9688fa0c0947b5337c078d39f1048e3e3798076ba5e6ebd35b3\
             2006ed8784867df776ac6ca6ec55ec466a2d201bb0f45200013ecd35811337548b187bc717ef73da35105f72\
             8d8c64fc5007b319addcb54875dc47c911f83a8b37f7cc551706fa1fb4000eb7d1719fa7b76c2f7cb672300c\
             280101017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e89af1c9b95e3bf77c\
             38b0e531f9a7190cac7fab0edc0ee751ed9e92903280010f028518043b559aa886db195aa75323be9ba10d99\
             e67b26fff0f0a92c0f6bfc115a067cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5\
             a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109a924b6a204cb6db8d9410\
             9014181a37206fcbe20ef07779fdbaecc1f32be8584e047cfd0ecd52438094cebc57213a8088cdd025cc04e5\
             ebe04786094e87b8e5a1e88c2876d3f6c70d034b3780594ca324ca1e722e2ec662e2298285c4815024ddad03\
             7cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8c7031e935da5d0d07397f788\
             c983158b593e4888a29eecb18e9f636b603a61a4422a5d807a4046a1b3b79c5f121794de54d24ef6ed34d86d\
             393ddf8ece658d63057cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d3\
             10ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109add323370f109585c7159764b9101af\
             942710b689e5740356bbf81ddeff2c847cc7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f63\
             6b603a61a4a68820c27384e15676f3c6959602423ed2da55c424c4750d1cf69c285c9105430701017cfd0ecd\
             52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e88c2876d3f6c70d034b3780594ca324ca\
             1e722e2ec662e2298285c4815024ddad0101624ff5959be8abdad1f66c3cd0556ec652d007ba6e5902044222\
             ef0d7cafe17e00000000000000000701017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e\
             87b8e5a1e8c7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f636b603a61a4010202795db50e\
             b71e7272a99f80b971caa9854542ea3cc809cb60a83c13f11729840701017cfd0ecd52438094cebc57213a80\
             88cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0\
             bcbb848d109a000702027cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8802d\
             bb84cf8ee62b5f6d2363258f3c3a9d694218006bd7edfad04595c22cf7c30101593b294b89a9d82d3266c4f5\
             42d1bb3b68aed67782409154eb168c2796bee8594c04000000000000032241607f9ebddcfb1a39587796b5d4\
             f31231506f8eadcceb0a29486786a5c4e30221405f7e9dbcdbfa1938577695b4d3f211304f6e8daccbea0928\
             476685a4c3e201203f5e7d9cbbdaf91837567594b3d2f1102f4e6d8cabcae90827466584a3c2e1001f3e5d7c\
             9bbad9f81736557493b2d1f00f2e4d6c8baac9e80726456483a2c1e0ff1e3d5c7b9ab9d8f71635547392b1d0\
             ef0e2d4c6b8aa9c8e70625446382a1c0dffe1d3c5b7a99b8d7f61534537291b0cfee0d2c4b6a89a8c7e60524\
             436281a0bfdefd1c3b5a7998b7d6f51433527190afceed0c2b4a6988a7c6e504234261809fbeddfc1b3a5978\
             97b6d5f4133251708faecdec0b2a496887a6c5e4032241607f9ebddcfb1a39587796b5d4f31231506f8eadcc\
             eb0a29486786a5c4e30221405f7e9dbcdbfa1938577695b4d3f211304f6e8daccbea0928476685a4c3e20120\
             3f5e7d9cbbdaf91837567594b3d2f1102f4e6d8cabcae90827466584a3c2e1001f3e5d7c9bbad9f817365574\
             93b2d1f00f2e4d6c8baac9e80726456483a2c1e0ff1e3d5c7b9ab9d8f71635547392b1d0ef0e2d4c6b8aa9c8\
             e70625446382a1c0dffe1d3c5b7a99b8d7f61534537291b0cfee0d2c4b6a89a8c7e60524436281a0bfdefd1c\
             3b5a7998b7d6f51433527190afceed0c2b4a6988a7c6e504234261809fbeddfc1b3a597897b6d5f413325170\
             8faecdec0b2a496887a6c5e4032241607f9ebddcfb1a39587796b5d4f31231506f8eadcceb0a29486786a5c4\
             e30221405f7e9dbcdbfa1938577695b4d3f211304f6e8daccbea0928476685a4c3e201203f5e7d9cbbdaf918\
             37567594b3d2f1102f4e6d8cabcae90827466584a3c2e1001f3e5d7c9bbad9f81736557493b2d1f00f2e4d6c\
             8baac9e80726456483a2c1e0ff1e3d5c7b9ab9d8f71635547392b1d0ef0e2d4c6b8aa9c8e70625446382a1c0\
             dffe1d3c5b7a99b8d7f61534537291b0cfee0d2c4b6a89a8c7e60524436281a0bfdefd1c3b5a7998b7d6f514\
             33527190afceed0c2b4a6988a7c6e504234261809fbeddfc1b3a597897b6d5f4133251708faecdec0b2a4968\
             87a6c5e4032241607f9ebddcfb1a39587796b5d4f31231506f8eadcceb0a29486786a5c4e30221405f7e9dbc\
             dbfa1938577695b4d3f211304f6e8daccbea0928476685a4c3e201203f5e7d9cbbdaf91837567594b3d2f110\
             2f4e6d8cabcae90827466584a3c2e1001f3e5d7c9bbad9f81736557493b2d1f00f2e4d6c8baac9e807264564\
             83a2c1e0ff1e3d5c7b9ab9d8f71635547392b1d0ef0e2d4c6b8aa9c8e70625446382a1c0dffe1d3c5b7a99b8\
             d7f61534537291b0cfee0d2c4b6a89a8c7e60524436281a0bfdefd1c3b5a7998b7d6f51433527190afceed0c\
             2b4a6988a7c6e504234261809fbeddfc1b3a597897b6d5f4133251708faecdec0b2a496887a6c5e403224160\
             7f9ebddcfb1a39587796b5d4f31231506f8eadcceb0a29486786a5c4e30221405f7e9dbcdbfa1938577695b4\
             d3f211304f6e8daccbea0928476685a4c3e201203f5e7d9cbbdaf9180702027cfd0ecd52438094cebc57213a\
             8088cdd025cc04e5ebe04786094e87b8e5a1e8dd323370f109585c7159764b9101af942710b689e5740356bb\
             f81ddeff2c847c0101d7c9c1633359aa670e100631e5a07c776ba83fd72f0c927bba00824cad4271c8050000\
             00000000000928476685",
        ),
        in_slots: &[3, 9, 10, 5, 6, 4, 2, 7, 8, 1, 13, 0],
        out_slots: &[1, 4, 5, 0, 3, 2],
        ops: &[3, 8, 11, 5, 10, 6, 2, 7, 1, 13, 4, 9, 0, 12],
    },
    Expected {
        name: "last-wins",
        digest: "a7e9f7a84bbc9ec49bf1934d21bf4b7a76e473a69e6fb778189e6f5d9ccf46ac",
        input_len: 1814,
        input: Some(
            "6563686f3a70617463685f6469676573743a7631000200070000002df57616db48b7d383847627da0a203c5a\
             00ffdc3e20348c4225a19fb9af806d020400000000000000017cfd0ecd52438094cebc57213a8088cdd025cc\
             04e5ebe04786094e87b8e5a1e8c7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f636b603a61\
             a4027cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8dd323370f109585c7159\
             764b9101af942710b689e5740356bbf81ddeff2c847c0301017cfd0ecd52438094cebc57213a8088cdd025cc\
             04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d10\
             9a057cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc\
             3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109aa68820c27384e15676f3c6959602423ed2da55c424c4\
             750d1cf69c285c9105430200000000000000017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786\
             094e87b8e5a1e8c7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f636b603a61a40301017cfd\
             0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1\
             a85936508ad2e399c25bf4b0bcbb848d109a0c000000000000000801027cfd0ecd52438094cebc57213a8088\
             cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bc\
             bb848d109a33f6a8278dd1a5c9f38cb39a5abd058f837b336a6b9236bfa9266ee05e84ff33dc47c911f83a8b\
             37f7cc551706fa1fb4000eb7d1719fa7b76c2f7cb672300c2800013ecd35811337548b187bc717ef73da3510\
             5f728d8c64fc5007b319addcb54875dc47c911f83a8b37f7cc551706fa1fb4000eb7d1719fa7b76c2f7cb672\
             300c280102027cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8802dbb84cf8e\
             e62b5f6d2363258f3c3a9d694218006bd7edfad04595c22cf7c3028518043b559aa886db195aa75323be9ba1\
             0d99e67b26fff0f0a92c0f6bfc115a067cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87\
             b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109a924b6a204cb6db8d\
             94109014181a37206fcbe20ef07779fdbaecc1f32be8584e047cfd0ecd52438094cebc57213a8088cdd025cc\
             04e5ebe04786094e87b8e5a1e88c2876d3f6c70d034b3780594ca324ca1e722e2ec662e2298285c4815024dd\
             ad047cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8c7031e935da5d0d07397\
             f788c983158b593e4888a29eecb18e9f636b603a61a4037cfd0ecd52438094cebc57213a8088cdd025cc04e5\
             ebe04786094e87b8e5a1e8c7031e935da5d0d07397f788c983158b593e4888a29eecb18e9f636b603a61a484\
             e39c444a71ee7de44499c33a9fd70922dbf78a4ddf1b9a60962baf16294b08057cfd0ecd52438094cebc5721\
             3a8088cdd025cc04e5ebe04786094e87b8e5a1e89af1c9b95e3bf77c38b0e531f9a7190cac7fab0edc0ee751\
             ed9e92903280010fdd323370f109585c7159764b9101af942710b689e5740356bbf81ddeff2c847c8c2876d3\
             f6c70d034b3780594ca324ca1e722e2ec662e2298285c4815024ddad56b636c1be8ec2bad5b43286e049e828\
             a7789830285e523b20b86e708776b5fc057cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e\
             87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109add323370f10958\
             5c7159764b9101af942710b689e5740356bbf81ddeff2c847c9af1c9b95e3bf77c38b0e531f9a7190cac7fab\
             0edc0ee751ed9e92903280010f2cf6c61b974de434a9bed84b98f7e162e29e82927b69ed655586fff83f762e\
             e20701017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8cb34d310ccbe0e32\
             3abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109a0101c1dca8679dc258dcba6c4b8c1cbcbd53f46f\
             79eee298e628d4ecc6c90585593928000000000000000221405f7e9dbcdbfa1938577695b4d3f211304f6e8d\
             accbea0928476685a4c3e201203f5e7d9cbb0701027cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe0\
             4786094e87b8e5a1e8cb34d310ccbe0e323abc3ea15ae1a85936508ad2e399c25bf4b0bcbb848d109a0101fa\
             2467432f8991507761023bda45f1c15f564adf00d49301c3303899eda23a6b02000000000000000423070202\
             7cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8802dbb84cf8ee62b5f6d2363\
             258f3c3a9d694218006bd7edfad04595c22cf7c30102d226b66ead06217b86091a64eb029ac191cdaa714723\
             acbf7ea35ffafd383e6a",
        ),
        in_slots: &[0, 5, 1, 3],
        out_slots: &[1, 0],
        ops: &[11, 12, 15, 14, 13, 17, 20, 16, 21, 19, 18, 23],
    },
    Expected {
        name: "slots-only",
        digest: "fac1598ca120c8a9c82b54b5179105449635e067d22bb74df8fc5749fde081fb",
        input_len: 1164,
        input: Some(
            "6563686f3a70617463685f6469676573743a7631000200ffffffffa5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5\
             a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5010800000000000000017cfd0ecd52438094cebc57213a8088cdd025cc\
             04e5ebe04786094e87b8e5a1e86643743695fe0625bf8fe1066f0251bdc4782dad4fd175230232e04edf780f\
             28027cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8c90cf1688c4ff9294b79\
             7047040280dd6e64591f564d4cbfab330ce5e205880b0301027cfd0ecd52438094cebc57213a8088cdd025cc\
             04e5ebe04786094e87b8e5a1e8541697e512bf77289a7db12aac5e98afe865acea3681eb1d095abdafc81397\
             ef0302017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e82b0c4993d5accd7d\
             703f954a4787ebde7f163a922bfb3bf7d32a1798252d6d18043ecd35811337548b187bc717ef73da35105f72\
             8d8c64fc5007b319addcb548750000000000000000043ecd35811337548b187bc717ef73da35105f728d8c64\
             fc5007b319addcb548750100000000000000057cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786\
             094e87b8e5a1e88c2876d3f6c70d034b3780594ca324ca1e722e2ec662e2298285c4815024ddad4181a4d86f\
             31a3bdd2a3bf966b723a108c86bf457af711a8bcc5fb1ca1d8ac25057cfd0ecd52438094cebc57213a8088cd\
             d025cc04e5ebe04786094e87b8e5a1e88c2876d3f6c70d034b3780594ca324ca1e722e2ec662e2298285c481\
             5024ddad72a4950faeb19e049ed341b5f7309174cc14fa6f0d3ce65f181de1e3a5e016c00800000000000000\
             017cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e86643743695fe0625bf8fe1\
             066f0251bdc4782dad4fd175230232e04edf780f28027cfd0ecd52438094cebc57213a8088cdd025cc04e5eb\
             e04786094e87b8e5a1e8c90cf1688c4ff9294b797047040280dd6e64591f564d4cbfab330ce5e205880b0301\
             027cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e8541697e512bf77289a7db1\
             2aac5e98afe865acea3681eb1d095abdafc81397ef0302017cfd0ecd52438094cebc57213a8088cdd025cc04\
             e5ebe04786094e87b8e5a1e82b0c4993d5accd7d703f954a4787ebde7f163a922bfb3bf7d32a1798252d6d18\
             043ecd35811337548b187bc717ef73da35105f728d8c64fc5007b319addcb548750000000000000000043ecd\
             35811337548b187bc717ef73da35105f728d8c64fc5007b319addcb548750100000000000000057cfd0ecd52\
             438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e88c2876d3f6c70d034b3780594ca324ca1e\
             722e2ec662e2298285c4815024ddad4181a4d86f31a3bdd2a3bf966b723a108c86bf457af711a8bcc5fb1ca1\
             d8ac25057cfd0ecd52438094cebc57213a8088cdd025cc04e5ebe04786094e87b8e5a1e88c2876d3f6c70d03\
             4b3780594ca324ca1e722e2ec662e2298285c4815024ddad72a4950faeb19e049ed341b5f7309174cc14fa6f\
             0d3ce65f181de1e3a5e016c00000000000000000",
        ),
        in_slots: &[2, 0, 5, 6, 1, 4, 3, 7],
        out_slots: &[4, 6, 1, 0, 5, 2, 3, 7],
        ops: &[],
    },
    Expected {
        name: "generated",
        digest: "875f62830875abe16e5e21d1ea01c3e28610d12607762f1da6239736ca0b1119",
        input_len: 9579,
        input: None,
        in_slots: &[26, 17, 6, 10, 45, 65, 4, 7, 19, 39, 22, 15, 61, 52, 0, 76, 5, 98, 1, 12, 8, 25, 43, 16, 11, 42, 2, 36],
        out_slots: &[6, 31, 10, 26, 4, 19, 16, 34, 74, 7, 9, 57, 39, 25, 0, 65, 38, 47, 51, 36, 11, 1, 5, 67, 3, 93, 63],
        ops: &[206, 386, 369, 391, 264, 290, 336, 308, 378, 249, 381, 343, 379, 397, 370, 322, 344, 237, 396, 394, 393, 384, 380, 305, 329, 289, 376, 363, 285, 390, 226, 348, 389, 399, 383, 382, 398, 392, 350, 262, 373, 385, 365, 328, 252, 395, 387, 297, 375, 307, 303, 337],
    },
];

fn digest_input(patch: &WarpTickPatchV1) -> Vec<u8> {
    patch_digest_input_v2(
        patch.policy_id(),
        &patch.rule_pack_id(),
        patch.commit_status(),
        patch.in_slots(),
        patch.out_slots(),
        patch.ops(),
    )
}

#[test]
fn patch_vectors_match_the_baseline_bytes_and_digests() {
    for (v, expected) in vectors().iter().zip(EXPECTED.iter()) {
        assert_eq!(v.name, expected.name);
        let patch = build(v);
        let input = digest_input(&patch);
        assert_eq!(input.len(), expected.input_len, "{}: input length", v.name);
        if let Some(hex) = expected.input {
            assert_eq!(hex::encode(&input), hex, "{}: digest input bytes", v.name);
        }
        assert_eq!(
            hex::encode(patch.digest()),
            expected.digest,
            "{}: patch digest",
            v.name
        );
        assert_eq!(blake3::hash(&input).as_bytes(), &patch.digest());
        assert!(patch.validate_digest().is_ok());
        assert_eq!(
            slot_sources(&v.in_slots, patch.in_slots()),
            expected.in_slots,
            "{}: canonical in_slots",
            v.name
        );
        assert_eq!(
            slot_sources(&v.out_slots, patch.out_slots()),
            expected.out_slots,
            "{}: canonical out_slots",
            v.name
        );
        assert_eq!(
            op_sources(v, &patch),
            expected.ops,
            "{}: canonical ops",
            v.name
        );
    }
}

/// The map and set canonicalizers this module's parent replaced, kept as an
/// independent oracle over every vector.
#[test]
fn patch_vectors_match_the_map_and_set_canonicalizers() {
    for v in vectors() {
        let patch = build(&v);
        let mut map = std::collections::BTreeMap::new();
        for op in v.ops.iter().cloned() {
            map.insert(op.sort_key(), op);
        }
        let ops: Vec<WarpOp> = map.into_values().collect();
        let in_slots: Vec<SlotId> = v
            .in_slots
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let out_slots: Vec<SlotId> = v
            .out_slots
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(patch.ops(), ops.as_slice(), "{}: ops", v.name);
        assert_eq!(
            patch.in_slots(),
            in_slots.as_slice(),
            "{}: in_slots",
            v.name
        );
        assert_eq!(
            patch.out_slots(),
            out_slots.as_slice(),
            "{}: out_slots",
            v.name
        );
    }
}

fn op_kind(op: &WarpOp) -> &'static str {
    match op {
        WarpOp::OpenPortal {
            init: PortalInit::Empty { .. },
            ..
        } => "open-portal/empty",
        WarpOp::OpenPortal {
            init: PortalInit::RequireExisting,
            ..
        } => "open-portal/require-existing",
        WarpOp::UpsertWarpInstance { instance } => match instance.parent {
            None => "upsert-instance/no-parent",
            Some(_) => "upsert-instance/parent",
        },
        WarpOp::DeleteWarpInstance { .. } => "delete-instance",
        WarpOp::UpsertNode { .. } => "upsert-node",
        WarpOp::DeleteNode { .. } => "delete-node",
        WarpOp::UpsertEdge { .. } => "upsert-edge",
        WarpOp::DeleteEdge { .. } => "delete-edge",
        WarpOp::SetAttachment { value: None, .. } => "set-attachment/clear",
        WarpOp::SetAttachment {
            value: Some(AttachmentValue::Atom(atom)),
            ..
        } => {
            if atom.bytes.is_empty() {
                "set-attachment/atom-empty"
            } else {
                "set-attachment/atom"
            }
        }
        WarpOp::SetAttachment {
            value: Some(AttachmentValue::Descend(_)),
            ..
        } => "set-attachment/descend",
    }
}

fn slot_kind(slot: &SlotId) -> &'static str {
    match slot {
        SlotId::Node(_) => "node",
        SlotId::Edge(_) => "edge",
        SlotId::Attachment(key) => key_kind(key),
        SlotId::Port(_) => "port",
        SlotId::OutgoingEdges(_) => "outgoing-edges",
    }
}

fn key_kind(key: &AttachmentKey) -> &'static str {
    match (key.owner, key.plane) {
        (AttachmentOwner::Node(_), AttachmentPlane::Alpha) => "attachment/node-alpha",
        (AttachmentOwner::Node(_), AttachmentPlane::Beta) => "attachment/node-beta",
        (AttachmentOwner::Edge(_), AttachmentPlane::Alpha) => "attachment/edge-alpha",
        (AttachmentOwner::Edge(_), AttachmentPlane::Beta) => "attachment/edge-beta",
    }
}

/// The vectors exercise every op variant (and each sub-variant the encoding
/// distinguishes), every slot kind and every attachment owner and plane. A
/// new variant fails to compile in the matches above until it is covered.
#[test]
fn patch_vectors_cover_every_op_and_slot_kind() {
    use std::collections::BTreeSet;
    let all = vectors();
    let ops: BTreeSet<&str> = all.iter().flat_map(|v| v.ops.iter().map(op_kind)).collect();
    assert_eq!(
        ops,
        BTreeSet::from([
            "open-portal/empty",
            "open-portal/require-existing",
            "upsert-instance/no-parent",
            "upsert-instance/parent",
            "delete-instance",
            "upsert-node",
            "delete-node",
            "upsert-edge",
            "delete-edge",
            "set-attachment/clear",
            "set-attachment/atom",
            "set-attachment/atom-empty",
            "set-attachment/descend",
        ])
    );
    let every_slot = BTreeSet::from([
        "node",
        "edge",
        "attachment/node-alpha",
        "attachment/node-beta",
        "attachment/edge-alpha",
        "attachment/edge-beta",
        "port",
        "outgoing-edges",
    ]);
    let every = &all[0];
    let in_kinds: BTreeSet<&str> = every.in_slots.iter().map(slot_kind).collect();
    assert_eq!(in_kinds, every_slot, "every-kind in_slots");
    let out_kinds: BTreeSet<&str> = every.out_slots.iter().map(slot_kind).collect();
    for kind in ["node", "edge", "port", "outgoing-edges"] {
        assert!(
            out_kinds.contains(kind),
            "every-kind out_slots lacks {kind}"
        );
    }
    // Each op variant repeats a key in the last-wins vector.
    let last_wins = &all[1];
    let mut seen = BTreeSet::new();
    let mut repeated = BTreeSet::new();
    for op in &last_wins.ops {
        if !seen.insert(op.sort_key()) {
            repeated.insert(op_kind(op).split('/').next().unwrap_or_default());
        }
    }
    assert_eq!(
        repeated,
        BTreeSet::from([
            "open-portal",
            "upsert-instance",
            "delete-instance",
            "upsert-node",
            "delete-node",
            "upsert-edge",
            "delete-edge",
            "set-attachment",
        ])
    );
}
