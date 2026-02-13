// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    dead_code,
    clippy::derivable_impls,
    non_snake_case,
    non_camel_case_types,
    missing_docs
)]
use serde::{Deserialize, Serialize};
/// SHA256 hash of the source TTD schema.
pub const SCHEMA_SHA256: &str = "d55d6000b43562e7be04702cdd4335452d1eb6df1f0fbea924e4c6434fff2871";
/// Timestamp when this code was generated.
pub const GENERATED_AT: &str = "2026-02-13T06:52:28.771Z";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CursorRole {
    WRITER,
    READER,
}
impl Default for CursorRole {
    fn default() -> Self {
        Self::WRITER
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlaybackMode {
    PAUSED,
    PLAY,
    STEP_FORWARD,
    STEP_BACK,
    SEEK,
}
impl Default for PlaybackMode {
    fn default() -> Self {
        Self::PAUSED
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeekResult {
    OK,
    OUT_OF_RANGE,
    DIVERGED,
}
impl Default for SeekResult {
    fn default() -> Self {
        Self::OK
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceStatus {
    COMPLIANT,
    VIOLATION,
    PENDING,
}
impl Default for ComplianceStatus {
    fn default() -> Self {
        Self::COMPLIANT
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationSeverity {
    INFO,
    WARN,
    ERROR,
    FATAL,
}
impl Default for ViolationSeverity {
    fn default() -> Self {
        Self::INFO
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepResultKind {
    NO_OP,
    ADVANCED,
    SEEKED,
    REACHED_FRONTIER,
}
impl Default for StepResultKind {
    fn default() -> Self {
        Self::NO_OP
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorMoved {
    pub sessionId: String,
    pub cursorId: String,
    pub worldlineId: String,
    pub warpId: String,
    pub tick: i32,
    pub commitHash: String,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekCompleted {
    pub sessionId: String,
    pub cursorId: String,
    pub fromTick: i32,
    pub toTick: i32,
    pub result: SeekResult,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekFailed {
    pub sessionId: String,
    pub cursorId: String,
    pub targetTick: i32,
    pub reason: String,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationDetected {
    pub sessionId: String,
    pub cursorId: String,
    pub tick: i32,
    pub violation: Violation,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceUpdate {
    pub sessionId: String,
    pub cursorId: String,
    pub tick: i32,
    pub status: ComplianceStatus,
    pub violationCount: i32,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStarted {
    pub sessionId: String,
    pub worldlineId: String,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEnded {
    pub sessionId: String,
    pub reason: Option<String>,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorCreated {
    pub sessionId: String,
    pub cursorId: String,
    pub role: CursorRole,
    pub initialTick: i32,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorDestroyed {
    pub sessionId: String,
    pub cursorId: String,
    pub finalTick: i32,
    pub timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub code: String,
    pub severity: ViolationSeverity,
    pub message: String,
    pub channelId: Option<String>,
    pub tick: Option<i32>,
    pub emissionCount: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthFrame {
    pub sessionId: String,
    pub cursorId: String,
    pub worldlineId: String,
    pub warpId: String,
    pub tick: i32,
    pub commitHash: String,
    pub channel: String,
    pub valueHash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObligationState {
    pub id: String,
    pub description: String,
    pub deadlineTick: i32,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub result: StepResultKind,
    pub tick: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub worldlineId: String,
    pub tick: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceModel {
    pub isGreen: bool,
    pub violations: Vec<Violation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    pub id: String,
    pub description: String,
    pub deadlineTick: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObligationReport {
    pub pending: Vec<Obligation>,
    pub satisfied: Vec<Obligation>,
    pub violated: Vec<Obligation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorState {
    pub sessionId: String,
    pub cursorId: String,
    pub worldlineId: String,
    pub warpId: String,
    pub tick: i32,
    pub commitHash: String,
    pub role: CursorRole,
    pub mode: PlaybackMode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtdSystem {
    pub _placeholder: Option<bool>,
}
/// Channel metadata.
#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub name: &'static str,
    pub version: u16,
    pub event_types: &'static [&'static str],
    pub ordered: bool,
    pub persistent: bool,
}
pub const CHANNEL_TTD_HEAD: &str = "ttd.head";
pub const CHANNEL_TTD_ERRORS: &str = "ttd.errors";
pub const CHANNEL_TTD_COMPLIANCE: &str = "ttd.compliance";
pub const CHANNEL_TTD_SESSION: &str = "ttd.session";
pub const CHANNELS: &[ChannelInfo] = &[
    ChannelInfo {
        name: "ttd.head",
        version: 1u16,
        event_types: &["CursorMoved", "SeekCompleted"],
        ordered: true,
        persistent: false,
    },
    ChannelInfo {
        name: "ttd.errors",
        version: 1u16,
        event_types: &["ViolationDetected", "SeekFailed"],
        ordered: true,
        persistent: true,
    },
    ChannelInfo {
        name: "ttd.compliance",
        version: 1u16,
        event_types: &["ComplianceUpdate"],
        ordered: true,
        persistent: false,
    },
    ChannelInfo {
        name: "ttd.session",
        version: 1u16,
        event_types: &[
            "SessionStarted",
            "SessionEnded",
            "CursorCreated",
            "CursorDestroyed",
        ],
        ordered: true,
        persistent: false,
    },
];
pub fn channel_by_name(name: &str) -> Option<&'static ChannelInfo> {
    CHANNELS.iter().find(|c| c.name == name)
}
/// Op metadata.
#[derive(Debug, Clone)]
pub struct OpInfo {
    pub name: &'static str,
    pub op_id: u32,
    pub result_type: &'static str,
    pub idempotent: bool,
    pub readonly: bool,
    pub arg_count: usize,
}
/// Argument metadata for ops.
#[derive(Debug, Clone)]
pub struct ArgInfo {
    pub name: &'static str,
    pub type_name: &'static str,
    pub required: bool,
    pub list: bool,
}
pub const OP_STEPFORWARD: u32 = 116350618u32;
pub const OP_GETCURSOR: u32 = 639452774u32;
pub const OP_DESTROYCURSOR: u32 = 856261846u32;
pub const OP_GETPENDINGOBLIGATIONS: u32 = 1131058299u32;
pub const OP_PLAY: u32 = 1379835894u32;
pub const OP_STEPBACK: u32 = 1394286172u32;
pub const OP_LISTCURSORS: u32 = 1482350986u32;
pub const OP_SEEK: u32 = 1824277946u32;
pub const OP_PAUSE: u32 = 2364740775u32;
pub const OP_ENDSESSION: u32 = 2437620378u32;
pub const OP_GETVIOLATIONS: u32 = 3281714801u32;
pub const OP_CREATECURSOR: u32 = 3581082943u32;
pub const OP_GETCOMPLIANCESTATUS: u32 = 3843805192u32;
pub const OP_STARTSESSION: u32 = 4193224715u32;
///Arguments for the `stepForward` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepForwardArgs {
    pub sessionId: String,
    pub cursorId: String,
}
///Arguments for the `getCursor` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCursorArgs {
    pub sessionId: String,
    pub cursorId: String,
}
///Arguments for the `destroyCursor` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyCursorArgs {
    pub sessionId: String,
    pub cursorId: String,
}
///Arguments for the `getPendingObligations` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPendingObligationsArgs {
    pub sessionId: String,
    pub cursorId: String,
}
///Arguments for the `play` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayArgs {
    pub sessionId: String,
    pub cursorId: String,
}
///Arguments for the `stepBack` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepBackArgs {
    pub sessionId: String,
    pub cursorId: String,
}
///Arguments for the `listCursors` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListCursorsArgs {
    pub sessionId: String,
}
///Arguments for the `seek` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekArgs {
    pub sessionId: String,
    pub cursorId: String,
    pub targetTick: i32,
}
///Arguments for the `pause` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseArgs {
    pub sessionId: String,
    pub cursorId: String,
}
///Arguments for the `endSession` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndSessionArgs {
    pub sessionId: String,
    pub reason: Option<String>,
}
///Arguments for the `getViolations` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetViolationsArgs {
    pub sessionId: String,
    pub fromTick: i32,
    pub toTick: i32,
}
///Arguments for the `createCursor` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCursorArgs {
    pub sessionId: String,
    pub role: CursorRole,
    pub initialTick: Option<i32>,
}
///Arguments for the `getComplianceStatus` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetComplianceStatusArgs {
    pub sessionId: String,
    pub cursorId: String,
    pub tick: i32,
}
///Arguments for the `startSession` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartSessionArgs {
    pub worldlineId: String,
}
pub const STEPFORWARD_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
];
pub const GETCURSOR_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
];
pub const DESTROYCURSOR_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
];
pub const GETPENDINGOBLIGATIONS_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
];
pub const PLAY_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
];
pub const STEPBACK_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
];
pub const LISTCURSORS_ARGS: &[ArgInfo] = &[ArgInfo {
    name: "sessionId",
    type_name: "Hash",
    required: true,
    list: false,
}];
pub const SEEK_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "targetTick",
        type_name: "Int",
        required: true,
        list: false,
    },
];
pub const PAUSE_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
];
pub const ENDSESSION_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "reason",
        type_name: "String",
        required: false,
        list: false,
    },
];
pub const GETVIOLATIONS_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "fromTick",
        type_name: "Int",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "toTick",
        type_name: "Int",
        required: true,
        list: false,
    },
];
pub const CREATECURSOR_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "role",
        type_name: "CursorRole",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "initialTick",
        type_name: "Int",
        required: false,
        list: false,
    },
];
pub const GETCOMPLIANCESTATUS_ARGS: &[ArgInfo] = &[
    ArgInfo {
        name: "sessionId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "cursorId",
        type_name: "Hash",
        required: true,
        list: false,
    },
    ArgInfo {
        name: "tick",
        type_name: "Int",
        required: true,
        list: false,
    },
];
pub const STARTSESSION_ARGS: &[ArgInfo] = &[ArgInfo {
    name: "worldlineId",
    type_name: "Hash",
    required: true,
    list: false,
}];
pub const OPS: &[OpInfo] = &[
    OpInfo {
        name: "stepForward",
        op_id: 116350618u32,
        result_type: "CursorMoved",
        idempotent: false,
        readonly: false,
        arg_count: 2usize,
    },
    OpInfo {
        name: "getCursor",
        op_id: 639452774u32,
        result_type: "CursorState",
        idempotent: false,
        readonly: true,
        arg_count: 2usize,
    },
    OpInfo {
        name: "destroyCursor",
        op_id: 856261846u32,
        result_type: "CursorDestroyed",
        idempotent: true,
        readonly: false,
        arg_count: 2usize,
    },
    OpInfo {
        name: "getPendingObligations",
        op_id: 1131058299u32,
        result_type: "ObligationState",
        idempotent: false,
        readonly: true,
        arg_count: 2usize,
    },
    OpInfo {
        name: "play",
        op_id: 1379835894u32,
        result_type: "CursorMoved",
        idempotent: true,
        readonly: false,
        arg_count: 2usize,
    },
    OpInfo {
        name: "stepBack",
        op_id: 1394286172u32,
        result_type: "CursorMoved",
        idempotent: false,
        readonly: false,
        arg_count: 2usize,
    },
    OpInfo {
        name: "listCursors",
        op_id: 1482350986u32,
        result_type: "CursorState",
        idempotent: false,
        readonly: true,
        arg_count: 1usize,
    },
    OpInfo {
        name: "seek",
        op_id: 1824277946u32,
        result_type: "SeekCompleted",
        idempotent: false,
        readonly: false,
        arg_count: 3usize,
    },
    OpInfo {
        name: "pause",
        op_id: 2364740775u32,
        result_type: "CursorMoved",
        idempotent: true,
        readonly: false,
        arg_count: 2usize,
    },
    OpInfo {
        name: "endSession",
        op_id: 2437620378u32,
        result_type: "SessionEnded",
        idempotent: true,
        readonly: false,
        arg_count: 2usize,
    },
    OpInfo {
        name: "getViolations",
        op_id: 3281714801u32,
        result_type: "Violation",
        idempotent: false,
        readonly: true,
        arg_count: 3usize,
    },
    OpInfo {
        name: "createCursor",
        op_id: 3581082943u32,
        result_type: "CursorCreated",
        idempotent: false,
        readonly: false,
        arg_count: 3usize,
    },
    OpInfo {
        name: "getComplianceStatus",
        op_id: 3843805192u32,
        result_type: "ComplianceUpdate",
        idempotent: false,
        readonly: true,
        arg_count: 3usize,
    },
    OpInfo {
        name: "startSession",
        op_id: 4193224715u32,
        result_type: "SessionStarted",
        idempotent: false,
        readonly: false,
        arg_count: 1usize,
    },
];
pub fn op_by_id(op_id: u32) -> Option<&'static OpInfo> {
    OPS.iter().find(|o| o.op_id == op_id)
}
pub fn op_by_name(name: &str) -> Option<&'static OpInfo> {
    OPS.iter().find(|o| o.name == name)
}
/// Footprint metadata (read/write sets).
#[derive(Debug, Clone)]
pub struct FootprintInfo {
    pub op_name: &'static str,
    pub reads: &'static [&'static str],
    pub writes: &'static [&'static str],
    pub creates: &'static [&'static str],
    pub deletes: &'static [&'static str],
}
pub const FOOTPRINTS: &[FootprintInfo] = &[
    FootprintInfo {
        op_name: "seek",
        reads: &["CursorState"],
        writes: &["CursorState"],
        creates: &[],
        deletes: &[],
    },
    FootprintInfo {
        op_name: "stepForward",
        reads: &["CursorState"],
        writes: &["CursorState"],
        creates: &[],
        deletes: &[],
    },
    FootprintInfo {
        op_name: "stepBack",
        reads: &["CursorState"],
        writes: &["CursorState"],
        creates: &[],
        deletes: &[],
    },
    FootprintInfo {
        op_name: "play",
        reads: &[],
        writes: &["CursorState"],
        creates: &[],
        deletes: &[],
    },
    FootprintInfo {
        op_name: "pause",
        reads: &[],
        writes: &["CursorState"],
        creates: &[],
        deletes: &[],
    },
    FootprintInfo {
        op_name: "createCursor",
        reads: &[],
        writes: &[],
        creates: &["CursorState"],
        deletes: &[],
    },
    FootprintInfo {
        op_name: "destroyCursor",
        reads: &[],
        writes: &[],
        creates: &[],
        deletes: &["CursorState"],
    },
    FootprintInfo {
        op_name: "getCursor",
        reads: &["CursorState"],
        writes: &[],
        creates: &[],
        deletes: &[],
    },
    FootprintInfo {
        op_name: "listCursors",
        reads: &["CursorState"],
        writes: &[],
        creates: &[],
        deletes: &[],
    },
];
pub fn footprint_for_op(op_name: &str) -> Option<&'static FootprintInfo> {
    FOOTPRINTS.iter().find(|f| f.op_name == op_name)
}
/// Registry entry (type ID mapping).
#[derive(Debug, Clone)]
pub struct RegistryEntryInfo {
    pub type_name: &'static str,
    pub id: u32,
    pub deprecated: bool,
}
pub const REGISTRY: &[RegistryEntryInfo] = &[
    RegistryEntryInfo {
        type_name: "CursorMoved",
        id: 1u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "SeekCompleted",
        id: 2u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "SeekFailed",
        id: 3u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "ViolationDetected",
        id: 4u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "ComplianceUpdate",
        id: 5u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "SessionStarted",
        id: 6u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "SessionEnded",
        id: 7u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "CursorCreated",
        id: 8u32,
        deprecated: false,
    },
    RegistryEntryInfo {
        type_name: "CursorDestroyed",
        id: 9u32,
        deprecated: false,
    },
];
pub fn registry_id_for_type(type_name: &str) -> Option<u32> {
    REGISTRY
        .iter()
        .find(|e| e.type_name == type_name)
        .map(|e| e.id)
}
pub fn registry_type_for_id(id: u32) -> Option<&'static str> {
    REGISTRY.iter().find(|e| e.id == id).map(|e| e.type_name)
}
/// Invariant metadata (law compiler stubs for v2).
#[derive(Debug, Clone)]
pub struct InvariantInfo {
    pub name: &'static str,
    pub expr: &'static str,
    pub severity: &'static str,
}
pub const INVARIANTS: &[InvariantInfo] = &[
    InvariantInfo {
        name: "tick_non_negative",
        expr: "forall c in CursorState: c.tick >= 0",
        severity: "error",
    },
    InvariantInfo {
        name: "seek_emits_head",
        expr: "op.seek.mustEmit(CursorMoved).within(100)",
        severity: "error",
    },
    InvariantInfo {
        name: "step_emits_head",
        expr: "op.stepForward.mustEmit(CursorMoved).within(50)",
        severity: "error",
    },
    InvariantInfo {
        name: "session_has_cursor",
        expr: "forall s in SessionStarted: exists c in CursorState: c.sessionId == s.sessionId",
        severity: "warning",
    },
];
pub fn invariant_by_name(name: &str) -> Option<&'static InvariantInfo> {
    INVARIANTS.iter().find(|i| i.name == name)
}
/// Emission declaration metadata.
#[derive(Debug, Clone)]
pub struct EmissionInfo {
    pub channel: &'static str,
    pub event: Option<&'static str>,
    pub op_name: &'static str,
    pub condition: Option<&'static str>,
    pub within_ms: Option<u64>,
}
pub const EMISSIONS: &[EmissionInfo] = &[
    EmissionInfo {
        channel: "ttd.head",
        event: Some("CursorMoved"),
        op_name: "seek",
        condition: None,
        within_ms: Some(100u64),
    },
    EmissionInfo {
        channel: "ttd.head",
        event: Some("CursorMoved"),
        op_name: "stepForward",
        condition: None,
        within_ms: Some(50u64),
    },
    EmissionInfo {
        channel: "ttd.head",
        event: Some("CursorMoved"),
        op_name: "stepBack",
        condition: None,
        within_ms: Some(50u64),
    },
    EmissionInfo {
        channel: "ttd.session",
        event: Some("CursorCreated"),
        op_name: "createCursor",
        condition: None,
        within_ms: None,
    },
    EmissionInfo {
        channel: "ttd.session",
        event: Some("CursorDestroyed"),
        op_name: "destroyCursor",
        condition: None,
        within_ms: None,
    },
    EmissionInfo {
        channel: "ttd.session",
        event: Some("SessionStarted"),
        op_name: "startSession",
        condition: None,
        within_ms: None,
    },
    EmissionInfo {
        channel: "ttd.session",
        event: Some("SessionEnded"),
        op_name: "endSession",
        condition: None,
        within_ms: None,
    },
];
pub fn emissions_for_op(op_name: &str) -> Vec<&'static EmissionInfo> {
    EMISSIONS.iter().filter(|e| e.op_name == op_name).collect()
}
pub fn emissions_for_channel(channel: &str) -> Vec<&'static EmissionInfo> {
    EMISSIONS.iter().filter(|e| e.channel == channel).collect()
}
