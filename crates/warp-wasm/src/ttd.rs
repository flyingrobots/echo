// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! TTD (Time-Travel Debugger) WASM bindings.
//!
//! This module exposes TTD primitives to JavaScript via `wasm-bindgen`:
//! - Digest computation (emissions, op index, commit hash)
//! - Compliance checking (channel policy validation)
//! - Wire codecs (EINT v2, TTDR v2)
//!
//! These are the building blocks for the `ttd-browser` crate.

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

// ─── Digest Functions ────────────────────────────────────────────────────────

/// Compute the emissions digest from finalized channel data.
///
/// # Arguments
///
/// * `channels_cbor` - CBOR-encoded array of `{channel: [u8;32], data: [u8]}`
///
/// # Returns
///
/// 32-byte BLAKE3 digest as `Uint8Array`, or throws on decode error.
#[wasm_bindgen]
pub fn compute_emissions_digest(channels_cbor: &[u8]) -> Result<Uint8Array, JsError> {
    let channels: Vec<ChannelEntry> =
        ciborium::from_reader(channels_cbor).map_err(|e| JsError::new(&e.to_string()))?;

    let finalized: Vec<warp_core::materialization::FinalizedChannel> = channels
        .into_iter()
        .map(|c| warp_core::materialization::FinalizedChannel {
            channel: warp_core::TypeId(c.channel),
            data: c.data,
        })
        .collect();

    let digest = warp_core::compute_emissions_digest(&finalized);
    Ok(hash_to_uint8array(&digest))
}

/// Compute the op emission index digest.
///
/// # Arguments
///
/// * `entries_cbor` - CBOR-encoded array of `{op_id: [u8;32], channels: [[u8;32]]}`
///
/// # Returns
///
/// 32-byte BLAKE3 digest as `Uint8Array`, or throws on decode error.
#[wasm_bindgen]
pub fn compute_op_emission_index_digest(entries_cbor: &[u8]) -> Result<Uint8Array, JsError> {
    let entries: Vec<OpEmissionEntryJs> =
        ciborium::from_reader(entries_cbor).map_err(|e| JsError::new(&e.to_string()))?;

    let core_entries: Vec<warp_core::OpEmissionEntry> = entries
        .into_iter()
        .map(|e| warp_core::OpEmissionEntry {
            op_id: e.op_id,
            channels: e.channels.into_iter().map(warp_core::TypeId).collect(),
        })
        .collect();

    let digest = warp_core::compute_op_emission_index_digest(&core_entries);
    Ok(hash_to_uint8array(&digest))
}

/// Compute the TTD tick commit hash (v2).
///
/// # Arguments
///
/// * `schema_hash` - 32-byte schema hash
/// * `worldline_id` - 32-byte worldline identifier
/// * `tick` - Tick number (u64)
/// * `parent_hashes` - CBOR-encoded array of 32-byte parent hashes (pre-sorted)
/// * `patch_digest` - 32-byte patch digest
/// * `state_root` - Optional 32-byte state root (null if not present)
/// * `emissions_digest` - 32-byte emissions digest
/// * `op_emission_index_digest` - Optional 32-byte op emission index digest
///
/// # Returns
///
/// 32-byte BLAKE3 commit hash as `Uint8Array`.
// Allow many arguments: this signature matches the TTD wire spec exactly.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn compute_tick_commit_hash(
    schema_hash: &[u8],
    worldline_id: &[u8],
    tick: u64,
    parent_hashes_cbor: &[u8],
    patch_digest: &[u8],
    state_root: Option<Vec<u8>>,
    emissions_digest: &[u8],
    op_emission_index_digest: Option<Vec<u8>>,
) -> Result<Uint8Array, JsError> {
    // Validate input lengths
    let schema: [u8; 32] = schema_hash
        .try_into()
        .map_err(|_| JsError::new("schema_hash must be 32 bytes"))?;
    let worldline: [u8; 32] = worldline_id
        .try_into()
        .map_err(|_| JsError::new("worldline_id must be 32 bytes"))?;
    let patch: [u8; 32] = patch_digest
        .try_into()
        .map_err(|_| JsError::new("patch_digest must be 32 bytes"))?;
    let emissions: [u8; 32] = emissions_digest
        .try_into()
        .map_err(|_| JsError::new("emissions_digest must be 32 bytes"))?;

    // Decode parent hashes
    let parents: Vec<[u8; 32]> =
        ciborium::from_reader(parent_hashes_cbor).map_err(|e| JsError::new(&e.to_string()))?;

    // Convert optional state root
    let state_root_arr: Option<[u8; 32]> = state_root
        .map(|v| {
            v.try_into()
                .map_err(|_| JsError::new("state_root must be 32 bytes"))
        })
        .transpose()?;

    // Convert optional op emission index digest
    let op_idx_arr: Option<[u8; 32]> = op_emission_index_digest
        .map(|v| {
            v.try_into()
                .map_err(|_| JsError::new("op_emission_index_digest must be 32 bytes"))
        })
        .transpose()?;

    let hash = warp_core::compute_tick_commit_hash_v2(
        &schema,
        &warp_core::WorldlineId(worldline),
        tick,
        &parents,
        &patch,
        state_root_arr.as_ref(),
        &emissions,
        op_idx_arr.as_ref(),
    );

    Ok(hash_to_uint8array(&hash))
}

