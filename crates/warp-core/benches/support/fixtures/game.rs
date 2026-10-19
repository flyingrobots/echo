// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Fixture F6: a real-time game scene, run like a game.
//!
//! Each scene starts a trusted host, installs the game's contract package and
//! builds its world, then runs frames, then tears down. Every frame does what
//! a game loop does: it submits one step intent carrying that frame's input,
//! admits it, ticks, and reads every body back as a renderer would.
//!
//! **The game.** Bodies are circles with fixed-point (Q16.16) positions and
//! velocities. A step applies input, ages and despawns projectiles, spawns new
//! ones from emitters, integrates under gravity, bounces off the walls, sorts
//! bodies into a broadphase grid, and resolves overlapping pairs with a
//! position correction and an impulse, in canonical order. The same pure
//! [`step`] function drives both the Echo rule and the in-memory reference
//! the fixture validates against, so a valid run means Echo stored exactly
//! what the game computed.
//!
//! **The graph.** A root links a world node, which links one node per grid
//! cell. Each body is a node with its state as an attachment, and an edge
//! from the cell it is in; a body that changes cell gets a new edge. Each
//! touching pair has a contact edge from the lower body to the higher, added
//! when they touch and deleted when they part. A despawned body loses its
//! edges, then its node.
//!
//! **One rewrite per frame.** The step is one rule application that reads the
//! whole world and writes what changed, as a single-threaded game loop would.
//! Footprints must name every edge a rule writes, so the footprint runs the
//! same step to find them. Splitting the world into per-cell rewrites would
//! let workers share a frame, but neighbouring cells overlap, and Echo
//! rejects a conflicting rewrite outright; that variant is not built here.

use std::collections::{BTreeMap, BTreeSet};
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
    RuleExecutor, SchedulerKind, TickDelta, TrustedRuntimeHost, TypeId, WarpOp, WorldlineId,
    WorldlineRuntime, WorldlineState, WriterHead, WriterHeadKey,
};

use super::manifest::{Construction, EvidenceClass, WorkloadSpec};
use super::outcome::{Accounting, Checks, ResultClass};
use super::prng::SplitMix64;

/// Bumped whenever this generator's logic changes.
pub const FIXTURE_VERSION: u32 = 1;

const OP_ID: u32 = 7106;
const RULE_NAME: &str =
    "cmd/contract/6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f/7106/step";
const RULE_ID_LABEL: &str =
    "rule:cmd/contract/6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f/7106/step";
const SCHEMA_SHA256_HEX: &str = "6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f";
const WORLDLINE: [u8; 32] = [0xF6; 32];
const WARP: &str = "bench-fixture-f6";

/// One unit in Q16.16.
pub const ONE: i64 = 1 << 16;
/// A frame's budget at 60 Hz.
pub const FRAME_BUDGET: Duration = Duration::from_micros(16_667);

/// What a body is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Moves under gravity and collides.
    Dynamic = 0,
    /// A dynamic body steered by input.
    Player = 1,
    /// Spawned by an emitter; ignores gravity and other projectiles; expires.
    Projectile = 2,
    /// Fixed; spawns projectiles; does not collide.
    Emitter = 3,
}

impl Kind {
    fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Dynamic,
            1 => Self::Player,
            2 => Self::Projectile,
            3 => Self::Emitter,
            _ => return None,
        })
    }

    const fn collides(self) -> bool {
        !matches!(self, Self::Emitter)
    }
}

/// One body. Lengths and speeds are Q16.16 world units (per frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    /// Stable body number; it determines the body's node.
    pub id: u32,
    /// What the body is.
    pub kind: Kind,
    /// Centre.
    pub pos: [i64; 2],
    /// Velocity per frame.
    pub vel: [i64; 2],
    /// Radius.
    pub radius: i64,
    /// Frames left for a projectile; zero otherwise.
    pub ttl: u32,
}

const BODY_BYTES: usize = 4 + 1 + 8 * 5 + 4;

impl Body {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(BODY_BYTES);
        out.extend_from_slice(&self.id.to_le_bytes());
        out.push(self.kind as u8);
        for value in [self.pos[0], self.pos[1], self.vel[0], self.vel[1], self.radius] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&self.ttl.to_le_bytes());
        out
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != BODY_BYTES {
            return None;
        }
        let value = |at: usize| -> Option<i64> {
            Some(i64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
        };
        Some(Self {
            id: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            kind: Kind::from_u8(bytes[4])?,
            pos: [value(5)?, value(13)?],
            vel: [value(21)?, value(29)?],
            radius: value(37)?,
            ttl: u32::from_le_bytes(bytes[45..49].try_into().ok()?),
        })
    }
}

