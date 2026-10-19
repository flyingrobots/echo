// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Fixture F5: append and replay, modeled on a census of a real Think store.
//!
//! The scene is a root with a few bucket nodes. Each tick, one admitted
//! intent appends an entry: a new node, its payload attachment, and an edge
//! from its bucket. The intents go through the trusted host as an installed
//! native contract package, so the history is real admitted history.
//!
//! Replay here means **applying retained patches**: the provenance service
//! rebuilds the worldline from the initial state, checking each tick's state
//! root, patch digest and commit hash as it goes. Re-executing admitted
//! operations and restoring stored records are not exercised by this
//! fixture; see `HISTORY_OPERATIONS` and `NOT_COVERED`.

use std::time::{Duration, Instant};

use bytes::Bytes;
use echo_registry_api::{
    ArgDef, ContractArtifactVerificationPolicy, ObjectDef, OpDef, OpKind, RegistryInfo,
    RegistryProvider,
};
use warp_core::{
    make_head_id, make_intent_kind, make_node_id, make_type_id, make_warp_id, AtomPayload,
    AttachmentKey, AttachmentValue, ConflictPolicy, ContractMutationHandler,
    ContractPackageIdentity, EdgeId, EdgeRecord, EngineBuilder, ExecutionGraphView, Footprint,
    GraphStore, GraphView, InboxPolicy, IngressEnvelope, IngressTarget, InstalledContractPackage,
    IntentOutcome, NodeId, NodeKey, NodeRecord, PatternGraph, PlaybackMode, RewriteRule,
    RuleExecutor, SchedulerKind, TickDelta, TrustedRuntimeHost, WarpOp, WorldlineId,
    WorldlineRuntime, WorldlineState, WorldlineTick, WriterHead, WriterHeadKey,
};

use super::manifest::{hex32, Construction, EvidenceClass, Expectation, WorkloadSpec};
use super::outcome::{Accounting, Checks, ResultClass};
use super::prng::SplitMix64;

/// Bumped whenever this generator's logic changes.
pub const FIXTURE_VERSION: u32 = 1;

/// History operations this fixture runs.
pub const HISTORY_OPERATIONS: &[&str] = &[
    "append-admitted-operations",
    "replay-apply-retained-patches",
    "readings-at-earlier-coordinates",
];

/// History operations the fixture does not yet exercise, and why.
pub const NOT_COVERED: &[(&str, &str)] = &[
    (
        "replay-re-execute-admitted-operations",
        "Echo has no re-execution path for native-rule intents on main",
    ),
    (
        "replay-restore-stored-records",
        "WAL recovery is not wired into this fixture yet",
    ),
];

const OP_ID: u32 = 7105;
const RULE_NAME: &str =
    "cmd/contract/5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f/7105/append";
const RULE_ID_LABEL: &str =
    "rule:cmd/contract/5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f/7105/append";
const SCHEMA_SHA256_HEX: &str = "5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f";
const KEY_LEN: usize = 16;
const WORLDLINE: [u8; 32] = [0xF5; 32];

/// A size tier: explicit dimensions, not a label.
#[derive(Clone, Copy, Debug)]
pub struct Tier {
    /// Tier name.
    pub name: &'static str,
    /// Bucket nodes under the root.
    pub buckets: u64,
    /// Appends, one per tick.
    pub appends: u64,
    /// Payload bytes per entry.
    pub payload_bytes: u64,
    /// Evenly spaced earlier coordinates read back after the run.
    pub checkpoints: u64,
}

/// Tier S: the first gate.
pub const S: Tier = Tier {
    name: "S",
    buckets: 8,
    appends: 200,
    payload_bytes: 16,
    checkpoints: 4,
};

/// Tier M.
pub const M: Tier = Tier {
    name: "M",
    buckets: 32,
    appends: 5_000,
    payload_bytes: 16,
    checkpoints: 8,
};

/// The F5 workload for a tier and seed.
pub fn workload(tier: Tier, seed: u64) -> WorkloadSpec {
    WorkloadSpec {
        fixture: "F5",
        fixture_version: FIXTURE_VERSION,
        seed,
        construction: Construction::AdmittedOperations,
        evidence: EvidenceClass::Modeled,
        dimensions: vec![
            ("buckets", tier.buckets),
            ("appends", tier.appends),
            ("payload_bytes", tier.payload_bytes),
            ("checkpoints", tier.checkpoints),
            ("writer_heads", 1),
        ],
        settings: vec![
            ("inbox", "accept-all"),
            ("admission", "installed-contract"),
            ("durability", "none"),
            ("ticks", "one-append-per-tick"),
            (
                "source",
                "git-warp@4889004e docs/READINGS_AND_OPTICS.md census",
            ),
        ],
        history_operations: HISTORY_OPERATIONS.to_vec(),
    }
}

