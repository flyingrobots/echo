// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use alloc::{vec, vec::Vec};

use ciborium::value::Value;

use crate::{
    CanonError, decode_cbor, decode_value, encode_cbor, encode_value,
    kernel_port::{
        BaseRef, CausalSuffixBundle, ConflictReason, ExportSuffixObstruction, ExportSuffixRequest,
        ImportSuffixRequest, ImportSuffixResult, ProvenanceRef, ReadingResidualPosture,
        SettlementBasisReport, SettlementOverlapRevalidation, SettlementParentRevalidation,
        WitnessedSuffixAdmissionOutcome, WitnessedSuffixAdmissionRequest,
        WitnessedSuffixAdmissionResponse, WitnessedSuffixShell, WorldlineId, WorldlineTick,
    },
};

fn worldline(seed: u8) -> WorldlineId {
    WorldlineId::from_bytes([seed; 32])
}

fn tick(value: u64) -> WorldlineTick {
    WorldlineTick(value)
}

fn provenance_ref(seed: u8, worldline_tick: u64) -> ProvenanceRef {
    ProvenanceRef {
        worldline_id: worldline(seed),
        worldline_tick: tick(worldline_tick),
        commit_hash: vec![seed.wrapping_add(1); 32],
    }
}

fn base_ref(seed: u8) -> BaseRef {
    BaseRef {
        source_worldline_id: worldline(seed),
        fork_tick: tick(1),
        commit_hash: vec![seed.wrapping_add(2); 32],
        boundary_hash: vec![seed.wrapping_add(3); 32],
        provenance_ref: provenance_ref(seed.wrapping_add(4), 1),
    }
}

fn basis_report() -> SettlementBasisReport {
    SettlementBasisReport {
        parent_anchor: base_ref(20),
        child_worldline_id: worldline(21),
        source_suffix_start_tick: tick(2),
        source_suffix_end_tick: Some(tick(5)),
        realized_parent_ref: provenance_ref(22, 9),
        owned_closed_slot_count: 3,
        parent_written_slot_count: 1,
        parent_revalidation: SettlementParentRevalidation::ParentAdvancedDisjoint {
            parent_from: provenance_ref(23, 1),
            parent_to: provenance_ref(24, 9),
        },
    }
}

fn shell_with_entries(entries: Vec<ProvenanceRef>) -> WitnessedSuffixShell {
    WitnessedSuffixShell {
        source_worldline_id: worldline(3),
        source_suffix_start_tick: tick(2),
        source_suffix_end_tick: Some(tick(4)),
        source_entries: entries,
        boundary_witness: Some(provenance_ref(5, 1)),
        witness_digest: vec![6; 32],
        basis_report: Some(basis_report()),
    }
}

fn request() -> WitnessedSuffixAdmissionRequest {
    WitnessedSuffixAdmissionRequest {
        source_suffix: shell_with_entries(vec![provenance_ref(4, 3)]),
        target_worldline_id: worldline(11),
        target_basis: provenance_ref(12, 9),
        basis_report: Some(basis_report()),
    }
}

fn response(outcome: WitnessedSuffixAdmissionOutcome) -> WitnessedSuffixAdmissionResponse {
    WitnessedSuffixAdmissionResponse {
        source_shell_digest: vec![6; 32],
        target_basis: provenance_ref(12, 9),
        outcome,
    }
}

fn export_request() -> ExportSuffixRequest {
    ExportSuffixRequest {
        source_worldline_id: worldline(3),
        base_frontier: provenance_ref(3, 2),
        target_frontier: Some(provenance_ref(3, 4)),
        basis_report: Some(basis_report()),
    }
}

fn causal_suffix_bundle() -> CausalSuffixBundle {
    CausalSuffixBundle {
        base_frontier: provenance_ref(3, 2),
        target_frontier: provenance_ref(3, 4),
        source_suffix: shell_with_entries(vec![provenance_ref(3, 3), provenance_ref(3, 4)]),
        bundle_digest: vec![7; 32],
    }
}

fn import_request() -> ImportSuffixRequest {
    ImportSuffixRequest {
        bundle: causal_suffix_bundle(),
        target_worldline_id: worldline(11),
        target_basis: provenance_ref(12, 9),
        basis_report: Some(basis_report()),
    }
}

fn import_result(outcome: WitnessedSuffixAdmissionOutcome) -> ImportSuffixResult {
    ImportSuffixResult {
        bundle_digest: vec![7; 32],
        admission: response(outcome),
    }
}

fn overlap_revalidation() -> SettlementOverlapRevalidation {
    SettlementOverlapRevalidation::Conflict {
        overlapping_slot_count: 2,
        overlapping_slots_digest: vec![31; 32],
    }
}

fn assert_response_round_trip(
    outcome: WitnessedSuffixAdmissionOutcome,
) -> Result<(), crate::CanonError> {
    let original = response(outcome);
    let bytes = encode_cbor(&original)?;
    let decoded: WitnessedSuffixAdmissionResponse = decode_cbor(&bytes)?;

    assert_eq!(decoded, original);
    Ok(())
}