/// A scene: explicit dimensions, not a label.
#[derive(Clone, Copy, Debug)]
pub struct Scene {
    /// Scene name.
    pub name: &'static str,
    /// Index carried in the step intent, so the rule knows the scene.
    pub index: u8,
    /// Dynamic bodies at start.
    pub bodies: u32,
    /// Body radius, whole units.
    pub radius: i64,
    /// World width and height, whole units.
    pub world: i64,
    /// Grid cell size, whole units. At least twice the largest radius plus
    /// the largest per-frame speed, so contacts are found in neighbour cells.
    pub cell: i64,
    /// Gravity per frame, Q16.16.
    pub gravity: i64,
    /// Restitution, out of 256.
    pub restitution: i64,
    /// Largest initial speed per axis, Q16.16.
    pub initial_speed: i64,
    /// Whether body 0 is a player steered by input.
    pub player: bool,
    /// Emitters, spaced across the world.
    pub emitters: u32,
    /// Projectiles each emitter spawns per frame.
    pub spawn_per_emitter: u32,
    /// Frames a projectile lives.
    pub projectile_ttl: u32,
    /// Projectile speed per axis, Q16.16.
    pub projectile_speed: i64,
    /// Frames run.
    pub frames: u64,
}

impl Scene {
    fn grid(&self) -> i64 {
        self.world / self.cell
    }

    fn cells(&self) -> u32 {
        u32::try_from(self.grid() * self.grid()).expect("cell count fits u32")
    }

    /// The scene with `frames` frames and its body and spawn counts scaled by
    /// `numer / denom`, for quick tests.
    pub const fn scaled(mut self, numer: u32, denom: u32, frames: u64) -> Self {
        self.bodies = self.bodies * numer / denom;
        self.emitters = if self.emitters == 0 {
            0
        } else {
            let scaled = self.emitters * numer / denom;
            if scaled == 0 {
                1
            } else {
                scaled
            }
        };
        self.frames = frames;
        self
    }
}

/// A sparse arena: bodies drift and bounce, and a player steers through them.
pub const ARENA: Scene = Scene {
    name: "arena",
    index: 0,
    bodies: 256,
    radius: 6,
    world: 1024,
    cell: 32,
    gravity: 0,
    restitution: 230,
    initial_speed: 3 * ONE,
    player: true,
    emitters: 0,
    spawn_per_emitter: 0,
    projectile_ttl: 0,
    projectile_speed: 0,
    frames: 600,
};

/// A dense pile: bodies fall into a box and stay in contact.
pub const PILE: Scene = Scene {
    name: "pile",
    index: 1,
    bodies: 2_048,
    radius: 4,
    world: 512,
    cell: 16,
    gravity: ONE / 8,
    restitution: 64,
    initial_speed: ONE / 2,
    player: false,
    emitters: 0,
    spawn_per_emitter: 0,
    projectile_ttl: 0,
    projectile_speed: 0,
    frames: 600,
};

/// Projectile churn: emitters fire every frame and projectiles expire, among
/// a few drifting bodies.
pub const BULLETS: Scene = Scene {
    name: "bullets",
    index: 2,
    bodies: 128,
    radius: 6,
    world: 1024,
    cell: 32,
    gravity: 0,
    restitution: 230,
    initial_speed: 2 * ONE,
    player: true,
    emitters: 8,
    spawn_per_emitter: 4,
    projectile_ttl: 60,
    projectile_speed: 6 * ONE,
    frames: 600,
};

/// Every scene, in run order.
pub const SCENES: [Scene; 3] = [ARENA, PILE, BULLETS];

/// The canonical workload of a scene.
pub fn workload(scene: Scene, seed: u64) -> WorkloadSpec {
    WorkloadSpec {
        fixture: "F6",
        fixture_version: FIXTURE_VERSION,
        seed,
        construction: Construction::AdmittedOperations,
        evidence: EvidenceClass::Exploratory,
        dimensions: vec![
            ("scene", u64::from(scene.index)),
            ("bodies", u64::from(scene.bodies)),
            ("world_units", u64::try_from(scene.world).unwrap_or(0)),
            ("cell_units", u64::try_from(scene.cell).unwrap_or(0)),
            ("emitters", u64::from(scene.emitters)),
            ("spawn_per_emitter", u64::from(scene.spawn_per_emitter)),
            ("projectile_ttl", u64::from(scene.projectile_ttl)),
            ("frames", scene.frames),
            ("writer_heads", 1),
        ],
        settings: vec![
            ("inbox", "accept-all"),
            ("admission", "installed-contract"),
            ("durability", "none"),
            ("ticks", "one-step-intent-per-frame"),
            ("physics", "q16.16-impulse-v1"),
        ],
        history_operations: vec!["frames-admitted-operations", "replay-apply-retained-patches"],
    }
}

/// The game world: what the step reads and produces.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct World {
    /// Frames stepped.
    pub frame: u64,
    /// The next body number to allocate.
    pub next_id: u32,
    /// Bodies by number.
    pub bodies: BTreeMap<u32, Body>,
    /// Touching pairs, lower number first.
    pub contacts: BTreeSet<(u32, u32)>,
}

