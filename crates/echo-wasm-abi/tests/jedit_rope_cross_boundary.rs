// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::redundant_closure_for_method_calls
)]
//! Cross-boundary fixture proofs for the jedit rope schema LE binary codec.
//!
//! These tests assert that the Rust LE binary codec primitives, when invoked
//! in the same declaration-order layout that `echo-wesley-gen` (Rust emit) and
//! `wesley emit le-binary-typescript` (TS emit) emit for the rope schema,
//! produce byte sequences that are bytewise identical to the literal hex
//! vectors asserted by `jedit/spec/rope-codec.spec.mjs`.
//!
//! The hex literals here MUST stay in lockstep with the literals in that TS
//! spec. If you change one side, change both — they are the cross-boundary
//! contract.
//!
//! This is executable evidence for `docs/adr/0006-universal-little-endian-codec.md`
//! and `docs/spec/abi-golden-vectors.md`.

use echo_wasm_abi::codec::{Reader, Writer};

// ---------------------------------------------------------------------------
// rope-schema-shaped Rust shapes
//
// These mirror the structs that `echo-wesley-gen` would emit for the rope
// schema (without re-running the generator inside this test, which would
// require committing a generated.rs file to this crate). The Encode/Decode
// behaviour matches the generator's output exactly: enums as u32 LE
// discriminants, fields in SDL declaration order, nullables as presence-tagged
// options.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum AnchorBias {
    Left = 0,
    Right = 1,
}

fn encode_anchor_bias(w: &mut Writer, v: AnchorBias) {
    w.write_u32_le(v as u32);
}

