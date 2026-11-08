// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Conformance tests for the benchmark scene fixtures.
//!
//! These run in the ordinary test suite: generator vectors, the separation of
//! workload and expectation identities, C1 parity with `action_scene`, one
//! validated F5-S execution, validator canaries that must be invalid, small
//! validated F6 scenes, and hand-computed F6 scenes that check collision,
//! despawn and contact edges independently of the game's own step.
#![cfg(all(feature = "native_rule_bootstrap", feature = "trusted_runtime"))]
#![allow(missing_docs, dead_code, clippy::expect_used, clippy::panic)]

#[path = "../benches/support/action_scene.rs"]
mod action_scene;
#[path = "../benches/support/fixtures/mod.rs"]
mod fixtures;

use fixtures::manifest::hex32;
use fixtures::outcome::ResultClass;
use fixtures::prng::SplitMix64;
use fixtures::{game, star, think_append};

/// SplitMix64 from seed 1234567: the reference sequence, cross-checked with
/// an independent Python implementation of Vigna's code.
#[test]
fn splitmix64_matches_reference_vectors() {
    let mut rng = SplitMix64::new(1_234_567);
    let expected = [
        6_457_827_717_110_365_317_u64,
        3_203_168_211_198_807_973,
        9_817_491_932_198_370_423,
        4_593_380_528_125_082_431,
        16_408_922_859_458_223_821,
    ];
    for want in expected {
        assert_eq!(rng.next_u64(), want);
    }
}

/// The bounded-integer mapping, pinned separately from the output sequence,
/// including a bound above 2^63 where rejection matters.
#[test]
fn bounded_mapping_matches_vectors() {
    let cases: [(u64, [u64; 6]); 3] = [
        (10, [3, 1, 8, 4, 0, 2]),
        (7, [5, 5, 0, 2, 6, 4]),
        (
            (1 << 63) + 1,
            [
                4_456_085_495_900_499_604,
                6_792_609_088_808_213_253,
                5_545_679_290_133_000_099,
                2_185_608_355_395_893_165,
                247_114_729_376_335_589,
                369_180_215_851_445_686,
            ],
        ),
    ];
    for (bound, want) in cases {
        let mut rng = SplitMix64::new(42);
        let got: Vec<u64> = (0..6)
            .map(|_| rng.below(bound).expect("non-zero bound"))
            .collect();
        assert_eq!(got, want, "bound {bound}");
    }
    assert_eq!(SplitMix64::new(42).below(0), None);
}

#[test]
fn bytes_are_little_endian_outputs_truncated() {
    let first = SplitMix64::new(9).next_u64().to_le_bytes();
    let got = SplitMix64::new(9).bytes(10);
    assert_eq!(got.len(), 10);
    assert_eq!(&got[..8], &first);
}

#[test]
fn goldens_do_not_change_the_workload_identity() {
    let spec = think_append::workload(think_append::S, 7);
    let id = spec.workload_id();
    let a = think_append::expectation(think_append::S, None);
    let b = think_append::expectation(think_append::S, Some("00"));
    assert_eq!(spec.workload_id(), id);
    assert_ne!(a.expectation_id(&id), b.expectation_id(&id));
    assert_ne!(think_append::workload(think_append::S, 8).workload_id(), id);
    assert_ne!(think_append::workload(think_append::M, 7).workload_id(), id);
}

#[test]
fn c1_reproduces_the_action_scene_state_root() {
    for cells in [1_usize, 100, 1_000] {
        let scene = action_scene::Scene::new(
            cells,
            action_scene::Durability::None,
            std::path::Path::new("target"),
        );
        let scene_root = scene
            .host
            .runtime()
            .worldlines()
            .get(&warp_core::WorldlineId::from_bytes([67; 32]))
            .expect("scene worldline")
            .state()
            .state_root();
        assert_eq!(star::state_root(cells as u64), scene_root, "{cells} cells");
    }
}

/// The C1 root at 1,000 cells, under state-root format 1.
const C1_1000_ROOT: &str = "2a2f58e3ba1313c1c21918af58c4ab872dcb4bf871360b1f4f7592f4c2760f5f";

