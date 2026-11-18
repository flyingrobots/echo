// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! This module owns the stable reason codes that name which effect-program
//! check refused a program, and the bounded artifact path where it refused.
//!
//! The judgment is a sequence of checks over the compiler bundle, its Core
//! and Target IR artifacts and the pinned lowering artifacts. A [`Trail`]
//! records the check that is running and the artifact path it is reading;
//! when a check refuses, the trail is the refusal's evidence. Codes are part
//! of the provider contract: they never change meaning, and the shared verdict
//! corpus pins the code each refused entry reaches.

/// The most bytes a refusal path carries; a longer path is cut at a
/// character boundary and ends in [`TRUNCATED`].
pub const MAX_PATH_BYTES: usize = 256;
/// The marker that ends a cut path.
pub const TRUNCATED: &str = "...";

/// The effect-program check that refused, as a stable machine-readable code.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[non_exhaustive]
pub enum RefusalReason {
    /// The compiler bundle envelope, its schema or an embedded artifact.
    BundleEnvelope,
    /// The Target IR artifact envelope or its declared contract.
    TargetIrEnvelope,
    /// The lawpack exports artifact envelope.
    ExportsEnvelope,
    /// The Core module envelope.
    CoreEnvelope,
    /// The Target IR's pin of its source Core.
    CorePin,
    /// The selected intent is absent from Core or Target IR.
    IntentSelection,
    /// The intent and its lowered form disagree outside the body, or the
    /// intent's shape or budget is outside the profile.
    IntentBinding,
    /// An optic-scoped intent, outside the authored-effect route.
    OpticIntent,
    /// The lowered intent's envelope or instruction table.
    CorrespondenceEnvelope,
    /// The execution order does not place each instruction exactly once.
    CorrespondenceOrder,
    /// A Core node and its Target instruction carry different contents.
    CorrespondenceInstruction,
    /// A structured `for` or `branch` entry differs from its Core node.
    CorrespondenceControl,
    /// A loop body or branch arm is not a statement block.
    CorrespondenceStatementBlock,
    /// A program without effects names a profile other than `echo.dpo@1`.
    NoEffectProfile,
    /// The lowering artifacts envelope or an embedded artifact.
    LoweringArtifacts,
    /// The target profile pin, its Core ABI, namespace or control support.
    TargetProfile,
    /// The intrinsic catalog pin or envelope.
    IntrinsicCatalog,
    /// The identity intrinsic contracts or the Core target import.
    IdentityContracts,
    /// The lawpack pin, its single Core import or its exports pin.
    Lawpack,
    /// The lawpack's target adapter entry or the adapter pin.
    Adapter,
    /// The source's lawpack alias.
    SourceAlias,
    /// Imported types are not the lawpack's exports.
    ImportedTypes,
    /// The adapter's operation profile for the intent.
    OperationProfile,
    /// An effect node the adapter does not authorize for its profile.
    EffectAuthorization,
    /// The catalog, export and adapter disagree on an intrinsic's contract.
    IntrinsicContract,
    /// An effect node's failure arms against its declared failures.
    FailureArms,
    /// The configuration's graph-read declaration does not match the reads.
    GraphReadDeclaration,
    /// A type-table entry outside the runtime's kinds.
    TypeTable,
    /// The module's declared failure sets, `fail` or `attempt` forms.
    FailureSets,
    /// The helper inventory: imported or source helper declarations.
    HelperInventory,
    /// A helper function's body is outside the checked profile.
    HelperFunction,
    /// A call does not prove its helper's preconditions.
    HelperPreconditions,
    /// The intent's input local.
    InputBinding,
    /// The evaluation basis is not exactly 32 bytes.
    Basis,
    /// An input constraint's shape or predicate.
    InputConstraint,
    /// A block's local table.
    BlockLocals,
    /// A local is claimed by the wrong block, twice, or never.
    LocalClaim,
    /// An expression is untyped, unproven or calls outside the profile.
    Expression,
    /// A binding's declared type does not include its value's type.
    BindingType,
    /// A `require` node's shape or payload.
    Require,
    /// An `attempt` arm's obstruction payload.
    Obstruction,
    /// An effect's input type is not its intrinsic's input.
    EffectInput,
    /// An effect's input or result is outside its intrinsic's runtime ABI.
    EffectAbi,
    /// An effect's binding is not its intrinsic's result type.
    EffectOutput,
    /// A loop's bound is not a literal covering its iterable.
    LoopBound,
    /// A loop's binder does not accept the iterable's items.
    LoopBinder,
    /// A node kind outside the profile, or an effect site on another node.
    NodeKind,
    /// An effect site the lowering relation established is not in the body.
    EffectSites,
    /// The result's type is not the intent's output type.
    ResultType,
    /// The worst-case cost overflows.
    CostOverflow,
    /// The worst case exceeds the declared step budget.
    StepBudget,
    /// The worst case exceeds the declared allocation budget.
    AllocationBudget,
    /// The result or an obstruction payload exceeds the output ceiling.
    OutputBudget,
    /// The program emits more mutations than one evaluation admits.
    MutationCount,
}