fn decode_anchor_bias(r: &mut Reader<'_>) -> AnchorBias {
    match r.read_u32_le().unwrap() {
        0 => AnchorBias::Left,
        1 => AnchorBias::Right,
        d => panic!("invalid AnchorBias discriminant: {d}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum CheckpointKind {
    Initial = 0,
    ManualSave = 1,
    AutoSave = 2,
}

fn encode_checkpoint_kind(w: &mut Writer, v: CheckpointKind) {
    w.write_u32_le(v as u32);
}

fn decode_checkpoint_kind(r: &mut Reader<'_>) -> CheckpointKind {
    match r.read_u32_le().unwrap() {
        0 => CheckpointKind::Initial,
        1 => CheckpointKind::ManualSave,
        2 => CheckpointKind::AutoSave,
        d => panic!("invalid CheckpointKind discriminant: {d}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CreateBufferWorldlineInput {
    buffer_key: String,
    initial_text: Option<String>,
    projection_path: Option<String>,
    create_initial_checkpoint: Option<bool>,
}

fn encode_create_buffer_worldline_input(w: &mut Writer, v: &CreateBufferWorldlineInput) {
    w.write_string(&v.buffer_key, usize::MAX).unwrap();
    w.write_option(v.initial_text.as_deref(), |w, s| {
        w.write_string(s, usize::MAX)
    })
    .unwrap();
    w.write_option(v.projection_path.as_deref(), |w, s| {
        w.write_string(s, usize::MAX)
    })
    .unwrap();
    w.write_option(v.create_initial_checkpoint, |w, b| {
        w.write_bool(b);
        Ok(())
    })
    .unwrap();
}

fn decode_create_buffer_worldline_input(r: &mut Reader<'_>) -> CreateBufferWorldlineInput {
    CreateBufferWorldlineInput {
        buffer_key: r.read_string(usize::MAX).unwrap(),
        initial_text: r.read_option(|r| r.read_string(usize::MAX)).unwrap(),
        projection_path: r.read_option(|r| r.read_string(usize::MAX)).unwrap(),
        create_initial_checkpoint: r.read_option(|r| r.read_bool()).unwrap(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplaceRangeAsTickInput {
    worldline_id: String,
    base_head_id: String,
    start_byte: i32,
    end_byte: i32,
    insert_text: String,
    author: Option<String>,
}

fn encode_replace_range_as_tick_input(w: &mut Writer, v: &ReplaceRangeAsTickInput) {
    w.write_string(&v.worldline_id, usize::MAX).unwrap();
    w.write_string(&v.base_head_id, usize::MAX).unwrap();
    w.write_i32_le(v.start_byte);
    w.write_i32_le(v.end_byte);
    w.write_string(&v.insert_text, usize::MAX).unwrap();
    w.write_option(v.author.as_deref(), |w, s| w.write_string(s, usize::MAX))
        .unwrap();
}

fn decode_replace_range_as_tick_input(r: &mut Reader<'_>) -> ReplaceRangeAsTickInput {
    ReplaceRangeAsTickInput {
        worldline_id: r.read_string(usize::MAX).unwrap(),
        base_head_id: r.read_string(usize::MAX).unwrap(),
        start_byte: r.read_i32_le().unwrap(),
        end_byte: r.read_i32_le().unwrap(),
        insert_text: r.read_string(usize::MAX).unwrap(),
        author: r.read_option(|r| r.read_string(usize::MAX)).unwrap(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CreateCheckpointInput {
    worldline_id: String,
    kind: CheckpointKind,
    label: Option<String>,
}

fn encode_create_checkpoint_input(w: &mut Writer, v: &CreateCheckpointInput) {
    w.write_string(&v.worldline_id, usize::MAX).unwrap();
    encode_checkpoint_kind(w, v.kind);
    w.write_option(v.label.as_deref(), |w, s| w.write_string(s, usize::MAX))
        .unwrap();
}

fn decode_create_checkpoint_input(r: &mut Reader<'_>) -> CreateCheckpointInput {
    CreateCheckpointInput {
        worldline_id: r.read_string(usize::MAX).unwrap(),
        kind: decode_checkpoint_kind(r),
        label: r.read_option(|r| r.read_string(usize::MAX)).unwrap(),
    }
}

// Each operation has a single `input` arg in the rope schema, so Vars is just
// the input wrapped — matching what the generator emits.

fn encode_create_buffer_worldline_vars(input: &CreateBufferWorldlineInput) -> Vec<u8> {
    let mut w = Writer::with_capacity(0);
    encode_create_buffer_worldline_input(&mut w, input);
    w.into_vec()
}

fn encode_replace_range_as_tick_vars(input: &ReplaceRangeAsTickInput) -> Vec<u8> {
    let mut w = Writer::with_capacity(0);
    encode_replace_range_as_tick_input(&mut w, input);
    w.into_vec()
}

fn encode_create_checkpoint_vars(input: &CreateCheckpointInput) -> Vec<u8> {
    let mut w = Writer::with_capacity(0);
    encode_create_checkpoint_input(&mut w, input);
    w.into_vec()
}

// ---------------------------------------------------------------------------
// fixture vectors — must match jedit/spec/rope-codec.spec.mjs exactly
// ---------------------------------------------------------------------------

#[test]
fn anchor_bias_left_encodes_to_u32_le_zero() {
    let mut w = Writer::with_capacity(0);
    encode_anchor_bias(&mut w, AnchorBias::Left);
    assert_eq!(w.into_vec(), [0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn anchor_bias_right_encodes_to_u32_le_one() {
    let mut w = Writer::with_capacity(0);
    encode_anchor_bias(&mut w, AnchorBias::Right);
    assert_eq!(w.into_vec(), [0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn anchor_bias_roundtrips() {
    for variant in [AnchorBias::Left, AnchorBias::Right] {
        let mut w = Writer::with_capacity(0);
        encode_anchor_bias(&mut w, variant);
        let bytes = w.into_vec();
        let mut r = Reader::new(&bytes);
        assert_eq!(decode_anchor_bias(&mut r), variant);
    }
}

#[test]
fn checkpoint_kind_encodes_in_sdl_declaration_order() {
    let cases = [
        (CheckpointKind::Initial, [0x00, 0x00, 0x00, 0x00]),
        (CheckpointKind::ManualSave, [0x01, 0x00, 0x00, 0x00]),
        (CheckpointKind::AutoSave, [0x02, 0x00, 0x00, 0x00]),
    ];
    for (variant, expected) in cases {
        let mut w = Writer::with_capacity(0);
        encode_checkpoint_kind(&mut w, variant);
        assert_eq!(w.into_vec(), expected, "variant {variant:?}");
    }
}

#[test]
fn checkpoint_kind_roundtrips_all_variants() {
    for variant in [
        CheckpointKind::Initial,
        CheckpointKind::ManualSave,
        CheckpointKind::AutoSave,
    ] {
        let mut w = Writer::with_capacity(0);
        encode_checkpoint_kind(&mut w, variant);
        let bytes = w.into_vec();
        let mut r = Reader::new(&bytes);
        assert_eq!(decode_checkpoint_kind(&mut r), variant);
    }
}

#[test]
fn create_buffer_worldline_vars_minimal_matches_ts_spec_bytes() {
    // jedit/spec/rope-codec.spec.mjs: 'minimal: bufferKey only, all optionals null'
    let bytes = encode_create_buffer_worldline_vars(&CreateBufferWorldlineInput {
        buffer_key: "demo.txt".to_string(),
        initial_text: None,
        projection_path: None,
        create_initial_checkpoint: None,
    });

    let expected: Vec<u8> = vec![
        // u32 LE length = 8
        0x08, 0x00, 0x00, 0x00, // "demo.txt"
        0x64, 0x65, 0x6d, 0x6f, 0x2e, 0x74, 0x78, 0x74, // initialText: null
        0x00, // projectionPath: null
        0x00, // createInitialCheckpoint: null
        0x00,
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn create_buffer_worldline_vars_with_present_fields_matches_ts_spec_bytes() {
    // jedit/spec/rope-codec.spec.mjs: 'with initialText and createInitialCheckpoint=true'
    let bytes = encode_create_buffer_worldline_vars(&CreateBufferWorldlineInput {
        buffer_key: "a".to_string(),
        initial_text: Some("hello".to_string()),
        projection_path: None,
        create_initial_checkpoint: Some(true),
    });

    let expected: Vec<u8> = vec![
        // bufferKey: "a"
        0x01, 0x00, 0x00, 0x00, b'a', // initialText present
        0x01, // "hello"
        0x05, 0x00, 0x00, 0x00, b'h', b'e', b'l', b'l', b'o', // projectionPath: null
        0x00, // createInitialCheckpoint present + true
        0x01, 0x01,
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn create_buffer_worldline_vars_roundtrips() {
    let input = CreateBufferWorldlineInput {
        buffer_key: "full.ts".to_string(),
        initial_text: Some("export const x = 1;".to_string()),
        projection_path: Some("/src".to_string()),
        create_initial_checkpoint: Some(false),
    };
    let bytes = encode_create_buffer_worldline_vars(&input);
    let mut r = Reader::new(&bytes);
    let decoded = decode_create_buffer_worldline_input(&mut r);
    assert_eq!(decoded, input);
}

#[test]
fn replace_range_as_tick_vars_start_byte_lands_after_two_string_prefixes() {
    // jedit/spec/rope-codec.spec.mjs: 'startByte and endByte are i32 LE (check wire layout for byte 0)'
    let bytes = encode_replace_range_as_tick_vars(&ReplaceRangeAsTickInput {
        worldline_id: "w".to_string(),
        base_head_id: "b".to_string(),
        start_byte: 0,
        end_byte: 1,
        insert_text: String::new(),
        author: None,
    });

    // worldlineId: u32 LE (1) + "w" => 5 bytes
    // baseHeadId:  u32 LE (1) + "b" => 5 bytes
    // start_byte starts at offset 10.
    let start_byte_offset = (4 + 1) + (4 + 1);
    let read_i32 = |off: usize| -> i32 {
        let mut bytes_at = [0u8; 4];
        bytes_at.copy_from_slice(&bytes[off..off + 4]);
        i32::from_le_bytes(bytes_at)
    };
    assert_eq!(
        read_i32(start_byte_offset),
        0,
        "startByte should encode as 0"
    );
    assert_eq!(
        read_i32(start_byte_offset + 4),
        1,
        "endByte should encode as 1"
    );
}

#[test]
fn replace_range_as_tick_vars_roundtrips_with_optional_author() {
    let input = ReplaceRangeAsTickInput {
        worldline_id: "wl-002".to_string(),
        base_head_id: "hd-002".to_string(),
        start_byte: 10,
        end_byte: 20,
        insert_text: "replacement".to_string(),
        author: Some("james".to_string()),
    };
    let bytes = encode_replace_range_as_tick_vars(&input);
    let mut r = Reader::new(&bytes);
    let decoded = decode_replace_range_as_tick_input(&mut r);
    assert_eq!(decoded, input);
}

#[test]
fn replace_range_as_tick_vars_minimal_matches_pinned_bytes() {
    // Local-roundtrip alone won't catch encoder/decoder drift — pin literal
    // wire bytes the same way create_buffer_worldline does. This vector is
    // also the wire image that any TS-side rope-codec spec must produce for
    // the same input; keep both sides in lockstep.
    let bytes = encode_replace_range_as_tick_vars(&ReplaceRangeAsTickInput {
        worldline_id: "w".to_string(),
        base_head_id: "b".to_string(),
        start_byte: 0,
        end_byte: 1,
        insert_text: String::new(),
        author: None,
    });
    let expected: Vec<u8> = vec![
        // worldlineId: u32 LE length = 1, "w"
        0x01, 0x00, 0x00, 0x00, b'w', // baseHeadId: u32 LE length = 1, "b"
        0x01, 0x00, 0x00, 0x00, b'b', // startByte: i32 LE = 0
        0x00, 0x00, 0x00, 0x00, // endByte: i32 LE = 1
        0x01, 0x00, 0x00, 0x00, // insertText: u32 LE length = 0
        0x00, 0x00, 0x00, 0x00, // author: null
        0x00,
    ];
    assert_eq!(bytes, expected);
    // Decoder must accept the literal vector and produce the original input.
    let mut r = Reader::new(&expected);
    let decoded = decode_replace_range_as_tick_input(&mut r);
    assert_eq!(
        decoded,
        ReplaceRangeAsTickInput {
            worldline_id: "w".to_string(),
            base_head_id: "b".to_string(),
            start_byte: 0,
            end_byte: 1,
            insert_text: String::new(),
            author: None,
        }
    );
}

#[test]
fn replace_range_as_tick_vars_with_author_matches_pinned_bytes() {
    let bytes = encode_replace_range_as_tick_vars(&ReplaceRangeAsTickInput {
        worldline_id: "wl-002".to_string(),
        base_head_id: "hd-002".to_string(),
        start_byte: 10,
        end_byte: 20,
        insert_text: "replacement".to_string(),
        author: Some("james".to_string()),
    });
    let expected: Vec<u8> = vec![
        // worldlineId: len=6, "wl-002"
        0x06, 0x00, 0x00, 0x00, b'w', b'l', b'-', b'0', b'0', b'2',
        // baseHeadId: len=6, "hd-002"
        0x06, 0x00, 0x00, 0x00, b'h', b'd', b'-', b'0', b'0', b'2', // startByte: 10
        0x0a, 0x00, 0x00, 0x00, // endByte: 20
        0x14, 0x00, 0x00, 0x00, // insertText: len=11, "replacement"
        0x0b, 0x00, 0x00, 0x00, b'r', b'e', b'p', b'l', b'a', b'c', b'e', b'm', b'e', b'n', b't',
        // author: present + len=5 + "james"
        0x01, 0x05, 0x00, 0x00, 0x00, b'j', b'a', b'm', b'e', b's',
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn create_checkpoint_vars_roundtrips_with_manual_save_and_label() {
    let input = CreateCheckpointInput {
        worldline_id: "wl-001".to_string(),
        kind: CheckpointKind::ManualSave,
        label: Some("before refactor".to_string()),
    };
    let bytes = encode_create_checkpoint_vars(&input);
    let mut r = Reader::new(&bytes);
    let decoded = decode_create_checkpoint_input(&mut r);
    assert_eq!(decoded, input);
}

#[test]
fn create_checkpoint_vars_roundtrips_with_auto_save_and_no_label() {
    let input = CreateCheckpointInput {
        worldline_id: "wl-001".to_string(),
        kind: CheckpointKind::AutoSave,
        label: None,
    };
    let bytes = encode_create_checkpoint_vars(&input);
    let mut r = Reader::new(&bytes);
    let decoded = decode_create_checkpoint_input(&mut r);
    assert_eq!(decoded, input);
}

#[test]
fn create_checkpoint_vars_manual_save_with_label_matches_pinned_bytes() {
    let bytes = encode_create_checkpoint_vars(&CreateCheckpointInput {
        worldline_id: "wl-001".to_string(),
        kind: CheckpointKind::ManualSave,
        label: Some("before refactor".to_string()),
    });
    let expected: Vec<u8> = vec![
        // worldlineId: len=6, "wl-001"
        0x06, 0x00, 0x00, 0x00, b'w', b'l', b'-', b'0', b'0', b'1',
        // kind: MANUAL_SAVE = u32 LE 1
        0x01, 0x00, 0x00, 0x00, // label: present + len=15 + "before refactor"
        0x01, 0x0f, 0x00, 0x00, 0x00, b'b', b'e', b'f', b'o', b'r', b'e', b' ', b'r', b'e', b'f',
        b'a', b'c', b't', b'o', b'r',
    ];
    assert_eq!(bytes, expected);
    let mut r = Reader::new(&expected);
    assert_eq!(
        decode_create_checkpoint_input(&mut r),
        CreateCheckpointInput {
            worldline_id: "wl-001".to_string(),
            kind: CheckpointKind::ManualSave,
            label: Some("before refactor".to_string()),
        }
    );
}

#[test]
fn create_checkpoint_vars_auto_save_no_label_matches_pinned_bytes() {
    let bytes = encode_create_checkpoint_vars(&CreateCheckpointInput {
        worldline_id: "wl-001".to_string(),
        kind: CheckpointKind::AutoSave,
        label: None,
    });
    let expected: Vec<u8> = vec![
        // worldlineId: len=6, "wl-001"
        0x06, 0x00, 0x00, 0x00, b'w', b'l', b'-', b'0', b'0', b'1',
        // kind: AUTO_SAVE = u32 LE 2
        0x02, 0x00, 0x00, 0x00, // label: null
        0x00,
    ];
    assert_eq!(bytes, expected);
    let mut r = Reader::new(&expected);
    assert_eq!(
        decode_create_checkpoint_input(&mut r),
        CreateCheckpointInput {
            worldline_id: "wl-001".to_string(),
            kind: CheckpointKind::AutoSave,
            label: None,
        }
    );
}