/// One scripted append.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Append {
    /// Index of the bucket the entry goes under.
    pub bucket: u32,
    /// The entry's key; it determines the entry's node and edge identities.
    pub key: [u8; KEY_LEN],
    /// The entry's payload.
    pub payload: Vec<u8>,
}

/// The logical script, generated before execution.
///
/// Draw order per append: bucket (`below(buckets)`), then key (16 bytes),
/// then payload (`payload_bytes` bytes).
pub fn script(tier: Tier, seed: u64) -> Vec<Append> {
    let mut rng = SplitMix64::new(seed);
    let payload_len = usize::try_from(tier.payload_bytes).expect("payload length fits usize");
    (0..tier.appends)
        .map(|_| {
            let bucket = rng.below(tier.buckets).expect("tier has buckets");
            let mut key = [0; KEY_LEN];
            key.copy_from_slice(&rng.bytes(KEY_LEN));
            Append {
                bucket: u32::try_from(bucket).expect("bucket index fits u32"),
                key,
                payload: rng.bytes(payload_len),
            }
        })
        .collect()
}

/// Digest of the logical script. Identical dimensions do not establish an
/// identical workload; this does.
pub fn script_digest(script: &[Append]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"echo.bench.fixture.f5.script/v1\0");
    hash.update(&(script.len() as u64).to_le_bytes());
    for append in script {
        hash.update(&append.bucket.to_le_bytes());
        hash.update(&append.key);
        hash.update(&(append.payload.len() as u64).to_le_bytes());
        hash.update(&append.payload);
    }
    hash.finalize().into()
}

fn vars(append: &Append) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + KEY_LEN + append.payload.len());
    out.extend_from_slice(&append.bucket.to_le_bytes());
    out.extend_from_slice(&append.key);
    out.extend_from_slice(&append.payload);
    out
}

fn parse(vars: &[u8]) -> Option<(u32, [u8; KEY_LEN], &[u8])> {
    let bucket = u32::from_le_bytes(vars.get(..4)?.try_into().ok()?);
    let key: [u8; KEY_LEN] = vars.get(4..4 + KEY_LEN)?.try_into().ok()?;
    Some((bucket, key, vars.get(4 + KEY_LEN..)?))
}

/// The node of bucket `index`.
pub fn bucket_node(index: u32) -> NodeId {
    make_node_id(&format!("f5-bucket-{index:04}"))
}

/// The node of the entry with `key`.
pub fn entry_node(key: &[u8; KEY_LEN]) -> NodeId {
    let mut hash = blake3::Hasher::new();
    hash.update(b"echo.bench.fixture.f5.entry/v1\0");
    hash.update(key);
    NodeId(hash.finalize().into())
}

/// The bucket-to-entry edge of the entry with `key`.
pub fn entry_edge(key: &[u8; KEY_LEN]) -> EdgeId {
    let mut hash = blake3::Hasher::new();
    hash.update(b"echo.bench.fixture.f5.edge/v1\0");
    hash.update(key);
    EdgeId(hash.finalize().into())
}

fn entry_type() -> warp_core::TypeId {
    make_type_id("bench/f5/entry")
}

fn matches(view: GraphView<'_>, scope: &NodeId) -> bool {
    warp_core::eint_vars_for_op(view, scope, OP_ID)
        .and_then(parse)
        .is_some()
}

fn execute(view: &mut ExecutionGraphView<'_, '_>, scope: &NodeId, delta: &mut TickDelta) {
    let Some((bucket, key, payload)) =
        warp_core::observed_eint_vars_for_op(view, scope, OP_ID).and_then(parse)
    else {
        return;
    };
    let warp_id = view.warp_id();
    let entry = NodeKey {
        warp_id,
        local_id: entry_node(&key),
    };
    delta.push(WarpOp::UpsertNode {
        node: entry,
        record: NodeRecord { ty: entry_type() },
    });
    delta.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(entry),
        value: Some(AttachmentValue::Atom(AtomPayload::new(
            entry_type(),
            Bytes::copy_from_slice(payload),
        ))),
    });
    delta.push(WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: entry_edge(&key),
            from: bucket_node(bucket),
            to: entry.local_id,
            ty: make_type_id("bench/f5/contains"),
        },
    });
}