impl RefusalReason {
    /// The stable machine-readable code naming this check.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BundleEnvelope => "effect-program.bundle-envelope",
            Self::TargetIrEnvelope => "effect-program.target-ir-envelope",
            Self::ExportsEnvelope => "effect-program.exports-envelope",
            Self::CoreEnvelope => "effect-program.core-envelope",
            Self::CorePin => "effect-program.core-pin",
            Self::IntentSelection => "effect-program.intent-selection",
            Self::IntentBinding => "effect-program.intent-binding",
            Self::OpticIntent => "effect-program.optic-intent",
            Self::CorrespondenceEnvelope => "effect-program.correspondence.envelope",
            Self::CorrespondenceOrder => "effect-program.correspondence.order",
            Self::CorrespondenceInstruction => "effect-program.correspondence.instruction",
            Self::CorrespondenceControl => "effect-program.correspondence.control",
            Self::CorrespondenceStatementBlock => "effect-program.correspondence.statement-block",
            Self::NoEffectProfile => "effect-program.relation.no-effect-profile",
            Self::LoweringArtifacts => "effect-program.relation.lowering-artifacts",
            Self::TargetProfile => "effect-program.relation.target-profile",
            Self::IntrinsicCatalog => "effect-program.relation.intrinsic-catalog",
            Self::IdentityContracts => "effect-program.relation.identity-contracts",
            Self::Lawpack => "effect-program.relation.lawpack",
            Self::Adapter => "effect-program.relation.adapter",
            Self::SourceAlias => "effect-program.relation.source-alias",
            Self::ImportedTypes => "effect-program.relation.imported-types",
            Self::OperationProfile => "effect-program.relation.operation-profile",
            Self::EffectAuthorization => "effect-program.relation.effect-authorization",
            Self::IntrinsicContract => "effect-program.relation.intrinsic-contract",
            Self::FailureArms => "effect-program.relation.failure-arms",
            Self::GraphReadDeclaration => "effect-program.graph-read-declaration",
            Self::TypeTable => "effect-program.type-table",
            Self::FailureSets => "effect-program.failure-sets",
            Self::HelperInventory => "effect-program.helpers.inventory",
            Self::HelperFunction => "effect-program.helpers.function",
            Self::HelperPreconditions => "effect-program.helpers.preconditions",
            Self::InputBinding => "effect-program.input-binding",
            Self::Basis => "effect-program.basis",
            Self::InputConstraint => "effect-program.input-constraint",
            Self::BlockLocals => "effect-program.body.locals",
            Self::LocalClaim => "effect-program.body.local-claim",
            Self::Expression => "effect-program.body.expression",
            Self::BindingType => "effect-program.body.binding-type",
            Self::Require => "effect-program.body.require",
            Self::Obstruction => "effect-program.body.obstruction",
            Self::EffectInput => "effect-program.body.effect-input",
            Self::EffectAbi => "effect-program.body.effect-abi",
            Self::EffectOutput => "effect-program.body.effect-output",
            Self::LoopBound => "effect-program.body.loop-bound",
            Self::LoopBinder => "effect-program.body.loop-binder",
            Self::NodeKind => "effect-program.body.node-kind",
            Self::EffectSites => "effect-program.body.effect-sites",
            Self::ResultType => "effect-program.result-type",
            Self::CostOverflow => "effect-program.budget.overflow",
            Self::StepBudget => "effect-program.budget.steps",
            Self::AllocationBudget => "effect-program.budget.allocation",
            Self::OutputBudget => "effect-program.budget.output",
            Self::MutationCount => "effect-program.budget.mutations",
        }
    }
}