// ─── Compliance Checking ─────────────────────────────────────────────────────

/// Check channel policies against emissions.
///
/// # Arguments
///
/// * `emissions_cbor` - CBOR-encoded array of `{channel: [u8;32], data: [u8]}`
/// * `policies_cbor` - CBOR-encoded array of `{channel: [u8;32], policy: string}`
///   where policy is "Log", "StrictSingle", or "Reduce:<op>"
/// * `strict_mode` - If true, unknown channels are errors; if false, warnings
///
/// # Returns
///
/// CBOR-encoded array of violations.
#[wasm_bindgen]
pub fn check_channel_policies(
    emissions_cbor: &[u8],
    policies_cbor: &[u8],
    strict_mode: bool,
) -> Result<Uint8Array, JsError> {
    // Decode emissions
    let channels: Vec<ChannelEntry> =
        ciborium::from_reader(emissions_cbor).map_err(|e| JsError::new(&e.to_string()))?;

    let finalized: Vec<warp_core::materialization::FinalizedChannel> = channels
        .into_iter()
        .map(|c| warp_core::materialization::FinalizedChannel {
            channel: warp_core::TypeId(c.channel),
            data: c.data,
        })
        .collect();

    // Decode policies
    let policy_entries: Vec<PolicyEntry> =
        ciborium::from_reader(policies_cbor).map_err(|e| JsError::new(&e.to_string()))?;

    let policies: Vec<(warp_core::TypeId, warp_core::materialization::ChannelPolicy)> =
        policy_entries
            .into_iter()
            .map(|p| {
                let channel_id = warp_core::TypeId(p.channel);
                let policy = parse_policy(&p.policy)?;
                Ok((channel_id, policy))
            })
            .collect::<Result<Vec<_>, JsError>>()?;

    // Run compliance check
    let checker = if strict_mode {
        echo_ttd::PolicyChecker::strict()
    } else {
        echo_ttd::PolicyChecker::new()
    };

    let violations = checker.check_channel_policies(&finalized, &policies);

    // Encode violations to CBOR
    let js_violations: Vec<ViolationJs> = violations
        .into_iter()
        .map(|v| ViolationJs {
            severity: format!("{}", v.severity),
            code: format!("{}", v.code),
            message: v.message,
            channel_id: v.channel_id.map(|c| c.0),
            emission_count: v.emission_count,
        })
        .collect();

    let mut buf = Vec::new();
    ciborium::into_writer(&js_violations, &mut buf)
        .map_err(|e| JsError::new(&format!("CBOR encode error: {e}")))?;

    Ok(bytes_to_uint8array(&buf))
}