#[test]
fn c1_root_matches_its_golden() {
    let expect = star::expectation(1_000, Some(C1_1000_ROOT));
    assert_eq!(
        Some(hex32(&star::state_root(1_000)).as_str()),
        expect.get("state_root_v1")
    );
}

/// The F5-S script for seed 7, version 1 of the generator.
const F5_S_SEED7_SCRIPT: &str = "efc633360e2e237b9ed238e1cb0f57b8c12e64c779a52bba6e223c9bdfd33d4d";

#[test]
fn f5_script_matches_its_golden() {
    let script = think_append::script(think_append::S, 7);
    assert_eq!(script.len(), 200);
    assert_eq!(
        hex32(&think_append::script_digest(&script)),
        F5_S_SEED7_SCRIPT
    );
}

fn f5_s() -> (
    Vec<think_append::Append>,
    fixtures::manifest::Expectation,
    think_append::Observation,
) {
    let script = think_append::script(think_append::S, 7);
    let expect = think_append::expectation(think_append::S, Some(F5_S_SEED7_SCRIPT));
    let obs = think_append::run(think_append::S, 7);
    (script, expect, obs)
}

#[test]
fn f5_s_executes_validly_and_canaries_are_rejected() {
    let tier = think_append::S;
    let (script, expect, obs) = f5_s();
    assert_eq!(
        think_append::validate(tier, &script, &expect, &obs),
        ResultClass::Valid
    );

    let invalid = |obs: &think_append::Observation| {
        let class = think_append::validate(tier, &script, &expect, obs);
        assert!(!class.comparable(), "canary was accepted: {class:?}");
        class
    };

    // A missing accepted operation.
    let mut canary = obs.clone();
    canary.steps.pop();
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // An incorrect historical reading.
    let mut canary = obs.clone();
    canary.checkpoints[0].2 = Some([0xAB; 32]);
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // Replay that failed.
    let mut canary = obs.clone();
    canary.replay_root = None;
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // A duplicated authored effect: two commits for one occurrence.
    let mut canary = obs.clone();
    let last = *canary.steps.last().expect("steps");
    canary.steps.push(last);
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // An entry missing from the final state.
    let mut canary = obs.clone();
    canary.entries_verified -= 1;
    assert!(matches!(invalid(&canary), ResultClass::Invalid(_)));

    // Work left unfinished.
    let mut canary = obs.clone();
    canary.accounting.committed -= 1;
    canary.accounting.unfinished = 1;
    assert!(matches!(invalid(&canary), ResultClass::Incomplete(_)));

    // A workload that is not the pinned one.
    let wrong = think_append::expectation(tier, Some("00"));
    assert!(matches!(
        think_append::validate(tier, &script, &wrong, &obs),
        ResultClass::Invalid(_)
    ));
}

#[test]
fn f5_rejects_an_observation_from_a_different_executed_script() {
    let tier = think_append::S;
    let expected_script = think_append::script(tier, 7);
    let other_script = think_append::script(tier, 8);
    assert_ne!(
        think_append::script_digest(&expected_script),
        think_append::script_digest(&other_script)
    );
    let obs = think_append::run(tier, 8);
    let pinned = think_append::expectation(tier, Some(F5_S_SEED7_SCRIPT));
    let unpinned = think_append::expectation(tier, None);
    for expect in [&pinned, &unpinned] {
        let class = think_append::validate(tier, &expected_script, expect, &obs);
        assert!(
            matches!(class, ResultClass::Invalid(_)),
            "accepted an observation from a different script: {class:?}"
        );
    }
    assert_eq!(
        think_append::validate(tier, &other_script, &unpinned, &obs),
        ResultClass::Valid
    );
}

