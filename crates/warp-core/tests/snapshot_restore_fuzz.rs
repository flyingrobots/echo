// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::cast_possible_truncation
)]
//! Snapshot/restore fuzz harness for M004.
//!
//! The harness snapshots a deterministic worldline at pseudo-random ticks,
//! restores the materialized graph from canonical WSC bytes, replays the suffix
//! from recorded provenance, and compares the final state root with the
//! uninterrupted run. JSON output is rendered by hand; it is a diagnostic test
//! artifact, not a causal encoding boundary.

mod common;

use std::fmt::Write as _;

use bytes::Bytes;
use common::{append_fixture_entry, hex32, register_fixture_worldline, test_warp_id, XorShift64};
use warp_core::wsc::types::{AttRow, WarpDirEntry, WscHeader};
use warp_core::wsc::{build_one_warp_input, validate_wsc, write_wsc_one_warp, WarpView, WscFile};
use warp_core::{
    compute_commit_hash_v2, make_edge_id, make_node_id, make_type_id, AtomPayload, AttachmentKey,
    AttachmentValue, EdgeId, EdgeRecord, GlobalTick, GraphStore, Hash, HashTriplet,
    LocalProvenanceStore, NodeId, NodeKey, NodeRecord, ProvenanceStore, TickCommitStatus, TypeId,
    WarpId, WarpOp, WarpTickPatchV1, WorldlineId, WorldlineState, WorldlineTick,
    WorldlineTickHeaderV1, WorldlineTickPatchV1,
};

const TOTAL_TICKS: u64 = 500;
const FUZZ_ITERATIONS: usize = 50;
const SNAPSHOT_SCHEMA_HASH: Hash = [0x5A; 32];

type TestResult<T> = Result<T, String>;

#[derive(Clone, Copy, Debug)]
enum SnapshotFormat {
    CanonicalWscV1,
}

impl SnapshotFormat {
    fn name(self) -> &'static str {
        match self {
            Self::CanonicalWscV1 => "canonical_wsc_v1",
        }
    }
}

struct Simulation {
    provenance: LocalProvenanceStore,
    warp_id: WarpId,
    worldline_id: WorldlineId,
    expected_roots: Vec<Hash>,
    states_by_tick: Vec<WorldlineState>,
}

#[derive(Clone)]
struct SnapshotRestoreIteration {
    iteration: usize,
    format: SnapshotFormat,
    snapshot_tick: u64,
    restore_tick: u64,
    comparison_tick: u64,
    restored_state_root: Hash,
    expected_state_root: Hash,
    actual_state_root: Hash,
}

impl SnapshotRestoreIteration {
    fn matches(&self) -> bool {
        self.expected_state_root == self.actual_state_root
    }
}

struct SnapshotRestoreReport {
    simulation_ticks: u64,
    iterations: Vec<SnapshotRestoreIteration>,
}

impl SnapshotRestoreReport {
    fn divergence_count(&self) -> usize {
        self.iterations
            .iter()
            .filter(|iteration| !iteration.matches())
            .count()
    }

