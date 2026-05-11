// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Consumer-side proof that Echo can compile against Wesley's bounded rewrite API.

use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const GENERATED_REWRITE_API: &str = include_str!("fixtures/rewrite_api.generated.rs");
const GENERATED_STRUCTURED_REWRITE_API: &str =
    include_str!("fixtures/rewrite_api_structured.generated.rs");

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_rust(source: &str) -> std::process::Output {
    let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "echo-wesley-gen-rewrite-proof-{}-{}-{}",
        std::process::id(),
        nanos,
        unique
    ));
    create_dir_all(&dir).expect("failed to create temp dir");

    let src_path: PathBuf = dir.join("proof.rs");
    let out_path: PathBuf = dir.join("proof.rlib");
    write(&src_path, source).expect("failed to write proof source");

    let output = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            src_path.to_str().expect("non-utf8 source path"),
            "-o",
            out_path.to_str().expect("non-utf8 output path"),
        ])
        .output()
        .expect("failed to invoke rustc");

    remove_dir_all(&dir).expect("failed to remove temp dir");
    output
}

#[test]
fn fixture_exposes_only_declared_counter_capabilities() {
    assert!(GENERATED_REWRITE_API.contains("pub trait ReadCounter"));
    assert!(GENERATED_REWRITE_API.contains("pub trait WriteCounter"));
    assert!(GENERATED_REWRITE_API.contains("pub trait IncrementCounterContext"));
    assert!(GENERATED_REWRITE_API.contains("pub trait IncrementCounterRewrite"));
    assert!(!GENERATED_REWRITE_API.contains("DeleteCounter"));
}