#[cfg(feature = "application_root_shadow")]
#[test]
fn f5_rejects_missing_or_failed_application_root_parity() {
    let tier = think_append::S;
    let (script, expect, obs) = f5_s();
    assert_eq!(obs.application_root, Some((true, true)));
    assert_eq!(
        think_append::validate(tier, &script, &expect, &obs),
        ResultClass::Valid
    );
    for parity in [
        None,
        Some((false, false)),
        Some((false, true)),
        Some((true, false)),
    ] {
        let mut canary = obs.clone();
        canary.application_root = parity;
        let class = think_append::validate(tier, &script, &expect, &canary);
        assert!(
            matches!(class, ResultClass::Invalid(_)),
            "accepted missing or failed application-root parity {parity:?}: {class:?}"
        );
    }
}

/// The scaled F6 scenes the hosted conformance test runs.
const SMALL_SCENES: [game::Scene; 3] = [
    game::ARENA.scaled(1, 8, 30),
    game::PILE.scaled(1, 16, 110),
    game::BULLETS.scaled(1, 8, 70),
];

/// Pinned live roots of the small scenes at seed 6, as `hex32`. Regenerate
/// (the failure message prints the observed root) whenever the game, the
/// graph layout or `FIXTURE_VERSION` changes.
const SMALL_SCENE_ROOTS: [&str; 3] = [
    "76a94d9c932dd31433f9e06636ee13f1a36d8cee3390c99da2e4d91d3bed632a",
    "51b539864ad690800c02fcdafc5935197bfd501a2689eb60719f624a5720627f",
    "5395d6af41ae131d9088db24c242e60eea10eb207097cf653e94ab71a4619cec",
];

/// Pinned blake3 digests of each full-size scene's generated initial world at
/// seed 6: the concatenated encodings of its bodies in id order. Regenerate
/// (the failure message prints the observed digest) whenever the generator
/// changes.
const INITIAL_WORLD_DIGESTS: [&str; 3] = [
    "4a0074099f7e3ee1d2eef31fddf3482f36e42ac8516ff9eb09dc080bb5d71e09",
    "a872542db06f4284e9c231358ff5207edbb2e7afa801d7b4b896f0f4331c6ed1",
    "e1b3f46bfe2d54dd01520141f604f198fbb731dc57ad6b4dd7feda5318f6dda0",
];

/// Small F6 scenes run validly in a debug build, where footprint enforcement
/// checks that the step declares every read and write it makes. The pile runs
/// long enough to land and pile up, so contact edges are added (the hand
/// scenes below check removal); the bullets run past a projectile lifetime,
/// so bodies despawn. Each scene runs once; its live root must match the pin.
#[test]
fn f6_small_scenes_run_validly() {
    let mut observations = std::collections::BTreeMap::new();
    let mut moved = Vec::new();
    for (scene, pinned) in SMALL_SCENES.into_iter().zip(SMALL_SCENE_ROOTS) {
        let obs = game::run(scene, 6);
        assert_eq!(
            game::validate(scene, &game::expectation(scene), &obs),
            ResultClass::Valid,
            "{}",
            scene.name
        );
        assert!(obs.rendered > 0, "{} rendered nothing", scene.name);
        let root = hex32(&obs.live_root);
        if root != pinned {
            moved.push(format!("{}: live root is {root}", scene.name));
        }
        observations.insert(scene.name, obs);
    }
    assert!(
        moved.is_empty(),
        "live roots moved; repin if the change is intended:\n{}",
        moved.join("\n")
    );
    assert!(
        observations["pile"].peak_contacts > 0,
        "the pile never made contact"
    );
    let bullets = &observations["bullets"].reference_world;
    assert!(
        bullets.next_id as usize > bullets.bodies.len(),
        "no projectile despawned"
    );
}

/// Each full-size scene's generated initial world matches its pinned digest,
/// so a generator change cannot pass unnoticed.
#[test]
fn f6_initial_worlds_match_their_pinned_digests() {
    let mut moved = Vec::new();
    for (scene, pinned) in game::SCENES.into_iter().zip(INITIAL_WORLD_DIGESTS) {
        let world = game::initial_world(scene, 6);
        let mut hasher = blake3::Hasher::new();
        for body in world.bodies.values() {
            hasher.update(&body.encode());
        }
        let digest: [u8; 32] = hasher.finalize().into();
        let digest = hex32(&digest);
        if digest != pinned {
            moved.push(format!("{}: initial world digest is {digest}", scene.name));
        }
    }
    assert!(
        moved.is_empty(),
        "initial worlds moved; repin if the change is intended:\n{}",
        moved.join("\n")
    );
}