/// The world a scene starts with.
pub fn initial_world(scene: Scene, seed: u64) -> World {
    let mut rng = SplitMix64::new(seed ^ (u64::from(scene.index) << 56));
    let mut world = World::default();
    let radius = scene.radius * ONE;
    // Bodies start on a jittered lattice, so none overlap.
    let spacing = scene.radius * 3;
    let per_row = (scene.world / spacing).max(1);
    for n in 0..scene.bodies {
        let row = i64::from(n) / per_row;
        let col = i64::from(n) % per_row;
        let jitter = |rng: &mut SplitMix64| {
            i64::try_from(rng.below(u64::try_from(scene.radius / 2 * ONE).unwrap_or(1).max(1)).unwrap_or(0))
                .unwrap_or(0)
        };
        let x = (col * spacing + spacing / 2) * ONE + jitter(&mut rng);
        let y = (scene.world - 1 - (row * spacing + spacing / 2)) * ONE - jitter(&mut rng);
        let speed = |rng: &mut SplitMix64| {
            let span = u64::try_from(2 * scene.initial_speed + 1).unwrap_or(1);
            i64::try_from(rng.below(span).unwrap_or(0)).unwrap_or(0) - scene.initial_speed
        };
        let vel = [speed(&mut rng), speed(&mut rng)];
        let kind = if scene.player && n == 0 {
            Kind::Player
        } else {
            Kind::Dynamic
        };
        let id = world.next_id;
        world.next_id += 1;
        world.bodies.insert(
            id,
            Body {
                id,
                kind,
                pos: [x, y.max(radius)],
                vel,
                radius,
                ttl: 0,
            },
        );
    }
    for e in 0..scene.emitters {
        let x = scene.world * ONE * i64::from(2 * e + 1) / i64::from(2 * scene.emitters);
        let id = world.next_id;
        world.next_id += 1;
        world.bodies.insert(
            id,
            Body {
                id,
                kind: Kind::Emitter,
                pos: [x, scene.world * ONE / 2],
                vel: [0, 0],
                radius,
                ttl: 0,
            },
        );
    }
    world
}

/// The input of frame `frame`: an acceleration for the player.
pub fn input(scene: Scene, seed: u64, frame: u64) -> [i64; 2] {
    if !scene.player {
        return [0, 0];
    }
    let mut rng = SplitMix64::new(seed ^ 0xA5A5_0000 ^ frame);
    let axis = |rng: &mut SplitMix64| i64::try_from(rng.below(5).unwrap_or(2)).unwrap_or(2) - 2;
    [axis(&mut rng) * ONE / 4, axis(&mut rng) * ONE / 4]
}

fn cell_of(scene: Scene, pos: [i64; 2]) -> u32 {
    let grid = scene.grid();
    let cx = (pos[0] / (scene.cell * ONE)).clamp(0, grid - 1);
    let cy = (pos[1] / (scene.cell * ONE)).clamp(0, grid - 1);
    u32::try_from(cy * grid + cx).expect("cell index fits u32")
}