fn value_from_cbor<T: serde::Serialize>(value: &T) -> Result<Value, CanonError> {
    decode_value(&encode_cbor(value)?)
}

fn request_value() -> Result<Value, CanonError> {
    value_from_cbor(&request())
}

fn remove_map_field(value: &mut Value, name: &str) -> Option<Value> {
    let Value::Map(fields) = value else {
        return None;
    };
    let position = fields
        .iter()
        .position(|(key, _)| matches!(key, Value::Text(field) if field == name))?;
    Some(fields.remove(position).1)
}

fn insert_map_field(value: &mut Value, name: &str, field_value: Value) -> bool {
    let Value::Map(fields) = value else {
        return false;
    };
    fields.push((Value::Text(name.into()), field_value));
    true
}

fn decode_value_as<T>(value: &Value) -> Result<T, CanonError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    decode_cbor(&encode_value(value)?)
}

fn admitted_outcome() -> WitnessedSuffixAdmissionOutcome {
    WitnessedSuffixAdmissionOutcome::Admitted {
        target_worldline_id: worldline(11),
        admitted_refs: vec![provenance_ref(30, 10)],
        basis_report: Some(basis_report()),
    }
}

#[test]
fn witnessed_suffix_export_request_round_trips() -> Result<(), crate::CanonError> {
    let original = export_request();
    let bytes = encode_cbor(&original)?;
    let decoded: ExportSuffixRequest = decode_cbor(&bytes)?;

    assert_eq!(decoded, original);
    assert_eq!(decoded.base_frontier, provenance_ref(3, 2));
    assert_eq!(decoded.target_frontier, Some(provenance_ref(3, 4)));
    Ok(())
}

#[test]
fn witnessed_suffix_causal_bundle_round_trips() -> Result<(), crate::CanonError> {
    let original = causal_suffix_bundle();
    let bytes = encode_cbor(&original)?;
    let decoded: CausalSuffixBundle = decode_cbor(&bytes)?;

    assert_eq!(decoded, original);
    assert_eq!(decoded.bundle_digest, vec![7; 32]);
    assert_eq!(
        decoded.source_suffix.source_entries,
        vec![provenance_ref(3, 3), provenance_ref(3, 4)]
    );
    Ok(())
}

#[test]
fn witnessed_suffix_import_request_round_trips() -> Result<(), crate::CanonError> {
    let original = import_request();
    let bytes = encode_cbor(&original)?;
    let decoded: ImportSuffixRequest = decode_cbor(&bytes)?;

    assert_eq!(decoded, original);
    assert_eq!(decoded.target_basis, provenance_ref(12, 9));
    assert_eq!(decoded.bundle.bundle_digest, vec![7; 32]);
    Ok(())
}

#[test]
fn witnessed_suffix_import_result_round_trips() -> Result<(), crate::CanonError> {
    let original = import_result(admitted_outcome());
    let bytes = encode_cbor(&original)?;
    let decoded: ImportSuffixResult = decode_cbor(&bytes)?;

    assert_eq!(decoded, original);
    assert_eq!(decoded.bundle_digest, vec![7; 32]);
    assert!(matches!(
        decoded.admission.outcome,
        WitnessedSuffixAdmissionOutcome::Admitted { .. }
    ));
    Ok(())
}

#[test]
fn witnessed_suffix_export_obstruction_round_trips() -> Result<(), crate::CanonError> {
    let original = ExportSuffixObstruction {
        source_ref: provenance_ref(3, 2),
        residual_posture: ReadingResidualPosture::Obstructed,
        evidence_digest: vec![8; 32],
    };
    let bytes = encode_cbor(&original)?;
    let decoded: ExportSuffixObstruction = decode_cbor(&bytes)?;

    assert_eq!(decoded, original);
    Ok(())
}

#[test]
fn witnessed_suffix_request_round_trips_with_source_and_target_refs()
-> Result<(), crate::CanonError> {
    let original = request();
    let bytes = encode_cbor(&original)?;
    let decoded: WitnessedSuffixAdmissionRequest = decode_cbor(&bytes)?;

    assert_eq!(decoded, original);
    assert_eq!(
        decoded.source_suffix.source_entries,
        vec![provenance_ref(4, 3)]
    );
    assert_eq!(decoded.target_basis, provenance_ref(12, 9));
    Ok(())
}

#[test]
fn witnessed_suffix_response_round_trips_admitted() -> Result<(), crate::CanonError> {
    assert_response_round_trip(admitted_outcome())
}

#[test]
fn witnessed_suffix_response_round_trips_staged() -> Result<(), crate::CanonError> {
    assert_response_round_trip(WitnessedSuffixAdmissionOutcome::Staged {
        staged_refs: vec![provenance_ref(32, 11)],
        basis_report: Some(basis_report()),
    })
}

#[test]
fn witnessed_suffix_response_round_trips_plural() -> Result<(), crate::CanonError> {
    assert_response_round_trip(WitnessedSuffixAdmissionOutcome::Plural {
        candidate_refs: vec![provenance_ref(33, 12), provenance_ref(34, 13)],
        residual_posture: ReadingResidualPosture::PluralityPreserved,
        basis_report: Some(basis_report()),
    })
}

