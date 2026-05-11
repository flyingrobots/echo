// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#[derive(Debug, Clone, PartialEq)]
pub struct BufferWorldline {
    pub worldline_id: String,
    pub canonical_head_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeHead {
    pub head_id: String,
    pub worldline_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeBranch {
    pub branch_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RopeLeaf {
    pub leaf_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlob {
    pub blob_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub anchor_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub tick_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickReceipt {
    pub receipt_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    pub checkpoint_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateBufferWorldlineInput {
    pub buffer_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateBufferWorldlineResult {
    pub worldline_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateCheckpointInput {
    pub worldline_id: String,
    pub kind: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateCheckpointResult {
    pub checkpoint_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceRangeAsTickInput {
    pub worldline_id: String,
    pub base_head_id: String,
    pub start_byte: i64,
    pub end_byte: i64,
    pub insert_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceRangeAsTickResult {
    pub worldline_id: String,
    pub next_head_id: String,
    pub tick_id: String,
    pub receipt_id: String,
}

pub struct ReplaceRangeStore {
    pub worldline: BufferWorldline,
    pub base_head: RopeHead,
    pub branch: RopeBranch,
    pub leaf: RopeLeaf,
    pub blob: TextBlob,
    pub anchor: Anchor,
    pub next_head: RopeHead,
    pub tick: Tick,
    pub receipt: TickReceipt,
}

impl ReplaceRangeAsTickReadWorldlineSlot for ReplaceRangeStore {
    fn read_worldline_slot(&self) -> &BufferWorldline {
        &self.worldline
    }
}

impl ReplaceRangeAsTickWriteWorldlineSlot for ReplaceRangeStore {
    fn write_worldline_slot(&mut self, value: BufferWorldline) {
        self.worldline = value;
    }
}

impl ReplaceRangeAsTickReadBaseHeadSlot for ReplaceRangeStore {
    fn read_base_head_slot(&self) -> &RopeHead {
        &self.base_head
    }
}

impl ReplaceRangeAsTickReadTouchedRopeClosure for ReplaceRangeStore {
    fn read_touched_rope_closure(&self) -> Vec<ReplaceRangeAsTickTouchedRopeClosureItemRef<'_>> {
        vec![
            ReplaceRangeAsTickTouchedRopeClosureItemRef::RopeBranch(&self.branch),
            ReplaceRangeAsTickTouchedRopeClosureItemRef::RopeLeaf(&self.leaf),
            ReplaceRangeAsTickTouchedRopeClosureItemRef::TextBlob(&self.blob),
        ]
    }
}

impl ReplaceRangeAsTickReadAffectedAnchorsClosure for ReplaceRangeStore {
    fn read_affected_anchors_closure(
        &self,
    ) -> Vec<ReplaceRangeAsTickAffectedAnchorsClosureItemRef<'_>> {
        vec![ReplaceRangeAsTickAffectedAnchorsClosureItemRef::Anchor(&self.anchor)]
    }
}

impl ReplaceRangeAsTickCreateNewBlobSlot for ReplaceRangeStore {
    fn create_new_blob_slot(&mut self, value: TextBlob) -> TextBlob {
        self.blob = value.clone();
        value
    }
}

impl ReplaceRangeAsTickCreateNewLeavesSlot for ReplaceRangeStore {
    fn create_new_leaves_slot(&mut self, value: RopeLeaf) -> RopeLeaf {
        self.leaf = value.clone();
        value
    }
}

impl ReplaceRangeAsTickCreateNewBranchesSlot for ReplaceRangeStore {
    fn create_new_branches_slot(&mut self, value: RopeBranch) -> RopeBranch {
        self.branch = value.clone();
        value
    }
}

impl ReplaceRangeAsTickCreateNextHeadSlot for ReplaceRangeStore {
    fn create_next_head_slot(&mut self, value: RopeHead) -> RopeHead {
        self.next_head = value.clone();
        value
    }
}

impl ReplaceRangeAsTickCreateTickSlot for ReplaceRangeStore {
    fn create_tick_slot(&mut self, value: Tick) -> Tick {
        self.tick = value.clone();
        value
    }
}

impl ReplaceRangeAsTickCreateReceiptSlot for ReplaceRangeStore {
    fn create_receipt_slot(&mut self, value: TickReceipt) -> TickReceipt {
        self.receipt = value.clone();
        value
    }
}

impl ReplaceRangeAsTickUpdateWorldlineCanonicalHead for ReplaceRangeStore {
    fn update_worldline_canonical_head(&mut self, value: String) {
        self.worldline.canonical_head_id = value;
    }
}

pub struct ReplaceRange;

impl ReplaceRangeAsTickRewrite for ReplaceRange {
    type Error = ();

    fn apply<C>(
        &self,
        ctx: &mut C,
        _args: ReplaceRangeAsTickArgs,
    ) -> Result<ReplaceRangeAsTickResult, Self::Error>
    where
        C: ReplaceRangeAsTickContext,
    {
        ctx.read_ast_state_slot();
        Ok(ReplaceRangeAsTickResult {
            worldline_id: ctx.read_worldline_slot().worldline_id.clone(),
            next_head_id: String::new(),
            tick_id: String::new(),
            receipt_id: String::new(),
        })
    }
}