/// One frame of the game. Pure and deterministic: the Echo rule, its
/// footprint and the reference all call it.
pub fn step(scene: Scene, seed: u64, world: &World, input: [i64; 2]) -> World {
    let frame = world.frame;
    let mut next_id = world.next_id;
    let mut bodies: Vec<Body> = world.bodies.values().copied().collect();

    // Input, ageing and expiry.
    for body in &mut bodies {
        if body.kind == Kind::Player {
            body.vel[0] += input[0];
            body.vel[1] += input[1];
        }
        if body.kind == Kind::Projectile {
            body.ttl = body.ttl.saturating_sub(1);
        }
    }
    bodies.retain(|body| body.kind != Kind::Projectile || body.ttl > 0);

    // Spawns, in emitter order.
    let emitters: Vec<Body> = bodies
        .iter()
        .filter(|body| body.kind == Kind::Emitter)
        .copied()
        .collect();
    for emitter in emitters {
        for k in 0..scene.spawn_per_emitter {
            let mut rng = SplitMix64::new(
                seed ^ frame.rotate_left(20) ^ (u64::from(emitter.id) << 40) ^ u64::from(k),
            );
            let dir = |rng: &mut SplitMix64| {
                let span = u64::try_from(2 * scene.projectile_speed + 1).unwrap_or(1);
                i64::try_from(rng.below(span).unwrap_or(0)).unwrap_or(0) - scene.projectile_speed
            };
            let vel = [dir(&mut rng), dir(&mut rng)];
            bodies.push(Body {
                id: next_id,
                kind: Kind::Projectile,
                pos: emitter.pos,
                vel,
                radius: scene.radius * ONE / 2,
                ttl: scene.projectile_ttl,
            });
            next_id += 1;
        }
    }

    // Integration and walls.
    let limit = scene.world * ONE;
    for body in &mut bodies {
        if body.kind == Kind::Emitter {
            continue;
        }
        if body.kind != Kind::Projectile {
            body.vel[1] -= scene.gravity;
        }
        for axis in 0..2 {
            body.pos[axis] += body.vel[axis];
            if body.pos[axis] < body.radius {
                body.pos[axis] = body.radius;
                body.vel[axis] = -body.vel[axis] * scene.restitution / 256;
            } else if body.pos[axis] > limit - body.radius {
                body.pos[axis] = limit - body.radius;
                body.vel[axis] = -body.vel[axis] * scene.restitution / 256;
            }
        }
    }

    // Broadphase: bodies by cell, in body order.
    let grid = scene.grid();
    let mut cells: Vec<Vec<usize>> = vec![Vec::new(); usize::try_from(grid * grid).unwrap_or(0)];
    for (index, body) in bodies.iter().enumerate() {
        if body.kind.collides() {
            cells[cell_of(scene, body.pos) as usize].push(index);
        }
    }

    // Narrowphase and response, in canonical order: each body against later
    // bodies in its own and neighbouring cells.
    let mut contacts = BTreeSet::new();
    for i in 0..bodies.len() {
        if !bodies[i].kind.collides() {
            continue;
        }
        let home = i64::from(cell_of(scene, bodies[i].pos));
        let (cx, cy) = (home % grid, home / grid);
        let mut partners = Vec::new();
        for ny in (cy - 1).max(0)..=(cy + 1).min(grid - 1) {
            for nx in (cx - 1).max(0)..=(cx + 1).min(grid - 1) {
                partners.extend(cells[usize::try_from(ny * grid + nx).unwrap_or(0)].iter().copied());
            }
        }
        partners.retain(|&j| j > i);
        partners.sort_unstable();
        for j in partners {
            let (a, b) = (bodies[i], bodies[j]);
            if a.kind == Kind::Projectile && b.kind == Kind::Projectile {
                continue;
            }
            let d = [i128::from(b.pos[0] - a.pos[0]), i128::from(b.pos[1] - a.pos[1])];
            let reach = i128::from(a.radius + b.radius);
            let dist2 = d[0] * d[0] + d[1] * d[1];
            if dist2 >= reach * reach {
                continue;
            }
            contacts.insert((a.id.min(b.id), a.id.max(b.id)));
            let dist = i128::try_from(u128::try_from(dist2).unwrap_or(0).isqrt()).unwrap_or(0);
            let normal = if dist == 0 {
                [i128::from(ONE), 0]
            } else {
                [d[0] * i128::from(ONE) / dist, d[1] * i128::from(ONE) / dist]
            };
            let push = (reach - dist) / 2;
            let rel = [
                i128::from(b.vel[0] - a.vel[0]),
                i128::from(b.vel[1] - a.vel[1]),
            ];
            let closing = (rel[0] * normal[0] + rel[1] * normal[1]) / i128::from(ONE);
            let impulse = if closing < 0 {
                -(256 + i128::from(scene.restitution)) * closing / 512
            } else {
                0
            };
            for axis in 0..2 {
                let shift = i64::try_from(normal[axis] * push / i128::from(ONE)).unwrap_or(0);
                let kick = i64::try_from(normal[axis] * impulse / i128::from(ONE)).unwrap_or(0);
                bodies[i].pos[axis] -= shift;
                bodies[j].pos[axis] += shift;
                bodies[i].vel[axis] -= kick;
                bodies[j].vel[axis] += kick;
            }
        }
    }

    World {
        frame: frame + 1,
        next_id,
        bodies: bodies.into_iter().map(|body| (body.id, body)).collect(),
        contacts,
    }
}

// --- Graph identities ---------------------------------------------------

fn hashed(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(domain);
    for part in parts {
        hash.update(part);
    }
    hash.finalize().into()
}

fn root_node() -> NodeId {
    make_node_id("f6-root")
}

fn world_node() -> NodeId {
    make_node_id("f6-world")
}

fn cell_node(cell: u32) -> NodeId {
    NodeId(hashed(b"echo.bench.fixture.f6.cell/v1\0", &[&cell.to_le_bytes()]))
}

/// The node of body `id`.
pub fn body_node(id: u32) -> NodeId {
    NodeId(hashed(b"echo.bench.fixture.f6.body/v1\0", &[&id.to_le_bytes()]))
}

fn member_edge(id: u32, cell: u32) -> EdgeId {
    EdgeId(hashed(
        b"echo.bench.fixture.f6.member/v1\0",
        &[&id.to_le_bytes(), &cell.to_le_bytes()],
    ))
}

fn contact_edge(a: u32, b: u32) -> EdgeId {
    EdgeId(hashed(
        b"echo.bench.fixture.f6.contact/v1\0",
        &[&a.to_le_bytes(), &b.to_le_bytes()],
    ))
}

fn body_type() -> TypeId {
    make_type_id("bench/f6/body")
}

fn member_type() -> TypeId {
    make_type_id("bench/f6/member")
}

fn contact_type() -> TypeId {
    make_type_id("bench/f6/contact")
}

fn world_type() -> TypeId {
    make_type_id("bench/f6/world")
}

fn world_bytes(world: &World) -> Vec<u8> {
    let mut out = Vec::with_capacity(12);
    out.extend_from_slice(&world.frame.to_le_bytes());
    out.extend_from_slice(&world.next_id.to_le_bytes());
    out
}

fn atom(ty: TypeId, bytes: Vec<u8>) -> AttachmentValue {
    AttachmentValue::Atom(AtomPayload::new(ty, Bytes::from(bytes)))
}

// --- Reading the world from the graph ------------------------------------

/// The reads the step needs, over either view an executor or a footprint
/// gets.
trait WorldReader {
    fn attachment_bytes(&mut self, node: &NodeId) -> Option<Vec<u8>>;
    fn targets(&mut self, node: &NodeId, ty: TypeId) -> Vec<NodeId>;
}