#[test]
fn valid_echo_side_implementation_compiles() {
    let compile = compile_rust(&format!(
        r#"
{GENERATED_REWRITE_API}

#[derive(Debug, Clone, PartialEq)]
pub struct Counter {{
    pub id: String,
    pub value: i64,
}}

pub struct CounterStore {{
    pub counter: Counter,
}}

impl ReadCounter for CounterStore {{
    fn read_counter(&self) -> &Counter {{
        &self.counter
    }}
}}

impl WriteCounter for CounterStore {{
    fn write_counter(&mut self, value: Counter) {{
        self.counter = value;
    }}
}}

pub struct Increment;

impl IncrementCounterRewrite for Increment {{
    type Error = ();

    fn apply<C>(&self, ctx: &mut C, _args: IncrementCounterArgs) -> Result<Counter, Self::Error>
    where
        C: IncrementCounterContext,
    {{
        let mut next = ctx.read_counter().clone();
        next.value += 1;
        ctx.write_counter(next.clone());
        Ok(next)
    }}
}}
"#
    ));

    assert!(
        compile.status.success(),
        "rustc failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
}

#[test]
fn dishonest_echo_side_implementation_fails_to_compile() {
    let compile = compile_rust(&format!(
        r#"
{GENERATED_REWRITE_API}

#[derive(Debug, Clone, PartialEq)]
pub struct Counter {{
    pub id: String,
    pub value: i64,
}}

pub struct CounterStore {{
    pub counter: Counter,
}}

impl ReadCounter for CounterStore {{
    fn read_counter(&self) -> &Counter {{
        &self.counter
    }}
}}

impl WriteCounter for CounterStore {{
    fn write_counter(&mut self, value: Counter) {{
        self.counter = value;
    }}
}}

pub struct Increment;

impl IncrementCounterRewrite for Increment {{
    type Error = ();

    fn apply<C>(&self, ctx: &mut C, _args: IncrementCounterArgs) -> Result<Counter, Self::Error>
    where
        C: IncrementCounterContext,
    {{
        ctx.delete_counter();
        Ok(ctx.read_counter().clone())
    }}
}}
"#
    ));

    assert!(
        !compile.status.success(),
        "expected rustc failure, got success"
    );
    let stderr = String::from_utf8_lossy(&compile.stderr);
    assert!(
        stderr.contains("delete_counter"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn structured_fixture_exposes_only_declared_replace_range_capabilities() {
    assert!(
        GENERATED_STRUCTURED_REWRITE_API.contains("pub trait ReplaceRangeAsTickReadWorldlineSlot")
    );
    assert!(GENERATED_STRUCTURED_REWRITE_API
        .contains("pub trait ReplaceRangeAsTickReadTouchedRopeClosure"));
    assert!(
        GENERATED_STRUCTURED_REWRITE_API.contains("pub trait ReplaceRangeAsTickCreateNextHeadSlot")
    );
    assert!(GENERATED_STRUCTURED_REWRITE_API
        .contains("pub trait ReplaceRangeAsTickUpdateWorldlineCanonicalHead"));
    assert!(GENERATED_STRUCTURED_REWRITE_API.contains(
        "// ReplaceRangeAsTick forbidden surfaces: AstState, Diagnostics, GitWitness, UiState"
    ));
    assert!(!GENERATED_STRUCTURED_REWRITE_API.contains("read_ast_state_slot"));
}

#[test]
fn valid_structured_echo_side_implementation_compiles() {
    let compile = compile_rust(&format!(
        r#"
{GENERATED_STRUCTURED_REWRITE_API}

#[derive(Debug, Clone, PartialEq)]
pub struct BufferWorldline {{
    pub worldline_id: String,
    pub canonical_head_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeHead {{
    pub head_id: String,
    pub worldline_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeBranch {{
    pub branch_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeLeaf {{
    pub leaf_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlob {{
    pub blob_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {{
    pub anchor_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct Tick {{
    pub tick_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct TickReceipt {{
    pub receipt_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {{
    pub checkpoint_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateBufferWorldlineInput {{
    pub buffer_key: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateBufferWorldlineResult {{
    pub worldline_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateCheckpointInput {{
    pub worldline_id: String,
    pub kind: String,
    pub label: Option<String>,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateCheckpointResult {{
    pub checkpoint_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceRangeAsTickInput {{
    pub worldline_id: String,
    pub base_head_id: String,
    pub start_byte: i64,
    pub end_byte: i64,
    pub insert_text: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceRangeAsTickResult {{
    pub worldline_id: String,
    pub next_head_id: String,
    pub tick_id: String,
    pub receipt_id: String,
}}

pub struct ReplaceRangeStore {{
    pub worldline: BufferWorldline,
    pub base_head: RopeHead,
    pub branch: RopeBranch,
    pub leaf: RopeLeaf,
    pub blob: TextBlob,
    pub anchor: Anchor,
    pub next_head: RopeHead,
    pub tick: Tick,
    pub receipt: TickReceipt,
}}

impl ReplaceRangeAsTickReadWorldlineSlot for ReplaceRangeStore {{
    fn read_worldline_slot(&self) -> &BufferWorldline {{
        &self.worldline
    }}
}}

impl ReplaceRangeAsTickWriteWorldlineSlot for ReplaceRangeStore {{
    fn write_worldline_slot(&mut self, value: BufferWorldline) {{
        self.worldline = value;
    }}
}}

impl ReplaceRangeAsTickReadBaseHeadSlot for ReplaceRangeStore {{
    fn read_base_head_slot(&self) -> &RopeHead {{
        &self.base_head
    }}
}}

impl ReplaceRangeAsTickReadTouchedRopeClosure for ReplaceRangeStore {{
    fn read_touched_rope_closure(&self) -> Vec<ReplaceRangeAsTickTouchedRopeClosureItemRef<'_>> {{
        vec![
            ReplaceRangeAsTickTouchedRopeClosureItemRef::RopeBranch(&self.branch),
            ReplaceRangeAsTickTouchedRopeClosureItemRef::RopeLeaf(&self.leaf),
            ReplaceRangeAsTickTouchedRopeClosureItemRef::TextBlob(&self.blob),
        ]
    }}
}}

impl ReplaceRangeAsTickReadAffectedAnchorsClosure for ReplaceRangeStore {{
    fn read_affected_anchors_closure(
        &self,
    ) -> Vec<ReplaceRangeAsTickAffectedAnchorsClosureItemRef<'_>> {{
        vec![ReplaceRangeAsTickAffectedAnchorsClosureItemRef::Anchor(&self.anchor)]
    }}
}}

impl ReplaceRangeAsTickCreateNewBlobSlot for ReplaceRangeStore {{
    fn create_new_blob_slot(&mut self, value: TextBlob) -> TextBlob {{
        self.blob = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateNewLeavesSlot for ReplaceRangeStore {{
    fn create_new_leaves_slot(&mut self, value: RopeLeaf) -> RopeLeaf {{
        self.leaf = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateNewBranchesSlot for ReplaceRangeStore {{
    fn create_new_branches_slot(&mut self, value: RopeBranch) -> RopeBranch {{
        self.branch = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateNextHeadSlot for ReplaceRangeStore {{
    fn create_next_head_slot(&mut self, value: RopeHead) -> RopeHead {{
        self.next_head = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateTickSlot for ReplaceRangeStore {{
    fn create_tick_slot(&mut self, value: Tick) -> Tick {{
        self.tick = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateReceiptSlot for ReplaceRangeStore {{
    fn create_receipt_slot(&mut self, value: TickReceipt) -> TickReceipt {{
        self.receipt = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickUpdateWorldlineCanonicalHead for ReplaceRangeStore {{
    fn update_worldline_canonical_head(&mut self, value: String) {{
        self.worldline.canonical_head_id = value;
    }}
}}

pub struct ReplaceRange;

impl ReplaceRangeAsTickRewrite for ReplaceRange {{
    type Error = ();

    fn apply<C>(
        &self,
        ctx: &mut C,
        args: ReplaceRangeAsTickArgs,
    ) -> Result<ReplaceRangeAsTickResult, Self::Error>
    where
        C: ReplaceRangeAsTickContext,
    {{
        let _ = args.input.start_byte;
        let _ = ctx.read_touched_rope_closure();
        let _ = ctx.read_affected_anchors_closure();
        let next_head = ctx.create_next_head_slot(RopeHead {{
            head_id: "head-2".to_owned(),
            worldline_id: ctx.read_worldline_slot().worldline_id.clone(),
        }});
        let tick = ctx.create_tick_slot(Tick {{
            tick_id: "tick-1".to_owned(),
        }});
        let receipt = ctx.create_receipt_slot(TickReceipt {{
            receipt_id: "receipt-1".to_owned(),
        }});
        ctx.update_worldline_canonical_head(next_head.head_id.clone());
        Ok(ReplaceRangeAsTickResult {{
            worldline_id: ctx.read_worldline_slot().worldline_id.clone(),
            next_head_id: next_head.head_id,
            tick_id: tick.tick_id,
            receipt_id: receipt.receipt_id,
        }})
    }}
}}
"#
    ));

    assert!(
        compile.status.success(),
        "rustc failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );
}

#[test]
fn dishonest_structured_echo_side_implementation_fails_to_compile() {
    let compile = compile_rust(&format!(
        r#"
{GENERATED_STRUCTURED_REWRITE_API}

#[derive(Debug, Clone, PartialEq)]
pub struct BufferWorldline {{
    pub worldline_id: String,
    pub canonical_head_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeHead {{
    pub head_id: String,
    pub worldline_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeBranch {{
    pub branch_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeLeaf {{
    pub leaf_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlob {{
    pub blob_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {{
    pub anchor_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct Tick {{
    pub tick_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct TickReceipt {{
    pub receipt_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {{
    pub checkpoint_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateBufferWorldlineInput {{
    pub buffer_key: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateBufferWorldlineResult {{
    pub worldline_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateCheckpointInput {{
    pub worldline_id: String,
    pub kind: String,
    pub label: Option<String>,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateCheckpointResult {{
    pub checkpoint_id: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceRangeAsTickInput {{
    pub worldline_id: String,
    pub base_head_id: String,
    pub start_byte: i64,
    pub end_byte: i64,
    pub insert_text: String,
}}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceRangeAsTickResult {{
    pub worldline_id: String,
    pub next_head_id: String,
    pub tick_id: String,
    pub receipt_id: String,
}}

pub struct ReplaceRangeStore {{
    pub worldline: BufferWorldline,
    pub base_head: RopeHead,
    pub branch: RopeBranch,
    pub leaf: RopeLeaf,
    pub blob: TextBlob,
    pub anchor: Anchor,
    pub next_head: RopeHead,
    pub tick: Tick,
    pub receipt: TickReceipt,
}}

impl ReplaceRangeAsTickReadWorldlineSlot for ReplaceRangeStore {{
    fn read_worldline_slot(&self) -> &BufferWorldline {{
        &self.worldline
    }}
}}

impl ReplaceRangeAsTickWriteWorldlineSlot for ReplaceRangeStore {{
    fn write_worldline_slot(&mut self, value: BufferWorldline) {{
        self.worldline = value;
    }}
}}

impl ReplaceRangeAsTickReadBaseHeadSlot for ReplaceRangeStore {{
    fn read_base_head_slot(&self) -> &RopeHead {{
        &self.base_head
    }}
}}

impl ReplaceRangeAsTickReadTouchedRopeClosure for ReplaceRangeStore {{
    fn read_touched_rope_closure(&self) -> Vec<ReplaceRangeAsTickTouchedRopeClosureItemRef<'_>> {{
        vec![
            ReplaceRangeAsTickTouchedRopeClosureItemRef::RopeBranch(&self.branch),
            ReplaceRangeAsTickTouchedRopeClosureItemRef::RopeLeaf(&self.leaf),
            ReplaceRangeAsTickTouchedRopeClosureItemRef::TextBlob(&self.blob),
        ]
    }}
}}

impl ReplaceRangeAsTickReadAffectedAnchorsClosure for ReplaceRangeStore {{
    fn read_affected_anchors_closure(
        &self,
    ) -> Vec<ReplaceRangeAsTickAffectedAnchorsClosureItemRef<'_>> {{
        vec![ReplaceRangeAsTickAffectedAnchorsClosureItemRef::Anchor(&self.anchor)]
    }}
}}

impl ReplaceRangeAsTickCreateNewBlobSlot for ReplaceRangeStore {{
    fn create_new_blob_slot(&mut self, value: TextBlob) -> TextBlob {{
        self.blob = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateNewLeavesSlot for ReplaceRangeStore {{
    fn create_new_leaves_slot(&mut self, value: RopeLeaf) -> RopeLeaf {{
        self.leaf = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateNewBranchesSlot for ReplaceRangeStore {{
    fn create_new_branches_slot(&mut self, value: RopeBranch) -> RopeBranch {{
        self.branch = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateNextHeadSlot for ReplaceRangeStore {{
    fn create_next_head_slot(&mut self, value: RopeHead) -> RopeHead {{
        self.next_head = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateTickSlot for ReplaceRangeStore {{
    fn create_tick_slot(&mut self, value: Tick) -> Tick {{
        self.tick = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickCreateReceiptSlot for ReplaceRangeStore {{
    fn create_receipt_slot(&mut self, value: TickReceipt) -> TickReceipt {{
        self.receipt = value.clone();
        value
    }}
}}

impl ReplaceRangeAsTickUpdateWorldlineCanonicalHead for ReplaceRangeStore {{
    fn update_worldline_canonical_head(&mut self, value: String) {{
        self.worldline.canonical_head_id = value;
    }}
}}

pub struct ReplaceRange;

impl ReplaceRangeAsTickRewrite for ReplaceRange {{
    type Error = ();

    fn apply<C>(
        &self,
        ctx: &mut C,
        _args: ReplaceRangeAsTickArgs,
    ) -> Result<ReplaceRangeAsTickResult, Self::Error>
    where
        C: ReplaceRangeAsTickContext,
    {{
        ctx.read_ast_state_slot();
        Ok(ReplaceRangeAsTickResult {{
            worldline_id: ctx.read_worldline_slot().worldline_id.clone(),
            next_head_id: String::new(),
            tick_id: String::new(),
            receipt_id: String::new(),
        }})
    }}
}}
"#
    ));

    assert!(
        !compile.status.success(),
        "expected rustc failure, got success"
    );
    let stderr = String::from_utf8_lossy(&compile.stderr);
    assert!(
        stderr.contains("read_ast_state_slot"),
        "unexpected stderr: {stderr}"
    );
}