fn footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut footprint = warp_core::runtime_ingress_eint_read_footprint(view, scope);
    let Some((bucket, key, _)) = warp_core::eint_vars_for_op(view, scope, OP_ID).and_then(parse)
    else {
        return footprint;
    };
    let warp_id = view.warp_id();
    let entry = entry_node(&key);
    footprint.n_write.insert_with_warp(warp_id, entry);
    footprint
        .n_write
        .insert_with_warp(warp_id, bucket_node(bucket));
    footprint
        .e_write
        .insert_with_warp(warp_id, entry_edge(&key));
    footprint.a_write.insert(AttachmentKey::node_alpha(NodeKey {
        warp_id,
        local_id: entry,
    }));
    footprint
}

fn rule() -> RewriteRule {
    RewriteRule {
        id: make_type_id(RULE_ID_LABEL).0,
        name: RULE_NAME,
        left: PatternGraph { nodes: vec![] },
        matcher: matches,
        executor: RuleExecutor::observed(execute),
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

static APPEND_ARGS: &[ArgDef] = &[ArgDef {
    name: "input",
    ty: "AppendInput",
    required: true,
    list: false,
}];

static OPS: &[OpDef] = &[OpDef {
    kind: OpKind::Mutation,
    name: "append",
    op_id: OP_ID,
    args: APPEND_ARGS,
    result_ty: "Entry",
    directives_json: "{}",
    footprint_certificate: None,
}];

struct Registry;

impl RegistryProvider for Registry {
    fn info(&self) -> RegistryInfo {
        RegistryInfo {
            echo_abi_version: 1,
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            wesley_generator_version: "echo-wesley-gen/0.1.0",
            helper_api_version: 1,
        }
    }

    fn op_by_id(&self, op_id: u32) -> Option<&'static OpDef> {
        OPS.iter().find(|op| op.op_id == op_id)
    }

    fn all_ops(&self) -> &'static [OpDef] {
        OPS
    }

    fn all_enums(&self) -> &'static [echo_registry_api::EnumDef] {
        &[]
    }

    fn all_objects(&self) -> &'static [ObjectDef] {
        &[]
    }
}

fn package(registry: &Registry) -> InstalledContractPackage<'_> {
    InstalledContractPackage {
        identity: ContractPackageIdentity {
            package_name: "bench-fixture-f5",
            package_version: "1.0.0",
            artifact_hash_hex: "f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5f5",
        },
        registry,
        verification_policy: ContractArtifactVerificationPolicy {
            echo_abi_version: 1,
            codec_id: "cbor-canon-v1",
            registry_version: 1,
            schema_sha256_hex: SCHEMA_SHA256_HEX,
            wesley_generator_version: "echo-wesley-gen/0.1.0",
            helper_api_version: 1,
            footprint_certificates: &[],
            require_mutation_footprint_certificates: false,
        },
        mutation_handlers: vec![ContractMutationHandler {
            op_id: OP_ID,
            rule: rule(),
        }],
        inverse_handlers: vec![],
        query_observers: vec![],
    }
}

/// The initial worldline state: a root linked to each bucket.
pub fn initial_state(tier: Tier) -> WorldlineState {
    let warp_id = make_warp_id("bench-fixture-f5");
    let root = make_node_id("f5-root");
    let mut store = GraphStore::new(warp_id);
    store.insert_node(
        root,
        NodeRecord {
            ty: make_type_id("bench/f5/root"),
        },
    );
    for index in 0..tier.buckets {
        let index = u32::try_from(index).expect("bucket index fits u32");
        let bucket = bucket_node(index);
        store.insert_node(
            bucket,
            NodeRecord {
                ty: make_type_id("bench/f5/bucket"),
            },
        );
        store.insert_edge(
            root,
            EdgeRecord {
                id: warp_core::make_edge_id(&format!("f5-root-bucket-{index:04}")),
                from: root,
                to: bucket,
                ty: make_type_id("bench/f5/bucket-of"),
            },
        );
    }
    WorldlineState::from_root_store(store, root).expect("F5 root state")
}