    fn to_json(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{{");
        let _ = writeln!(out, "  \"simulation_ticks\": {},", self.simulation_ticks);
        let _ = writeln!(out, "  \"iteration_count\": {},", self.iterations.len());
        let _ = writeln!(out, "  \"divergence_count\": {},", self.divergence_count());
        let _ = writeln!(out, "  \"iterations\": [");
        for (index, iteration) in self.iterations.iter().enumerate() {
            let comma = if index + 1 == self.iterations.len() {
                ""
            } else {
                ","
            };
            let _ = writeln!(out, "    {{");
            let _ = writeln!(out, "      \"iteration\": {},", iteration.iteration);
            let _ = writeln!(out, "      \"format\": \"{}\",", iteration.format.name());
            let _ = writeln!(out, "      \"snapshot_tick\": {},", iteration.snapshot_tick);
            let _ = writeln!(out, "      \"restore_tick\": {},", iteration.restore_tick);
            let _ = writeln!(
                out,
                "      \"comparison_tick\": {},",
                iteration.comparison_tick
            );
            let _ = writeln!(
                out,
                "      \"restored_state_root\": \"{}\",",
                hex32(&iteration.restored_state_root)
            );
            let _ = writeln!(
                out,
                "      \"expected_state_root\": \"{}\",",
                hex32(&iteration.expected_state_root)
            );
            let _ = writeln!(
                out,
                "      \"actual_state_root\": \"{}\",",
                hex32(&iteration.actual_state_root)
            );
            let _ = writeln!(out, "      \"match\": {}", iteration.matches());
            let _ = writeln!(out, "    }}{comma}");
        }
        let _ = writeln!(out, "  ]");
        let _ = writeln!(out, "}}");
        out
    }
}

fn wt(raw: u64) -> WorldlineTick {
    WorldlineTick::from_raw(raw)
}

fn snapshot_worldline_id() -> WorldlineId {
    WorldlineId::from_bytes([4u8; 32])
}

fn root_key(warp_id: WarpId) -> NodeKey {
    NodeKey {
        warp_id,
        local_id: make_node_id("root"),
    }
}

fn snapshot_fuzz_patch(warp_id: WarpId, tick: u64) -> WorldlineTickPatchV1 {
    let root = root_key(warp_id);
    let child = make_node_id(&format!("snapshot-fuzz/node-{tick}"));
    let edge = make_edge_id(&format!("snapshot-fuzz/edge-{tick}"));
    let edge_ty = make_type_id(&format!("snapshot-fuzz/link-{}", tick % 7));
    let child_ty = make_type_id(&format!("snapshot-fuzz/child-{}", tick % 11));
    let root_ty = make_type_id(&format!("snapshot-fuzz/root-{tick}"));
    let attachment_ty = make_type_id("snapshot-fuzz/root-marker");

    let mut marker_bytes = Vec::with_capacity(16);
    marker_bytes.extend_from_slice(&tick.to_le_bytes());
    marker_bytes.extend_from_slice(&tick.rotate_left(17).to_le_bytes());
    let marker = AtomPayload::new(attachment_ty, Bytes::from(marker_bytes));

    let ops = vec![
        WarpOp::UpsertNode {
            node: root,
            record: NodeRecord { ty: root_ty },
        },
        WarpOp::UpsertNode {
            node: NodeKey {
                warp_id,
                local_id: child,
            },
            record: NodeRecord { ty: child_ty },
        },
        WarpOp::UpsertEdge {
            warp_id,
            record: EdgeRecord {
                id: edge,
                from: root.local_id,
                to: child,
                ty: edge_ty,
            },
        },
        WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(root),
            value: Some(AttachmentValue::Atom(marker)),
        },
    ];

    let header = WorldlineTickHeaderV1 {
        commit_global_tick: GlobalTick::from_raw(tick + 1),
        policy_id: 0,
        rule_pack_id: [0u8; 32],
        plan_digest: [0u8; 32],
        decision_digest: [0u8; 32],
        rewrites_digest: [0u8; 32],
    };
    let patch_digest = WarpTickPatchV1::new(
        header.policy_id,
        header.rule_pack_id,
        TickCommitStatus::Committed,
        Vec::new(),
        Vec::new(),
        ops.clone(),
    )
    .digest();

    WorldlineTickPatchV1 {
        header,
        warp_id,
        ops,
        in_slots: Vec::new(),
        out_slots: Vec::new(),
        patch_digest,
    }
}

fn build_simulation(total_ticks: u64) -> Simulation {
    let warp_id = test_warp_id();
    let worldline_id = snapshot_worldline_id();
    let initial_state = common::create_initial_worldline_state(warp_id);
    let mut provenance = LocalProvenanceStore::new();
    register_fixture_worldline(&mut provenance, worldline_id, &initial_state)
        .expect("fixture worldline should register");

    let mut expected_roots = vec![initial_state.state_root()];
    let mut states_by_tick = vec![initial_state.clone()];
    let mut current_state = initial_state.clone();
    let mut parents: Vec<Hash> = Vec::new();

    for tick in 0..total_ticks {
        let patch = snapshot_fuzz_patch(warp_id, tick);
        patch
            .apply_to_worldline_state(&mut current_state)
            .expect("generated fuzz patch should apply");

        let state_root = current_state.state_root();
        let commit_hash = compute_commit_hash_v2(
            &state_root,
            &parents,
            &patch.patch_digest,
            patch.policy_id(),
        );
        let triplet = HashTriplet {
            state_root,
            patch_digest: patch.patch_digest,
            commit_hash,
        };
        append_fixture_entry(&mut provenance, worldline_id, patch, triplet, Vec::new())
            .expect("fixture entry should append");

        parents = vec![commit_hash];
        expected_roots.push(state_root);
        states_by_tick.push(current_state.clone());
    }

    Simulation {
        provenance,
        warp_id,
        worldline_id,
        expected_roots,
        states_by_tick,
    }
}

fn apply_suffix_from(
    provenance: &LocalProvenanceStore,
    worldline_id: WorldlineId,
    mut state: WorldlineState,
    start_tick: u64,
    target_tick: u64,
) -> TestResult<WorldlineState> {
    for raw_tick in start_tick..target_tick {
        let tick = wt(raw_tick);
        let entry = provenance
            .entry(worldline_id, tick)
            .map_err(|error| format!("missing provenance at tick {raw_tick}: {error}"))?;
        let patch = entry
            .patch
            .as_ref()
            .ok_or_else(|| format!("missing replay patch at tick {raw_tick}"))?;

        patch
            .apply_to_worldline_state(&mut state)
            .map_err(|error| format!("restore suffix apply failed at tick {raw_tick}: {error}"))?;

        let actual_state_root = state.state_root();
        if actual_state_root != entry.expected.state_root {
            return Err(format!(
                "state_root mismatch at tick {raw_tick}: expected {}, got {}",
                hex32(&entry.expected.state_root),
                hex32(&actual_state_root)
            ));
        }

        let parents = entry
            .parents
            .iter()
            .map(|parent| parent.commit_hash)
            .collect::<Vec<_>>();
        let actual_commit_hash = compute_commit_hash_v2(
            &actual_state_root,
            &parents,
            &entry.expected.patch_digest,
            patch.policy_id(),
        );
        if actual_commit_hash != entry.expected.commit_hash {
            return Err(format!(
                "commit_hash mismatch at tick {raw_tick}: expected {}, got {}",
                hex32(&entry.expected.commit_hash),
                hex32(&actual_commit_hash)
            ));
        }
    }
    Ok(state)
}

fn materialize_state_at(simulation: &Simulation, tick: u64) -> TestResult<WorldlineState> {
    simulation
        .states_by_tick
        .get(tick as usize)
        .cloned()
        .ok_or_else(|| format!("snapshot tick {tick} is outside materialized states"))
}

fn encode_snapshot(
    state: &WorldlineState,
    warp_id: WarpId,
    tick: u64,
    format: SnapshotFormat,
) -> TestResult<Vec<u8>> {
    match format {
        SnapshotFormat::CanonicalWscV1 => {
            let store = state
                .store(&warp_id)
                .ok_or_else(|| format!("snapshot state missing warp {warp_id:?}"))?;
            let input = build_one_warp_input(store, state.root().local_id);
            write_wsc_one_warp(&input, SNAPSHOT_SCHEMA_HASH, tick)
                .map_err(|error| format!("WSC snapshot encode failed: {error}"))
        }
    }
}

fn decode_attachment(view: &WarpView<'_>, row: &AttRow) -> TestResult<AttachmentValue> {
    if row.is_atom() {
        let blob = view
            .blob_for_attachment(row)
            .ok_or_else(|| "atom attachment blob range is invalid".to_string())?;
        return Ok(AttachmentValue::Atom(AtomPayload::new(
            TypeId(row.type_or_warp),
            Bytes::copy_from_slice(blob),
        )));
    }

    if row.is_descend() {
        return Ok(AttachmentValue::Descend(WarpId(row.type_or_warp)));
    }

    Err(format!("unknown attachment tag {}", row.tag))
}

fn decode_single_attachment(
    view: &WarpView<'_>,
    owner: &'static str,
    rows: &[AttRow],
) -> TestResult<Option<AttachmentValue>> {
    match rows {
        [] => Ok(None),
        [row] => decode_attachment(view, row).map(Some),
        _ => Err(format!("{owner} carried more than one attachment row")),
    }
}

fn restore_snapshot(bytes: &[u8], expected_tick: u64) -> TestResult<WorldlineState> {
    let file = WscFile::from_bytes(bytes.to_vec())
        .map_err(|error| format!("WSC snapshot header restore failed: {error}"))?;
    validate_wsc(&file).map_err(|error| format!("WSC snapshot validation failed: {error}"))?;
    if file.tick() != expected_tick {
        return Err(format!(
            "WSC snapshot tick mismatch: expected {expected_tick}, got {}",
            file.tick()
        ));
    }
    if file.warp_count() != 1 {
        return Err(format!(
            "expected exactly one WARP in snapshot, got {}",
            file.warp_count()
        ));
    }

    let view = file
        .warp_view(0)
        .map_err(|error| format!("WSC warp restore failed: {error}"))?;
    let warp_id = WarpId(*view.warp_id());
    let root = NodeId(*view.root_node_id());
    let mut store = GraphStore::new(warp_id);

    for node in view.nodes() {
        store.insert_node(
            NodeId(node.node_id),
            NodeRecord {
                ty: TypeId(node.node_type),
            },
        );
    }
    for edge in view.edges() {
        store.insert_edge(
            NodeId(edge.from_node_id),
            EdgeRecord {
                id: EdgeId(edge.edge_id),
                from: NodeId(edge.from_node_id),
                to: NodeId(edge.to_node_id),
                ty: TypeId(edge.edge_type),
            },
        );
    }

    for (index, node) in view.nodes().iter().enumerate() {
        let rows = view.node_attachments(index);
        if let Some(value) = decode_single_attachment(&view, "node", rows)? {
            store.set_node_attachment(NodeId(node.node_id), Some(value));
        }
    }
    for (index, edge) in view.edges().iter().enumerate() {
        let rows = view.edge_attachments(index);
        if let Some(value) = decode_single_attachment(&view, "edge", rows)? {
            store.set_edge_attachment(EdgeId(edge.edge_id), Some(value));
        }
    }

    WorldlineState::from_root_store(store, root)
        .map_err(|error| format!("restored WSC graph is not a worldline state: {error}"))
}

fn run_iteration_from_snapshot_bytes(
    simulation: &Simulation,
    iteration: usize,
    format: SnapshotFormat,
    snapshot_tick: u64,
    comparison_tick: u64,
    bytes: &[u8],
) -> TestResult<SnapshotRestoreIteration> {
    let restored = restore_snapshot(bytes, snapshot_tick)?;
    let restored_state_root = restored.state_root();
    let continued = apply_suffix_from(
        &simulation.provenance,
        simulation.worldline_id,
        restored,
        snapshot_tick,
        comparison_tick,
    )?;
    let actual_state_root = continued.state_root();
    let expected_state_root = simulation
        .expected_roots
        .get(comparison_tick as usize)
        .copied()
        .ok_or_else(|| format!("comparison tick {comparison_tick} is outside expected roots"))?;

    Ok(SnapshotRestoreIteration {
        iteration,
        format,
        snapshot_tick,
        restore_tick: snapshot_tick,
        comparison_tick,
        restored_state_root,
        expected_state_root,
        actual_state_root,
    })
}

fn run_iteration(
    simulation: &Simulation,
    iteration: usize,
    snapshot_tick: u64,
    comparison_tick: u64,
    format: SnapshotFormat,
) -> TestResult<SnapshotRestoreIteration> {
    let snapshot_state = materialize_state_at(simulation, snapshot_tick)?;
    let bytes = encode_snapshot(&snapshot_state, simulation.warp_id, snapshot_tick, format)?;
    run_iteration_from_snapshot_bytes(
        simulation,
        iteration,
        format,
        snapshot_tick,
        comparison_tick,
        &bytes,
    )
}

fn iteration_ticks(iteration: usize, rng: &mut XorShift64) -> (u64, u64) {
    match iteration {
        // Genesis snapshot; replay the whole suffix.
        0 => (0, TOTAL_TICKS),
        // Last-tick snapshot; restore and compare immediately.
        1 => (TOTAL_TICKS, TOTAL_TICKS),
        _ => {
            let snapshot_tick = rng.gen_range_usize((TOTAL_TICKS + 1) as usize) as u64;
            let remaining = (TOTAL_TICKS - snapshot_tick) as usize;
            let advance = rng.gen_range_usize(remaining + 1) as u64;
            (snapshot_tick, snapshot_tick + advance)
        }
    }
}

fn run_snapshot_restore_fuzz() -> TestResult<SnapshotRestoreReport> {
    let simulation = build_simulation(TOTAL_TICKS);
    let mut rng = XorShift64::new(0xA11C_EC0F_FEE0_0004);
    let mut iterations = Vec::with_capacity(FUZZ_ITERATIONS);

    for iteration in 0..FUZZ_ITERATIONS {
        let (snapshot_tick, comparison_tick) = iteration_ticks(iteration, &mut rng);
        iterations.push(run_iteration(
            &simulation,
            iteration,
            snapshot_tick,
            comparison_tick,
            SnapshotFormat::CanonicalWscV1,
        )?);
    }

    Ok(SnapshotRestoreReport {
        simulation_ticks: TOTAL_TICKS,
        iterations,
    })
}

fn corrupt_first_edge_id_byte(bytes: &mut [u8]) -> TestResult<()> {
    let header_size = std::mem::size_of::<WscHeader>();
    let dir_size = std::mem::size_of::<WarpDirEntry>();
    if bytes.len() < header_size {
        return Err("WSC bytes are shorter than the header".to_string());
    }
    let header = bytemuck::from_bytes::<WscHeader>(&bytes[..header_size]);
    let dir_start = header.warp_dir_off() as usize;
    let dir_end = dir_start + dir_size;
    if bytes.len() < dir_end {
        return Err("WSC bytes are shorter than the WARP directory".to_string());
    }
    let dir = bytemuck::from_bytes::<WarpDirEntry>(&bytes[dir_start..dir_end]);
    if u64::from_le(dir.edges_len_le) == 0 {
        return Err("WSC snapshot has no edge row to corrupt".to_string());
    }
    let edge_start = u64::from_le(dir.edges_off_le) as usize;
    let byte = bytes
        .get_mut(edge_start)
        .ok_or_else(|| "first edge row offset is outside WSC bytes".to_string())?;
    *byte ^= 0x80;
    Ok(())
}

#[test]
fn snapshot_restore_fuzz_matches_uninterrupted_run() {
    let report = run_snapshot_restore_fuzz().expect("snapshot/restore fuzz should run");
    let json = report.to_json();

    assert_eq!(report.iterations.len(), FUZZ_ITERATIONS, "{json}");
    assert_eq!(report.divergence_count(), 0, "{json}");
    assert!(json.contains("\"iteration_count\": 50"));
    assert!(json.contains("\"format\": \"canonical_wsc_v1\""));
    assert!(json.contains("\"snapshot_tick\":"));
    assert!(json.contains("\"restore_tick\":"));
    assert!(json.contains("\"comparison_tick\":"));
    assert!(json.contains("\"expected_state_root\":"));
    assert!(json.contains("\"actual_state_root\":"));
    assert!(json.contains("\"match\": true"));
}

#[test]
fn corrupted_snapshot_byte_fails_restore_or_reports_divergence() {
    let simulation = build_simulation(32);
    let snapshot_tick = 12;
    let comparison_tick = 32;
    let snapshot_state =
        materialize_state_at(&simulation, snapshot_tick).expect("snapshot materialization");
    let mut bytes = encode_snapshot(
        &snapshot_state,
        simulation.warp_id,
        snapshot_tick,
        SnapshotFormat::CanonicalWscV1,
    )
    .expect("snapshot encode");
    corrupt_first_edge_id_byte(&mut bytes).expect("snapshot should contain an edge to corrupt");

    match run_iteration_from_snapshot_bytes(
        &simulation,
        0,
        SnapshotFormat::CanonicalWscV1,
        snapshot_tick,
        comparison_tick,
        &bytes,
    ) {
        Err(_) => {}
        Ok(iteration) => {
            assert!(
                !iteration.matches(),
                "corrupted snapshot unexpectedly matched uninterrupted root: {}",
                hex32(&iteration.actual_state_root)
            );
        }
    }
}