/// Every scene's cell is at least twice its radius, the rule the broadphase
/// relies on to find every touching pair in a 3x3 neighbourhood.
#[test]
fn f6_scenes_satisfy_the_cell_rule() {
    for scene in game::SCENES.into_iter().chain([HAND]) {
        assert!(
            scene.cell >= 2 * scene.radius,
            "{}: cell {} is smaller than twice the radius {}",
            scene.name,
            scene.cell,
            scene.radius
        );
    }
}

/// The step intent's variables round-trip: `parse` reads back exactly what
/// `vars` packed, for every scene, so adding a field cannot silently skew the
/// hand-computed offsets.
#[test]
fn f6_step_vars_round_trip() {
    for scene in game::SCENES.into_iter().chain([HAND]) {
        let (parsed, seed, frame, input) =
            game::parse(&game::vars(scene, 6, 17, [3, -5])).expect("vars parse");
        assert_eq!((seed, frame, input), (6, 17, [3, -5]), "{}", scene.name);
        // The name is resolved from the index, so a hand scene parses back
        // with the indexed scene's name; every dimension must round-trip.
        let expected = game::Scene {
            name: parsed.name,
            ..scene
        };
        assert_eq!(
            format!("{parsed:?}"),
            format!("{expected:?}"),
            "{}",
            scene.name
        );
    }
}

/// Every full-size F6 scene places its bodies without overlap. The lattice
/// comment in `initial_world` is load-bearing for the pile, whose 2,048
/// bodies once overflowed a 512-unit world onto the floor, so it is checked
/// at the real dimensions rather than the scaled ones the hosted test uses.
#[test]
fn f6_full_size_initial_worlds_do_not_overlap() {
    for scene in game::SCENES {
        let world = game::initial_world(scene, 6);
        let bodies: Vec<_> = world.bodies.values().collect();
        assert_eq!(
            bodies.len(),
            (scene.bodies + scene.emitters) as usize,
            "{}",
            scene.name
        );
        for (i, a) in bodies.iter().enumerate() {
            for b in &bodies[i + 1..] {
                let dx = i128::from(b.pos[0] - a.pos[0]);
                let dy = i128::from(b.pos[1] - a.pos[1]);
                let reach = i128::from(a.radius + b.radius);
                assert!(
                    dx * dx + dy * dy >= reach * reach,
                    "{}: bodies {} and {} overlap at the start",
                    scene.name,
                    a.id,
                    b.id
                );
            }
        }
    }
}

/// A 64-unit box with 16-unit cells, no gravity, fully elastic, no player
/// and no emitters: the dimensions of the hand-computed F6 scenes below. It
/// shares ARENA's index: `parse` names a scene by index, and every dimension
/// that matters travels in the intent.
const HAND: game::Scene = game::Scene {
    name: "hand",
    index: 0,
    bodies: 0,
    radius: 1,
    world: 64,
    cell: 16,
    gravity: 0,
    restitution: 256,
    initial_speed: 0,
    player: false,
    emitters: 0,
    spawn_per_emitter: 0,
    projectile_ttl: 0,
    projectile_speed: 0,
    frames: 0,
};

fn hand_body(id: u32, kind: game::Kind, pos: [i64; 2], vel: [i64; 2], radius: i64) -> game::Body {
    game::Body {
        id,
        kind,
        pos,
        vel,
        radius,
        ttl: 0,
    }
}

fn hand_world(
    frame: u64,
    next_id: u32,
    bodies: &[game::Body],
    contacts: &[(u32, u32)],
) -> game::World {
    game::World {
        frame,
        next_id,
        bodies: bodies.iter().map(|body| (body.id, *body)).collect(),
        contacts: contacts.iter().copied().collect(),
    }
}