/// The check that refused a program and the bounded artifact path it was
/// reading, such as `/core/intents/basisRead/body/nodes/2/value`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgramRefusal {
    reason: RefusalReason,
    path: String,
}

impl ProgramRefusal {
    /// The check that refused.
    #[must_use]
    pub const fn reason(&self) -> RefusalReason {
        self.reason
    }

    /// The artifact path the check was reading, at most [`MAX_PATH_BYTES`].
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl std::fmt::Display for ProgramRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.reason.code(), self.path)
    }
}

/// The running check and the artifact path it reads.
///
/// A check marks its reason before it reads, and a nested walk enters a path
/// segment before it judges a child and leaves it once the child is
/// accepted. A refusal returns early without leaving, so the trail then names
/// the refusing check and the child it was judging.
pub(crate) struct Trail {
    reason: RefusalReason,
    segments: Vec<String>,
}

impl Trail {
    pub(crate) const fn new() -> Self {
        Self {
            reason: RefusalReason::BundleEnvelope,
            segments: Vec::new(),
        }
    }

    /// Starts a top-level check at an absolute path.
    pub(crate) fn at(&mut self, reason: RefusalReason, segments: &[&str]) {
        self.reason = reason;
        self.segments.clear();
        self.segments
            .extend(segments.iter().map(|segment| (*segment).to_owned()));
    }

    /// Names the check about to run, at the current path.
    pub(crate) fn mark(&mut self, reason: RefusalReason) {
        self.reason = reason;
    }

    /// Descends into a child of the current path.
    pub(crate) fn enter(&mut self, segment: impl Into<String>) {
        self.segments.push(segment.into());
    }

    /// Returns from an accepted child.
    pub(crate) fn leave(&mut self) {
        self.segments.pop();
    }

    /// The refusal this trail records, with its path bounded.
    pub(crate) fn refusal(&self) -> ProgramRefusal {
        ProgramRefusal {
            reason: self.reason,
            path: bounded_path(&self.segments),
        }
    }
}

fn bounded_path(segments: &[String]) -> String {
    let mut path = String::new();
    for segment in segments {
        path.push('/');
        path.push_str(segment);
    }
    if path.is_empty() {
        path.push('/');
    }
    if path.len() <= MAX_PATH_BYTES {
        return path;
    }
    let mut end = MAX_PATH_BYTES - TRUNCATED.len();
    while !path.is_char_boundary(end) {
        end -= 1;
    }
    path.truncate(end);
    path.push_str(TRUNCATED);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_path_is_cut_at_a_character_boundary() {
        let mut trail = Trail::new();
        trail.at(RefusalReason::HelperFunction, &["functions"]);
        trail.enter("é".repeat(MAX_PATH_BYTES));
        let refusal = trail.refusal();
        // Oracle: the path contract, at most MAX_PATH_BYTES ending in TRUNCATED.
        assert!(refusal.path().len() <= MAX_PATH_BYTES, "{}", refusal.path());
        assert!(refusal.path().ends_with(TRUNCATED), "{}", refusal.path());
        assert!(refusal.path().starts_with("/functions/é"));
    }

    #[test]
    fn an_accepted_child_leaves_the_path() {
        let mut trail = Trail::new();
        trail.at(RefusalReason::Expression, &["core", "intents", "i", "body"]);
        trail.enter("nodes/0");
        trail.leave();
        trail.mark(RefusalReason::ResultType);
        assert_eq!(
            trail.refusal().to_string(),
            "effect-program.result-type at /core/intents/i/body"
        );
    }
}