/// What one execution produced, measured and observed.
#[derive(Clone, Debug, Default)]
pub struct Observation {
    /// Work accounting.
    pub accounting: Accounting,
    /// Per committed tick: worldline tick after, admitted count, state root.
    pub steps: Vec<(u64, usize, [u8; 32])>,
    /// Scheduler passes that committed nothing.
    pub idle_passes: u64,
    /// State root of the live worldline after the run.
    pub live_root: [u8; 32],
    /// State root rebuilt by applying retained patches, if replay succeeded.
    pub replay_root: Option<[u8; 32]>,
    /// Earlier coordinates: (tick, root recorded at commit, root replayed).
    pub checkpoints: Vec<(u64, [u8; 32], Option<[u8; 32]>)>,
    /// Entries found with the scripted payload and an edge from their bucket.
    pub entries_verified: u64,
    /// Measured durations.
    pub timings: Timings,
    /// Study probe: (reachable nodes, time) of each root computation in the append phase.
    #[cfg(feature = "host_test")]
    pub root_probe_append: Vec<(usize, Duration)>,
    /// Study probe: incremental shadow root equals its independent rebuild.
    #[cfg(feature = "shadow_root_probe")]
    pub shadow_parity: Option<bool>,
    /// Study probe: every shadow update, in commit order.
    #[cfg(feature = "shadow_root_probe")]
    pub shadow_log: Vec<warp_core::shadow_root_probe::Update>,
}

/// Measured durations, each with a declared boundary.
#[derive(Clone, Debug, Default)]
pub struct Timings {
    /// Submit, admit and tick for every append; envelopes built beforehand.
    pub append_end_to_end: Duration,
    /// The same, per append.
    pub per_append: Vec<Duration>,
    /// Full replay by applying retained patches from the initial state.
    pub replay: Duration,
}