/// Get a compliance summary from violations.
///
/// # Arguments
///
/// * `violations_cbor` - CBOR-encoded array of violations (from `check_channel_policies`)
///
/// # Returns
///
/// CBOR-encoded summary object.
#[wasm_bindgen]
pub fn compliance_summary(violations_cbor: &[u8]) -> Result<Uint8Array, JsError> {
    let violations: Vec<ViolationJs> =
        ciborium::from_reader(violations_cbor).map_err(|e| JsError::new(&e.to_string()))?;

    // Count by severity
    let mut fatal = 0u32;
    let mut error = 0u32;
    let mut warn = 0u32;
    let mut info = 0u32;

    for v in &violations {
        match v.severity.as_str() {
            "FATAL" => fatal += 1,
            "ERROR" => error += 1,
            "WARN" => warn += 1,
            "INFO" => info += 1,
            _ => {}
        }
    }

    let summary = SummaryJs {
        fatal_count: fatal,
        error_count: error,
        warn_count: warn,
        info_count: info,
        is_green: fatal == 0 && error == 0,
    };

    let mut buf = Vec::new();
    ciborium::into_writer(&summary, &mut buf)
        .map_err(|e| JsError::new(&format!("CBOR encode error: {e}")))?;

    Ok(bytes_to_uint8array(&buf))
}

// ─── Wire Codec Bindings ─────────────────────────────────────────────────────

/// Encode an EINT v2 frame.
///
/// # Arguments
///
/// * `schema_hash` - 32-byte schema hash
/// * `opcode` - 32-bit opcode (note: EINT v2 uses u32)
/// * `op_version` - 16-bit op version
/// * `payload` - Intent payload bytes
///
/// # Returns
///
/// Encoded EINT v2 frame as `Uint8Array`.
#[wasm_bindgen]
pub fn encode_eint_v2(
    schema_hash: &[u8],
    opcode: u32,
    op_version: u16,
    payload: &[u8],
) -> Result<Uint8Array, JsError> {
    let schema: [u8; 32] = schema_hash
        .try_into()
        .map_err(|_| JsError::new("schema_hash must be 32 bytes"))?;

    let bytes = echo_session_proto::encode_eint_v2(
        schema,
        opcode,
        op_version,
        echo_session_proto::EintFlags::default(),
        payload,
    )
    .map_err(|e| JsError::new(&e.to_string()))?;

    Ok(bytes_to_uint8array(&bytes))
}

/// Decode an EINT v2 frame.
///
/// # Arguments
///
/// * `bytes` - Encoded EINT v2 frame
///
/// # Returns
///
/// CBOR-encoded object with header fields and payload.
#[wasm_bindgen]
pub fn decode_eint_v2(bytes: &[u8]) -> Result<Uint8Array, JsError> {
    let (frame, _consumed) =
        echo_session_proto::decode_eint_v2(bytes).map_err(|e| JsError::new(&e.to_string()))?;

    let result = EintFrameJs {
        schema_hash: frame.header.schema_hash,
        opcode: frame.header.opcode,
        op_version: frame.header.op_version,
        payload: frame.payload.to_vec(),
    };

    let mut buf = Vec::new();
    ciborium::into_writer(&result, &mut buf)
        .map_err(|e| JsError::new(&format!("CBOR encode error: {e}")))?;

    Ok(bytes_to_uint8array(&buf))
}