impl WorldReader for GraphView<'_> {
    fn attachment_bytes(&mut self, node: &NodeId) -> Option<Vec<u8>> {
        match self.node_attachment(node) {
            Some(AttachmentValue::Atom(atom)) => Some(atom.bytes.to_vec()),
            _ => None,
        }
    }

    fn targets(&mut self, node: &NodeId, ty: TypeId) -> Vec<NodeId> {
        self.edges_from(node)
            .filter(|edge| edge.ty == ty)
            .map(|edge| edge.to)
            .collect()
    }
}

impl WorldReader for ExecutionGraphView<'_, '_> {
    fn attachment_bytes(&mut self, node: &NodeId) -> Option<Vec<u8>> {
        match self.node_attachment(node) {
            Some(AttachmentValue::Atom(atom)) => Some(atom.bytes.to_vec()),
            _ => None,
        }
    }

    fn targets(&mut self, node: &NodeId, ty: TypeId) -> Vec<NodeId> {
        self.edges_from(node)
            .filter(|edge| edge.ty == ty)
            .map(|edge| edge.to)
            .collect()
    }
}

/// Reads the world back from the graph, or `None` if it is malformed.
fn read_world(reader: &mut impl WorldReader, scene: Scene) -> Option<World> {
    let meta = reader.attachment_bytes(&world_node())?;
    let frame = u64::from_le_bytes(meta.get(0..8)?.try_into().ok()?);
    let next_id = u32::from_le_bytes(meta.get(8..12)?.try_into().ok()?);
    let mut bodies = BTreeMap::new();
    let mut contacts = BTreeSet::new();
    let mut nodes = Vec::new();
    for cell in 0..scene.cells() {
        nodes.extend(reader.targets(&cell_node(cell), member_type()));
    }
    // Emitters do not collide but still sit in a cell, so every body is
    // reached through its cell.
    for node in nodes {
        let body = Body::decode(&reader.attachment_bytes(&node)?)?;
        bodies.insert(body.id, body);
    }
    let by_node: BTreeMap<NodeId, u32> = bodies.keys().map(|&id| (body_node(id), id)).collect();
    for &id in bodies.keys() {
        for other in reader.targets(&body_node(id), contact_type()) {
            contacts.insert((id, *by_node.get(&other)?));
        }
    }
    Some(World {
        frame,
        next_id,
        bodies,
        contacts,
    })
}

/// The ops that turn `before` into `after` in the graph.
fn diff_ops(scene: Scene, warp_id: warp_core::WarpId, before: &World, after: &World) -> Vec<WarpOp> {
    let key = |node: NodeId| NodeKey {
        warp_id,
        local_id: node,
    };
    let mut ops = Vec::new();
    for pair in &before.contacts {
        if !after.contacts.contains(pair) {
            ops.push(WarpOp::DeleteEdge {
                warp_id,
                from: body_node(pair.0),
                edge_id: contact_edge(pair.0, pair.1),
            });
        }
    }
    for (id, old) in &before.bodies {
        let old_cell = cell_of(scene, old.pos);
        match after.bodies.get(id) {
            None => {
                ops.push(WarpOp::DeleteEdge {
                    warp_id,
                    from: cell_node(old_cell),
                    edge_id: member_edge(*id, old_cell),
                });
                ops.push(WarpOp::DeleteNode {
                    node: key(body_node(*id)),
                });
            }
            Some(new) => {
                if new != old {
                    ops.push(WarpOp::SetAttachment {
                        key: AttachmentKey::node_alpha(key(body_node(*id))),
                        value: Some(atom(body_type(), new.encode())),
                    });
                }
                let new_cell = cell_of(scene, new.pos);
                if new_cell != old_cell {
                    ops.push(WarpOp::DeleteEdge {
                        warp_id,
                        from: cell_node(old_cell),
                        edge_id: member_edge(*id, old_cell),
                    });
                    ops.push(member_upsert(warp_id, *id, new_cell));
                }
            }
        }
    }
    for (id, new) in &after.bodies {
        if before.bodies.contains_key(id) {
            continue;
        }
        ops.push(WarpOp::UpsertNode {
            node: key(body_node(*id)),
            record: NodeRecord { ty: body_type() },
        });
        ops.push(WarpOp::SetAttachment {
            key: AttachmentKey::node_alpha(key(body_node(*id))),
            value: Some(atom(body_type(), new.encode())),
        });
        ops.push(member_upsert(warp_id, *id, cell_of(scene, new.pos)));
    }
    for pair in &after.contacts {
        if !before.contacts.contains(pair) {
            ops.push(WarpOp::UpsertEdge {
                warp_id,
                record: EdgeRecord {
                    id: contact_edge(pair.0, pair.1),
                    from: body_node(pair.0),
                    to: body_node(pair.1),
                    ty: contact_type(),
                },
            });
        }
    }
    ops.push(WarpOp::SetAttachment {
        key: AttachmentKey::node_alpha(key(world_node())),
        value: Some(atom(world_type(), world_bytes(after))),
    });
    ops
}