/// Executes the fixture through the trusted host.
pub fn run(tier: Tier, seed: u64) -> Observation {
    let script = script(tier, seed);
    let initial = initial_state(tier);
    let base = initial.clone();
    let worldline_id = WorldlineId::from_bytes(WORLDLINE);
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(worldline_id, initial)
        .expect("worldline registers");
    runtime
        .register_writer_head(WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("f5-writer"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("writer head registers");
    let mut engine_store = GraphStore::default();
    let engine_root = make_node_id("f5-engine-root");
    engine_store.insert_node(
        engine_root,
        NodeRecord {
            ty: make_type_id("bench/f5/engine-root"),
        },
    );
    let engine = EngineBuilder::new(engine_store, engine_root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let mut host = TrustedRuntimeHost::new(runtime, engine).expect("host initializes");
    let registry = Registry;
    host.register_contract_package(package(&registry))
        .expect("package installs");

    // Setup outside measurement: every envelope is built before the clock.
    let envelopes: Vec<IngressEnvelope> = script
        .iter()
        .map(|append| {
            IngressEnvelope::local_intent(
                IngressTarget::DefaultWriter { worldline_id },
                make_intent_kind("echo.intent/eint-v1"),
                echo_wasm_abi::pack_intent_v1(OP_ID, &vars(append)).expect("intent packs"),
            )
        })
        .collect();

    let mut obs = Observation::default();
    obs.accounting.occurrences = script.len() as u64;
    let mut submissions = Vec::with_capacity(script.len());
    #[cfg(feature = "host_test")]
    let _ = warp_core::state_root_probe::take();
    let start = Instant::now();
    for envelope in envelopes {
        let one = Instant::now();
        obs.accounting.delivery_attempts += 1;
        let Ok(submission) = host.app().submit_intent(envelope) else {
            continue;
        };
        obs.accounting.submitted += 1;
        if host
            .admit_installed_contract_submission(submission.submission_id)
            .is_ok()
        {
            obs.accounting.admitted += 1;
        }
        // One append per tick; a pass that commits nothing is counted, and
        // the loop gives up after four.
        for _ in 0..4 {
            let steps = host.tick_once().expect("tick runs");
            if steps.is_empty() {
                obs.idle_passes += 1;
                continue;
            }
            for step in steps {
                obs.steps.push((
                    step.worldline_tick_after.as_u64(),
                    step.admitted_count,
                    step.state_root,
                ));
            }
            break;
        }
        obs.timings.per_append.push(one.elapsed());
        submissions.push(submission.submission_id);
    }
    obs.timings.append_end_to_end = start.elapsed();
    #[cfg(feature = "host_test")]
    {
        obs.root_probe_append = warp_core::state_root_probe::take();
    }

    // Outcome collection, outside measurement.
    for submission in &submissions {
        match host.app().observe_intent_outcome(submission) {
            IntentOutcome::Applied { .. } => obs.accounting.committed += 1,
            IntentOutcome::Pending { .. } => obs.accounting.unfinished += 1,
            _ => obs.accounting.refused += 1,
        }
    }
    obs.accounting.unfinished += obs.accounting.occurrences - submissions.len() as u64;
    let frontier = host
        .runtime()
        .worldlines()
        .get(&worldline_id)
        .expect("worldline exists");
    let live = frontier.state();
    obs.live_root = live.state_root();
    #[cfg(feature = "shadow_root_probe")]
    {
        let shadows = host.runtime().shadow_roots();
        obs.shadow_parity = Some(
            shadows.root(worldline_id)
                == Some(warp_core::shadow_root_probe::rebuild_root(
                    live.warp_state(),
                )),
        );
        obs.shadow_log = shadows.log().to_vec();
    }
    let warp_id = make_warp_id("bench-fixture-f5");
    if let Some(store) = live.store(&warp_id) {
        for append in &script {
            let entry = entry_node(&append.key);
            let payload_ok = matches!(
                store.node_attachment(&entry),
                Some(AttachmentValue::Atom(atom)) if atom.bytes.as_ref() == append.payload.as_slice()
            );
            let linked = store
                .edges_from(&bucket_node(append.bucket))
                .any(|edge| edge.id == entry_edge(&append.key) && edge.to == entry);
            if payload_ok && linked {
                obs.entries_verified += 1;
            }
        }
    }

    // Replay by applying retained patches, measured on its own.
    let start = Instant::now();
    let replayed = host
        .provenance()
        .replay_worldline_state(worldline_id, &base);
    obs.timings.replay = start.elapsed();
    obs.replay_root = replayed.ok().map(|state| state.state_root());

    // Readings at earlier coordinates, outside measurement.
    for k in 1..=tier.checkpoints {
        let tick = tier.appends * k / tier.checkpoints;
        let recorded = obs
            .steps
            .iter()
            .find(|(after, _, _)| *after == tick)
            .map_or([0; 32], |(_, _, root)| *root);
        let replayed = host
            .provenance()
            .replay_worldline_state_at(worldline_id, &base, WorldlineTick::from_raw(tick))
            .ok()
            .map(|state| state.state_root());
        obs.checkpoints.push((tick, recorded, replayed));
    }
    obs
}

/// Declared outcomes for a tier and seed, with optional pinned goldens.
pub fn expectation(tier: Tier, script_digest_hex: Option<&str>) -> Expectation {
    let mut assertions = vec![
        ("committed", tier.appends.to_string()),
        ("refused", "0".to_owned()),
        ("entries_verified", tier.appends.to_string()),
        ("one_commit_per_append", "true".to_owned()),
        ("replay_root_equals_live_root", "true".to_owned()),
        ("checkpoint_roots_equal_recorded", "true".to_owned()),
    ];
    if let Some(digest) = script_digest_hex {
        assertions.push(("script_digest", digest.to_owned()));
    }
    Expectation {
        assertions,
        output_formats: vec![("state_root", 1), ("f5_script", 1)],
    }
}

/// Validates an observation against the declared outcomes.
///
/// Expected counts come from the tier, not from Echo.
pub fn validate(
    tier: Tier,
    script: &[Append],
    expect: &Expectation,
    obs: &Observation,
) -> ResultClass {
    if obs.accounting.unfinished > 0 && obs.accounting.committed < tier.appends {
        return ResultClass::Incomplete(format!(
            "{} of {} appends unfinished",
            obs.accounting.unfinished, tier.appends
        ));
    }
    let mut checks = Checks::default();
    if let Some(pinned) = expect.get("script_digest") {
        checks.check(
            hex32(&script_digest(script)) == pinned,
            "generated script digest differs from the pinned digest",
        );
    }
    checks.check(
        obs.accounting.committed == tier.appends,
        format!(
            "committed {} of {} appends",
            obs.accounting.committed, tier.appends
        ),
    );
    checks.check(obs.accounting.refused == 0, "an append was refused");
    checks.check(
        obs.entries_verified == tier.appends,
        format!(
            "{} of {} entries present with payload and bucket edge",
            obs.entries_verified, tier.appends
        ),
    );
    let one_per_tick = obs.steps.len() as u64 == tier.appends
        && obs
            .steps
            .iter()
            .enumerate()
            .all(|(i, (after, admitted, _))| *after == i as u64 + 1 && *admitted == 1);
    checks.check(
        one_per_tick,
        "commits are not exactly one admitted append per consecutive tick",
    );
    checks.check(
        obs.steps.last().map(|(_, _, root)| *root) == Some(obs.live_root),
        "last recorded root differs from the live root",
    );
    checks.check(
        obs.replay_root == Some(obs.live_root),
        "replay by applying retained patches did not reproduce the live root",
    );
    checks.check(
        obs.checkpoints.len() as u64 == tier.checkpoints
            && obs
                .checkpoints
                .iter()
                .all(|(_, recorded, replayed)| *replayed == Some(*recorded)),
        "a reading at an earlier coordinate differs from the root recorded at commit",
    );
    checks.class()
}