/// Encode a TTDR v2 frame (Light mode - minimal receipt).
///
/// # Arguments
///
/// * `schema_hash` - 32-byte schema hash
/// * `worldline_id` - 32-byte worldline ID
/// * `tick` - Tick number
/// * `commit_hash` - 32-byte commit hash
/// * `emissions_digest` - 32-byte emissions digest
/// * `state_root` - Optional 32-byte state root
///
/// # Returns
///
/// Encoded TTDR v2 frame (Light mode) as `Uint8Array`.
#[wasm_bindgen]
pub fn encode_ttdr_v2_light(
    schema_hash: &[u8],
    worldline_id: &[u8],
    tick: u64,
    commit_hash: &[u8],
    emissions_digest: &[u8],
    state_root: Option<Vec<u8>>,
) -> Result<Uint8Array, JsError> {
    let schema: [u8; 32] = schema_hash
        .try_into()
        .map_err(|_| JsError::new("schema_hash must be 32 bytes"))?;
    let worldline: [u8; 32] = worldline_id
        .try_into()
        .map_err(|_| JsError::new("worldline_id must be 32 bytes"))?;
    let commit: [u8; 32] = commit_hash
        .try_into()
        .map_err(|_| JsError::new("commit_hash must be 32 bytes"))?;
    let emissions: [u8; 32] = emissions_digest
        .try_into()
        .map_err(|_| JsError::new("emissions_digest must be 32 bytes"))?;

    let has_state_root = state_root.is_some();
    let state: [u8; 32] = state_root
        .map(|v| {
            v.try_into()
                .map_err(|_| JsError::new("state_root must be 32 bytes"))
        })
        .transpose()?
        .unwrap_or([0u8; 32]);

    let flags = echo_session_proto::TtdrFlags::new(
        has_state_root,
        false, // has_entry_hashes
        false, // has_channel_digests
        false, // has_channel_payload_hash
        echo_session_proto::ReceiptMode::Light,
    );

    let header = echo_session_proto::TtdrHeader {
        version: echo_session_proto::TTDR_VERSION,
        flags,
        schema_hash: schema,
        worldline_id: worldline,
        tick,
        commit_hash: commit,
        state_root: state,
        patch_digest: [0u8; 32], // Not used in Light mode
        emissions_digest: emissions,
        op_emission_index_digest: [0u8; 32], // Not used in Light mode
        parent_count: 0,
        channel_count: 0,
    };

    let frame = echo_session_proto::TtdrFrame {
        header,
        parent_hashes: vec![],
        channel_digests: vec![],
    };

    let bytes =
        echo_session_proto::encode_ttdr_v2(&frame).map_err(|e| JsError::new(&e.to_string()))?;
    Ok(bytes_to_uint8array(&bytes))
}

/// Decode a TTDR v2 frame header.
///
/// # Arguments
///
/// * `bytes` - Encoded TTDR v2 frame
///
/// # Returns
///
/// CBOR-encoded object with header fields.
#[wasm_bindgen]
pub fn decode_ttdr_v2(bytes: &[u8]) -> Result<Uint8Array, JsError> {
    let (frame, _consumed) =
        echo_session_proto::decode_ttdr_v2(bytes).map_err(|e| JsError::new(&e.to_string()))?;

    let result = TtdrHeaderJs {
        schema_hash: frame.header.schema_hash,
        worldline_id: frame.header.worldline_id,
        tick: frame.header.tick,
        commit_hash: frame.header.commit_hash,
        state_root: if frame.header.flags.has_state_root() {
            Some(frame.header.state_root)
        } else {
            None
        },
        emissions_digest: frame.header.emissions_digest,
        receipt_mode: match frame.header.flags.receipt_mode() {
            echo_session_proto::ReceiptMode::Full => "Full",
            echo_session_proto::ReceiptMode::Proof => "Proof",
            echo_session_proto::ReceiptMode::Light => "Light",
            echo_session_proto::ReceiptMode::Reserved => "Reserved",
        }
        .to_string(),
        parent_count: frame.header.parent_count,
    };

    let mut buf = Vec::new();
    ciborium::into_writer(&result, &mut buf)
        .map_err(|e| JsError::new(&format!("CBOR encode error: {e}")))?;

    Ok(bytes_to_uint8array(&buf))
}

// ─── Helper Types ────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ChannelEntry {
    channel: [u8; 32],
    data: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct OpEmissionEntryJs {
    op_id: [u8; 32],
    channels: Vec<[u8; 32]>,
}