#[test]
fn witnessed_suffix_response_round_trips_conflict() -> Result<(), crate::CanonError> {
    assert_response_round_trip(WitnessedSuffixAdmissionOutcome::Conflict {
        reason: ConflictReason::ParentFootprintOverlap,
        source_ref: provenance_ref(35, 14),
        evidence_digest: vec![36; 32],
        overlap_revalidation: Some(overlap_revalidation()),
    })
}

#[test]
fn witnessed_suffix_response_round_trips_obstructed() -> Result<(), crate::CanonError> {
    assert_response_round_trip(WitnessedSuffixAdmissionOutcome::Obstructed {
        source_ref: provenance_ref(37, 15),
        residual_posture: ReadingResidualPosture::Obstructed,
        evidence_digest: vec![38; 32],
    })
}

#[test]
fn witnessed_suffix_shell_round_trips_empty_suffix() -> Result<(), crate::CanonError> {
    let mut shell = shell_with_entries(Vec::new());
    shell.source_suffix_end_tick = None;

    let bytes = encode_cbor(&shell)?;
    let decoded: WitnessedSuffixShell = decode_cbor(&bytes)?;

    assert_eq!(decoded.source_entries, Vec::new());
    assert_eq!(decoded.source_suffix_end_tick, None);
    assert_eq!(decoded, shell);
    Ok(())
}

#[test]
fn witnessed_suffix_shell_round_trips_single_entry_suffix() -> Result<(), crate::CanonError> {
    let shell = shell_with_entries(vec![provenance_ref(41, 16)]);
    let bytes = encode_cbor(&shell)?;
    let decoded: WitnessedSuffixShell = decode_cbor(&bytes)?;

    assert_eq!(decoded.source_entries, vec![provenance_ref(41, 16)]);
    assert_eq!(decoded, shell);
    Ok(())
}

#[test]
fn witnessed_suffix_shell_round_trips_boundary_witness_only() -> Result<(), crate::CanonError> {
    let mut shell = shell_with_entries(Vec::new());
    shell.source_suffix_end_tick = None;
    shell.boundary_witness = Some(provenance_ref(42, 1));

    let bytes = encode_cbor(&shell)?;
    let decoded: WitnessedSuffixShell = decode_cbor(&bytes)?;

    assert_eq!(decoded.boundary_witness, Some(provenance_ref(42, 1)));
    assert_eq!(decoded, shell);
    Ok(())
}

#[test]
fn witnessed_suffix_request_rejects_missing_source_suffix() -> Result<(), CanonError> {
    let mut value = request_value()?;
    assert!(remove_map_field(&mut value, "source_suffix").is_some());

    assert!(decode_value_as::<WitnessedSuffixAdmissionRequest>(&value).is_err());
    Ok(())
}

#[test]
fn witnessed_suffix_request_rejects_missing_target_basis() -> Result<(), CanonError> {
    let mut value = request_value()?;
    assert!(remove_map_field(&mut value, "target_basis").is_some());

    assert!(decode_value_as::<WitnessedSuffixAdmissionRequest>(&value).is_err());
    Ok(())
}

#[test]
fn witnessed_suffix_response_rejects_zero_outcomes() -> Result<(), CanonError> {
    let mut value = value_from_cbor(&response(admitted_outcome()))?;
    assert!(remove_map_field(&mut value, "outcome").is_some());

    assert!(decode_value_as::<WitnessedSuffixAdmissionResponse>(&value).is_err());
    Ok(())
}

#[test]
fn witnessed_suffix_response_rejects_multiple_outcomes() -> Result<(), CanonError> {
    let mut value = value_from_cbor(&response(admitted_outcome()))?;
    let Some(outcome) = remove_map_field(&mut value, "outcome") else {
        return Err(CanonError::Decode("outcome field missing".into()));
    };
    assert!(insert_map_field(
        &mut value,
        "outcomes",
        Value::Array(vec![outcome.clone(), outcome])
    ));

    assert!(decode_value_as::<WitnessedSuffixAdmissionResponse>(&value).is_err());
    Ok(())
}

#[test]
fn witnessed_suffix_request_rejects_raw_transport_endpoint() -> Result<(), CanonError> {
    let mut value = request_value()?;
    assert!(insert_map_field(
        &mut value,
        "transport_endpoint",
        Value::Text("https://example.invalid/sync".into())
    ));

    assert!(decode_value_as::<WitnessedSuffixAdmissionRequest>(&value).is_err());
    Ok(())
}

#[test]
fn witnessed_suffix_request_rejects_network_sync_api() -> Result<(), CanonError> {
    let mut value = request_value()?;
    assert!(insert_map_field(
        &mut value,
        "network_sync_api",
        Value::Map(vec![(
            Value::Text("kind".into()),
            Value::Text("pull".into())
        )])
    ));

    assert!(decode_value_as::<WitnessedSuffixAdmissionRequest>(&value).is_err());
    Ok(())
}