fn member_upsert(warp_id: warp_core::WarpId, id: u32, cell: u32) -> WarpOp {
    WarpOp::UpsertEdge {
        warp_id,
        record: EdgeRecord {
            id: member_edge(id, cell),
            from: cell_node(cell),
            to: body_node(id),
            ty: member_type(),
        },
    }
}

// --- The rule -------------------------------------------------------------

fn vars(scene: Scene, frame: u64, input: [i64; 2]) -> Vec<u8> {
    let mut out = Vec::with_capacity(25);
    out.push(scene.index);
    out.extend_from_slice(&frame.to_le_bytes());
    out.extend_from_slice(&input[0].to_le_bytes());
    out.extend_from_slice(&input[1].to_le_bytes());
    out
}

fn parse(vars: &[u8]) -> Option<(Scene, u64, [i64; 2])> {
    let scene = *SCENES.get(usize::from(*vars.first()?))?;
    let frame = u64::from_le_bytes(vars.get(1..9)?.try_into().ok()?);
    let x = i64::from_le_bytes(vars.get(9..17)?.try_into().ok()?);
    let y = i64::from_le_bytes(vars.get(17..25)?.try_into().ok()?);
    Some((scene, frame, [x, y]))
}

/// The scene the installed rule simulates. Scaled test scenes share their
/// full scene's index, so the rule reads the live dimensions from here.
static ACTIVE: std::sync::Mutex<Option<(Scene, u64)>> = std::sync::Mutex::new(None);

fn active(index: u8) -> Option<(Scene, u64)> {
    let guard = ACTIVE.lock().ok()?;
    guard.filter(|(scene, _)| scene.index == index)
}

fn matches(view: GraphView<'_>, scope: &NodeId) -> bool {
    warp_core::eint_vars_for_op(view, scope, OP_ID)
        .and_then(parse)
        .is_some()
}

fn execute(view: &mut ExecutionGraphView<'_, '_>, scope: &NodeId, delta: &mut TickDelta) {
    let Some((index, frame, input)) = warp_core::observed_eint_vars_for_op(view, scope, OP_ID)
        .and_then(parse)
        .map(|(scene, frame, input)| (scene.index, frame, input))
    else {
        return;
    };
    let Some((scene, seed)) = active(index) else {
        return;
    };
    let Some(before) = read_world(view, scene) else {
        return;
    };
    if before.frame != frame {
        return;
    }
    let after = step(scene, seed, &before, input);
    for op in diff_ops(scene, view.warp_id(), &before, &after) {
        delta.push(op);
    }
}

fn footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let mut footprint = warp_core::runtime_ingress_eint_read_footprint(view, scope);
    let Some((index, frame, input)) = warp_core::eint_vars_for_op(view, scope, OP_ID)
        .and_then(parse)
        .map(|(scene, frame, input)| (scene.index, frame, input))
    else {
        return footprint;
    };
    let Some((scene, seed)) = active(index) else {
        return footprint;
    };
    let warp_id = view.warp_id();
    let mut reader = view;
    let Some(before) = read_world(&mut reader, scene) else {
        return footprint;
    };
    // Reads: the world, every cell, every body.
    let alpha = |node: NodeId| {
        AttachmentKey::node_alpha(NodeKey {
            warp_id,
            local_id: node,
        })
    };
    footprint.n_read.insert_with_warp(warp_id, world_node());
    footprint.a_read.insert(alpha(world_node()));
    for cell in 0..scene.cells() {
        footprint.n_read.insert_with_warp(warp_id, cell_node(cell));
    }
    for &id in before.bodies.keys() {
        footprint.n_read.insert_with_warp(warp_id, body_node(id));
        footprint.a_read.insert(alpha(body_node(id)));
    }
    if before.frame != frame {
        return footprint;
    }
    // Writes: exactly what the step will write, found by running it.
    let after = step(scene, seed, &before, input);
    for op in diff_ops(scene, warp_id, &before, &after) {
        match op {
            WarpOp::SetAttachment { key, .. } => {
                footprint.a_write.insert(key);
            }
            WarpOp::UpsertNode { node, .. } | WarpOp::DeleteNode { node } => {
                footprint.n_write.insert(node);
                footprint.a_write.insert(AttachmentKey::node_alpha(node));
            }
            WarpOp::UpsertEdge { warp_id, record } => {
                footprint.n_write.insert_with_warp(warp_id, record.from);
                footprint.e_write.insert_with_warp(warp_id, record.id);
            }
            WarpOp::DeleteEdge {
                warp_id,
                from,
                edge_id,
            } => {
                footprint.n_write.insert_with_warp(warp_id, from);
                footprint.e_write.insert_with_warp(warp_id, edge_id);
                footprint
                    .a_write
                    .insert(AttachmentKey::edge_beta(warp_core::EdgeKey {
                        warp_id,
                        local_id: edge_id,
                    }));
            }
            _ => {}
        }
    }
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

static STEP_ARGS: &[ArgDef] = &[ArgDef {
    name: "input",
    ty: "StepInput",
    required: true,
    list: false,
}];