#[derive(serde::Deserialize)]
struct PolicyEntry {
    channel: [u8; 32],
    policy: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ViolationJs {
    severity: String,
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_id: Option<[u8; 32]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emission_count: Option<usize>,
}

#[derive(serde::Serialize)]
struct SummaryJs {
    fatal_count: u32,
    error_count: u32,
    warn_count: u32,
    info_count: u32,
    is_green: bool,
}

#[derive(serde::Serialize)]
struct EintFrameJs {
    schema_hash: [u8; 32],
    opcode: u32,
    op_version: u16,
    payload: Vec<u8>,
}

#[derive(serde::Serialize)]
struct TtdrHeaderJs {
    schema_hash: [u8; 32],
    worldline_id: [u8; 32],
    tick: u64,
    commit_hash: [u8; 32],
    #[serde(skip_serializing_if = "Option::is_none")]
    state_root: Option<[u8; 32]>,
    emissions_digest: [u8; 32],
    receipt_mode: String,
    parent_count: u16,
}

// ─── Helper Functions ────────────────────────────────────────────────────────

fn hash_to_uint8array(hash: &[u8; 32]) -> Uint8Array {
    let arr = Uint8Array::new_with_length(32);
    arr.copy_from(hash);
    arr
}

fn bytes_to_uint8array(bytes: &[u8]) -> Uint8Array {
    let arr = Uint8Array::new_with_length(bytes.len() as u32);
    arr.copy_from(bytes);
    arr
}

/// Error parsing a channel policy string.
#[derive(Debug, Clone)]
struct PolicyParseError(String);

impl std::fmt::Display for PolicyParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn parse_policy_inner(
    s: &str,
) -> Result<warp_core::materialization::ChannelPolicy, PolicyParseError> {
    match s {
        "Log" => Ok(warp_core::materialization::ChannelPolicy::Log),
        "StrictSingle" => Ok(warp_core::materialization::ChannelPolicy::StrictSingle),
        s if s.starts_with("Reduce:") => {
            let op_str = &s[7..];
            let op = match op_str {
                "Sum" => warp_core::materialization::ReduceOp::Sum,
                "First" => warp_core::materialization::ReduceOp::First,
                "Last" => warp_core::materialization::ReduceOp::Last,
                "Min" => warp_core::materialization::ReduceOp::Min,
                "Max" => warp_core::materialization::ReduceOp::Max,
                _ => return Err(PolicyParseError(format!("unknown reduce op: {op_str}"))),
            };
            Ok(warp_core::materialization::ChannelPolicy::Reduce(op))
        }
        _ => Err(PolicyParseError(format!("unknown policy: {s}"))),
    }
}

fn parse_policy(s: &str) -> Result<warp_core::materialization::ChannelPolicy, JsError> {
    parse_policy_inner(s).map_err(|e| JsError::new(&e.to_string()))
}

// Tests that use WASM types require target_arch = "wasm32".
// For native testing, we use parse_policy_inner which doesn't depend on js-sys.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_policy_log() {
        let policy = parse_policy_inner("Log").expect("parse Log failed");
        assert!(matches!(
            policy,
            warp_core::materialization::ChannelPolicy::Log
        ));
    }

    #[test]
    fn test_parse_policy_strict_single() {
        let policy = parse_policy_inner("StrictSingle").expect("parse StrictSingle failed");
        assert!(matches!(
            policy,
            warp_core::materialization::ChannelPolicy::StrictSingle
        ));
    }

    #[test]
    fn test_parse_policy_reduce_sum() {
        let policy = parse_policy_inner("Reduce:Sum").expect("parse Reduce:Sum failed");
        assert!(matches!(
            policy,
            warp_core::materialization::ChannelPolicy::Reduce(
                warp_core::materialization::ReduceOp::Sum
            )
        ));
    }

    #[test]
    fn test_parse_policy_reduce_all_ops() {
        // Test all reduce operations
        for (s, _) in [
            ("Reduce:Sum", warp_core::materialization::ReduceOp::Sum),
            ("Reduce:First", warp_core::materialization::ReduceOp::First),
            ("Reduce:Last", warp_core::materialization::ReduceOp::Last),
            ("Reduce:Min", warp_core::materialization::ReduceOp::Min),
            ("Reduce:Max", warp_core::materialization::ReduceOp::Max),
        ] {
            assert!(parse_policy_inner(s).is_ok(), "failed to parse {s}");
        }
    }

    #[test]
    fn test_parse_policy_unknown() {
        assert!(parse_policy_inner("Unknown").is_err());
        assert!(parse_policy_inner("Reduce:Unknown").is_err());
    }
}