/// Checks `expected` frame by frame, independently of `game::step`: the
/// expected worlds are worked out by hand from the physics, not by running
/// the step. Each prefix also runs hosted in Echo (a debug build, so with
/// footprint enforcement) and the world read back from the graph, including
/// its contact edges, must equal the hand-computed one.
fn check_hand_scene(start: &game::World, expected: &[game::World]) {
    let mut world = start.clone();
    for (frame, want) in expected.iter().enumerate() {
        world = game::step(HAND, 0, &world, [0, 0]);
        assert_eq!(&world, want, "step, frame {}", frame + 1);
        let scene = game::Scene {
            frames: frame as u64 + 1,
            ..HAND
        };
        let obs = game::run_from_world(scene, 0, start.clone());
        assert_eq!(
            game::validate(scene, &game::expectation(scene), &obs),
            ResultClass::Valid,
            "frame {}",
            frame + 1
        );
        assert_eq!(
            obs.final_world.as_ref(),
            Some(want),
            "graph, frame {}",
            frame + 1
        );
    }
}

/// Two equal unit circles approach head-on at one unit per frame each. After
/// integration their centres are one unit apart against a reach of two: they
/// touch. The correction moves each half the overlap apart (to 10.5 and
/// 12.5), and a fully elastic impulse between equal bodies swaps their
/// velocities. The contact edge appears; one frame later they have moved
/// apart to 9.5 and 13.5 and the edge is gone.
#[test]
fn f6_hand_collision_swaps_velocities_and_adds_then_removes_the_contact() {
    use game::{Kind::Dynamic, ONE};
    let y = 10 * ONE;
    let start = hand_world(
        0,
        2,
        &[
            hand_body(0, Dynamic, [10 * ONE, y], [ONE, 0], ONE),
            hand_body(1, Dynamic, [13 * ONE, y], [-ONE, 0], ONE),
        ],
        &[],
    );
    let touching = hand_world(
        1,
        2,
        &[
            hand_body(0, Dynamic, [10 * ONE + ONE / 2, y], [-ONE, 0], ONE),
            hand_body(1, Dynamic, [12 * ONE + ONE / 2, y], [ONE, 0], ONE),
        ],
        &[(0, 1)],
    );
    let parted = hand_world(
        2,
        2,
        &[
            hand_body(0, Dynamic, [9 * ONE + ONE / 2, y], [-ONE, 0], ONE),
            hand_body(1, Dynamic, [13 * ONE + ONE / 2, y], [ONE, 0], ONE),
        ],
        &[],
    );
    check_hand_scene(&start, &[touching, parted]);
}

/// A resting unit circle and a resting half-unit projectile one unit apart
/// overlap by half a unit; each moves a quarter unit away and, at rest, no
/// impulse applies. The projectile starts with two frames to live: it ages
/// to one and stays, in contact. Next frame it ages to zero and despawns
/// before integration, taking its contact edge with it; its number is not
/// reused.
#[test]
fn f6_hand_projectile_despawns_and_drops_its_contact() {
    use game::{
        Kind::{Dynamic, Projectile},
        ONE,
    };
    let y = 30 * ONE;
    let projectile = |pos, ttl| game::Body {
        ttl,
        ..hand_body(1, Projectile, pos, [0, 0], ONE / 2)
    };
    let start = hand_world(
        0,
        2,
        &[
            hand_body(0, Dynamic, [30 * ONE, y], [0, 0], ONE),
            projectile([31 * ONE, y], 2),
        ],
        &[],
    );
    let resting = hand_body(0, Dynamic, [30 * ONE - ONE / 4, y], [0, 0], ONE);
    let aged = hand_world(
        1,
        2,
        &[resting, projectile([31 * ONE + ONE / 4, y], 1)],
        &[(0, 1)],
    );
    let despawned = hand_world(2, 2, &[resting], &[]);
    check_hand_scene(&start, &[aged, despawned]);
}