static OPS: &[OpDef] = &[OpDef {
    kind: OpKind::Mutation,
    name: "step",
    op_id: OP_ID,
    args: STEP_ARGS,
    result_ty: "Frame",
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
            package_name: "bench-fixture-f6",
            package_version: "1.0.0",
            artifact_hash_hex: "f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6f6",
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

/// The initial worldline state of `world`.
pub fn initial_state(scene: Scene, world: &World) -> WorldlineState {
    let warp_id = make_warp_id(WARP);
    let mut store = GraphStore::new(warp_id);
    let root = root_node();
    store.insert_node(root, NodeRecord { ty: make_type_id("bench/f6/root") });
    store.insert_node(world_node(), NodeRecord { ty: world_type() });
    store.set_node_attachment(world_node(), Some(atom(world_type(), world_bytes(world))));
    store.insert_edge(
        root,
        EdgeRecord {
            id: warp_core::make_edge_id("f6-root-world"),
            from: root,
            to: world_node(),
            ty: make_type_id("bench/f6/world-of"),
        },
    );
    for cell in 0..scene.cells() {
        store.insert_node(cell_node(cell), NodeRecord { ty: make_type_id("bench/f6/cell") });
        store.insert_edge(
            world_node(),
            EdgeRecord {
                id: EdgeId(hashed(b"echo.bench.fixture.f6.cell-of/v1\0", &[&cell.to_le_bytes()])),
                from: world_node(),
                to: cell_node(cell),
                ty: make_type_id("bench/f6/cell-of"),
            },
        );
    }
    for body in world.bodies.values() {
        store.insert_node(body_node(body.id), NodeRecord { ty: body_type() });
        store.set_node_attachment(body_node(body.id), Some(atom(body_type(), body.encode())));
        let cell = cell_of(scene, body.pos);
        store.insert_edge(
            cell_node(cell),
            EdgeRecord {
                id: member_edge(body.id, cell),
                from: cell_node(cell),
                to: body_node(body.id),
                ty: member_type(),
            },
        );
    }
    for &(a, b) in &world.contacts {
        store.insert_edge(
            body_node(a),
            EdgeRecord {
                id: contact_edge(a, b),
                from: body_node(a),
                to: body_node(b),
                ty: contact_type(),
            },
        );
    }
    WorldlineState::from_root_store(store, root).expect("F6 root state")
}

// --- Running a scene -------------------------------------------------------

/// What one scene run produced, measured and observed.
#[derive(Clone, Debug, Default)]
pub struct Observation {
    /// Work accounting: one step intent per frame.
    pub accounting: Accounting,
    /// Frames that committed exactly one tick.
    pub one_tick_frames: u64,
    /// The world read back from the live graph at the end.
    pub final_world: Option<World>,
    /// The reference world after the same frames.
    pub reference_world: World,
    /// Live state root at the end.
    pub live_root: [u8; 32],
    /// State root rebuilt by applying retained patches.
    pub replay_root: Option<[u8; 32]>,
    /// Bodies seen by the per-frame render read, summed over frames.
    pub rendered: u64,
    /// Peak live bodies and contacts in the reference.
    pub peak_bodies: usize,
    /// Peak contacts in the reference.
    pub peak_contacts: usize,
    /// Measured durations.
    pub timings: Timings,
}

/// Measured durations, each with a declared boundary.
#[derive(Clone, Debug, Default)]
pub struct Timings {
    /// Host, runtime and package set-up plus initial scene construction.
    pub startup: Duration,
    /// Per frame: submit, admit, tick, and the render read.
    pub frames: Vec<Duration>,
    /// Per frame: the render read alone.
    pub render: Vec<Duration>,
    /// Dropping the host and everything it owns.
    pub teardown: Duration,
    /// Per frame: the reference step alone, as the game's own cost.
    pub physics: Vec<Duration>,
    /// Replay of the whole run by applying retained patches.
    pub replay: Duration,
}

/// Runs `scene` under the legacy commitment profile.
pub fn run(scene: Scene, seed: u64) -> Observation {
    run_scene(scene, seed, |state| state)
}

/// Runs `scene` on a worldline with commitment profile `profile`.
#[cfg(feature = "application_root_profile")]
pub fn run_with_profile(
    scene: Scene,
    seed: u64,
    profile: warp_core::application_root::CommitmentProfile,
) -> Observation {
    run_scene(scene, seed, move |state| state.with_commitment_profile(profile))
}

fn run_scene(scene: Scene, seed: u64, profile: impl FnOnce(WorldlineState) -> WorldlineState) -> Observation {
    let mut obs = Observation::default();
    if let Ok(mut active) = ACTIVE.lock() {
        *active = Some((scene, seed));
    }
    let start_world = initial_world(scene, seed);
    let inputs: Vec<[i64; 2]> = (0..scene.frames).map(|f| input(scene, seed, f)).collect();

    // The reference: the game alone, timed per frame.
    let mut reference = start_world.clone();
    for frame_input in &inputs {
        let started = Instant::now();
        reference = step(scene, seed, &reference, *frame_input);
        obs.timings.physics.push(started.elapsed());
        obs.peak_bodies = obs.peak_bodies.max(reference.bodies.len());
        obs.peak_contacts = obs.peak_contacts.max(reference.contacts.len());
    }
    obs.reference_world = reference;

    // Start-up.
    let started = Instant::now();
    let worldline_id = WorldlineId::from_bytes(WORLDLINE);
    let initial = profile(initial_state(scene, &start_world));
    let base = initial.clone();
    let mut runtime = WorldlineRuntime::new();
    runtime
        .register_worldline(worldline_id, initial)
        .expect("worldline registers");
    runtime
        .register_writer_head(WriterHead::with_routing(
            WriterHeadKey {
                worldline_id,
                head_id: make_head_id("f6-game"),
            },
            PlaybackMode::Play,
            InboxPolicy::AcceptAll,
            None,
            true,
        ))
        .expect("writer head registers");
    let mut engine_store = GraphStore::default();
    let engine_root = make_node_id("f6-engine-root");
    engine_store.insert_node(engine_root, NodeRecord { ty: make_type_id("bench/f6/engine-root") });
    let engine = EngineBuilder::new(engine_store, engine_root)
        .scheduler(SchedulerKind::Radix)
        .workers(1)
        .build();
    let mut host = TrustedRuntimeHost::new(runtime, engine).expect("host initializes");
    let registry = Registry;
    host.register_contract_package(package(&registry))
        .expect("package installs");
    obs.timings.startup = started.elapsed();

    // Frames.
    let warp_id = make_warp_id(WARP);
    let mut submissions = Vec::with_capacity(inputs.len());
    for (frame, frame_input) in inputs.iter().enumerate() {
        let frame_started = Instant::now();
        obs.accounting.occurrences += 1;
        obs.accounting.delivery_attempts += 1;
        let envelope = IngressEnvelope::local_intent(
            IngressTarget::DefaultWriter { worldline_id },
            make_intent_kind("echo.intent/eint-v1"),
            echo_wasm_abi::pack_intent_v1(OP_ID, &vars(scene, frame as u64, *frame_input))
                .expect("intent packs"),
        );
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
        let steps = host.tick_once().expect("tick runs");
        if steps.len() == 1 && steps[0].admitted_count == 1 {
            obs.one_tick_frames += 1;
        }
        // The render read: every body's state, as a renderer would copy it.
        let render_started = Instant::now();
        let live = host
            .runtime()
            .worldlines()
            .get(&worldline_id)
            .expect("worldline exists");
        if let Some(store) = live.state().store(&warp_id) {
            let mut seen = 0_u64;
            let mut checksum = 0_i64;
            for cell in 0..scene.cells() {
                for edge in store.edges_from(&cell_node(cell)) {
                    if let Some(AttachmentValue::Atom(atom)) = store.node_attachment(&edge.to) {
                        if let Some(body) = Body::decode(&atom.bytes) {
                            checksum = checksum.wrapping_add(body.pos[0] ^ body.pos[1]);
                            seen += 1;
                        }
                    }
                }
            }
            std::hint::black_box(checksum);
            obs.rendered += seen;
        }
        obs.timings.render.push(render_started.elapsed());
        obs.timings.frames.push(frame_started.elapsed());
        submissions.push(submission.submission_id);
    }

    // Outcomes and the final world, outside measurement.
    for submission in &submissions {
        match host.app().observe_intent_outcome(submission) {
            IntentOutcome::Applied { .. } => obs.accounting.committed += 1,
            IntentOutcome::Pending { .. } => obs.accounting.unfinished += 1,
            _ => obs.accounting.refused += 1,
        }
    }
    {
        let live = host
            .runtime()
            .worldlines()
            .get(&worldline_id)
            .expect("worldline exists");
        obs.live_root = live.state().state_root();
        if let Some(store) = live.state().store(&warp_id) {
            let mut view = GraphView::new(store);
            obs.final_world = read_world(&mut view, scene);
        }
    }
    let started = Instant::now();
    let replayed = host.provenance().replay_worldline_state(worldline_id, &base);
    obs.timings.replay = started.elapsed();
    obs.replay_root = replayed.ok().map(|state| state.state_root());

    // Teardown.
    let started = Instant::now();
    drop(host);
    obs.timings.teardown = started.elapsed();
    if let Ok(mut active) = ACTIVE.lock() {
        *active = None;
    }
    obs
}

/// Validates an observation. Expected counts come from the scene.
pub fn validate(scene: Scene, obs: &Observation) -> ResultClass {
    if obs.accounting.committed + obs.accounting.refused < scene.frames {
        return ResultClass::Incomplete(format!(
            "{} of {} frames finished",
            obs.accounting.committed + obs.accounting.refused,
            scene.frames
        ));
    }
    let mut checks = Checks::default();
    checks.check(
        obs.accounting.committed == scene.frames,
        format!("committed {} of {} frames", obs.accounting.committed, scene.frames),
    );
    checks.check(obs.accounting.refused == 0, "a frame was refused");
    checks.check(
        obs.one_tick_frames == scene.frames,
        format!(
            "{} of {} frames committed exactly one tick",
            obs.one_tick_frames, scene.frames
        ),
    );
    checks.check(
        obs.final_world.as_ref() == Some(&obs.reference_world),
        "the world in the graph differs from the reference simulation",
    );
    checks.check(
        obs.replay_root == Some(obs.live_root),
        "replay by applying retained patches did not reproduce the live root",
    );
    checks.class()
}
