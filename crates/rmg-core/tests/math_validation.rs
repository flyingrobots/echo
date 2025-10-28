//! Deterministic math validation harness for the motion rewrite spike.
//!
//! Ensures scalar, vector, matrix, quaternion, and PRNG behaviour stays
//! consistent with the documented fixtures across platforms.

use once_cell::sync::Lazy;
use serde::Deserialize;

use rmg_core::math::{self, Mat4, Prng, Quat, Vec3};

// Path is documented in repo; kept for developer reference.
#[allow(dead_code)]
const FIXTURE_PATH: &str = "crates/rmg-core/tests/fixtures/math-fixtures.json";
static RAW_FIXTURES: &str = include_str!("fixtures/math-fixtures.json");

static FIXTURES: Lazy<MathFixtures> = Lazy::new(|| {
    // Keep message simple to satisfy clippy while still informative.
    let fixtures: MathFixtures = serde_json::from_str(RAW_FIXTURES)
        .expect("failed to parse math fixtures");
    fixtures.validate();
    fixtures
});

#[derive(Debug, Deserialize)]
struct MathFixtures {
    #[serde(default)]
    tolerance: Tolerance,
    scalars: ScalarFixtures,
    vec3: Vec3Fixtures,
    mat4: Mat4Fixtures,
    quat: QuatFixtures,
    prng: Vec<PrngFixture>,
}

impl MathFixtures {
    fn validate(&self) {
        fn ensure<T>(name: &str, slice: &[T]) {
            assert!(
                !slice.is_empty(),
                "math fixtures set '{name}' must not be empty (len={})",
                slice.len()
            );
        }

        ensure("scalars.clamp", &self.scalars.clamp);
        ensure("scalars.deg_to_rad", &self.scalars.deg_to_rad);
        ensure("scalars.rad_to_deg", &self.scalars.rad_to_deg);
        ensure("vec3.add", &self.vec3.add);
        ensure("vec3.dot", &self.vec3.dot);
        ensure("vec3.cross", &self.vec3.cross);
        ensure("vec3.length", &self.vec3.length);
        ensure("vec3.normalize", &self.vec3.normalize);
        ensure("mat4.multiply", &self.mat4.multiply);
        ensure("mat4.transform_point", &self.mat4.transform_point);
        ensure("mat4.transform_direction", &self.mat4.transform_direction);
        ensure("quat.from_axis_angle", &self.quat.from_axis_angle);
        ensure("quat.multiply", &self.quat.multiply);
        ensure("quat.normalize", &self.quat.normalize);
        ensure("quat.to_mat4", &self.quat.to_mat4);
        ensure("prng", &self.prng);
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Tolerance {
    #[serde(default = "Tolerance::default_absolute")]
    absolute: f32,
    #[serde(default = "Tolerance::default_relative")]
    relative: f32,
}

impl Tolerance {
    const fn default_absolute() -> f32 {
        1e-6
    }

    const fn default_relative() -> f32 {
        1e-6
    }

    fn allowed_error(&self, reference: f32) -> f32 {
        self.absolute.max(self.relative * reference.abs())
    }
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            absolute: Self::default_absolute(),
            relative: Self::default_relative(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ScalarFixtures {
    clamp: Vec<ClampFixture>,
    deg_to_rad: Vec<UnaryFixture>,
    rad_to_deg: Vec<UnaryFixture>,
}

#[derive(Debug, Deserialize)]
struct ClampFixture {
    value: f32,
    min: f32,
    max: f32,
    expected: f32,
}

#[derive(Debug, Deserialize)]
struct UnaryFixture {
    value: f32,
    expected: f32,
}

#[derive(Debug, Deserialize)]
struct Vec3Fixtures {
    add: Vec<Vec3BinaryFixture>,
    dot: Vec<Vec3DotFixture>,
    cross: Vec<Vec3BinaryFixture>,
    length: Vec<Vec3UnaryFixture>,
    normalize: Vec<Vec3UnaryFixture>,
}

#[derive(Debug, Deserialize)]
struct Vec3BinaryFixture {
    a: [f32; 3],
    b: [f32; 3],
    expected: [f32; 3],
}

#[derive(Debug, Deserialize)]
struct Vec3DotFixture {
    a: [f32; 3],
    b: [f32; 3],
    expected: f32,
}

#[derive(Debug, Deserialize)]
struct Vec3UnaryFixture {
    value: [f32; 3],
    expected: FixtureValue,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FixtureValue {
    Scalar(f32),
    Vector([f32; 3]),
}

#[derive(Debug, Deserialize)]
struct Mat4Fixtures {
    multiply: Vec<Mat4BinaryFixture>,
    #[serde(rename = "transform_point")]
    transform_point: Vec<Mat4Vec3Fixture>,
    #[serde(rename = "transform_direction")]
    transform_direction: Vec<Mat4Vec3Fixture>,
}

#[derive(Debug, Deserialize)]
struct Mat4BinaryFixture {
    a: [f32; 16],
    b: [f32; 16],
    expected: [f32; 16],
}

#[derive(Debug, Deserialize)]
struct Mat4Vec3Fixture {
    matrix: [f32; 16],
    vector: [f32; 3],
    expected: [f32; 3],
}

#[derive(Debug, Deserialize)]
struct QuatFixtures {
    from_axis_angle: Vec<QuatAxisAngleFixture>,
    multiply: Vec<QuatBinaryFixture>,
    normalize: Vec<QuatUnaryFixture>,
    to_mat4: Vec<QuatMat4Fixture>,
}

#[derive(Debug, Deserialize)]
struct QuatAxisAngleFixture {
    axis: [f32; 3],
    angle: f32,
    expected: [f32; 4],
}

#[derive(Debug, Deserialize)]
struct QuatBinaryFixture {
    a: [f32; 4],
    b: [f32; 4],
    expected: [f32; 4],
}

#[derive(Debug, Deserialize)]
struct QuatUnaryFixture {
    value: [f32; 4],
    expected: [f32; 4],
}

#[derive(Debug, Deserialize)]
struct QuatMat4Fixture {
    value: [f32; 4],
    expected: [f32; 16],
}

#[derive(Debug, Deserialize)]
struct PrngFixture {
    seed: [u64; 2],
    expected_next: Vec<f32>,
    #[serde(default)]
    expected_ints: Option<PrngIntFixture>,
}

#[derive(Debug, Deserialize)]
struct PrngIntFixture {
    min: i32,
    max: i32,
    values: Vec<i32>,
}

fn assert_scalar(actual: f32, expected: f32, tol: &Tolerance, ctx: &str) {
    let diff = (actual - expected).abs();
    let allowed = tol.allowed_error(expected);
    assert!(
        diff <= allowed,
        "{ctx}: expected {expected}, got {actual} (diff {diff} > {allowed})"
    );
}

fn assert_vec3(actual: Vec3, expected: [f32; 3], tol: &Tolerance, ctx: &str) {
    let arr = actual.to_array();
    for (i, (a, e)) in arr.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        let allowed = tol.allowed_error(*e);
        assert!(
            diff <= allowed,
            "{ctx}[{i}]: expected {e}, got {a} (diff {diff} > {allowed})"
        );
    }
}

fn assert_quat(actual: Quat, expected: [f32; 4], tol: &Tolerance, ctx: &str) {
    let arr = actual.to_array();
    for (i, (a, e)) in arr.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        let allowed = tol.allowed_error(*e);
        assert!(
            diff <= allowed,
            "{ctx}[{i}]: expected {e}, got {a} (diff {diff} > {allowed})"
        );
    }
}

fn assert_mat4(actual: Mat4, expected: [f32; 16], tol: &Tolerance, ctx: &str) {
    let arr = actual.to_array();
    for (i, (a, e)) in arr.iter().zip(expected.iter()).enumerate() {
        let diff = (a - e).abs();
        let allowed = tol.allowed_error(*e);
        assert!(
            diff <= allowed,
            "{ctx}[{i}]: expected {e}, got {a} (diff {diff} > {allowed})"
        );
    }
}

#[test]
fn scalar_fixtures_all_match() {
    let tol = &FIXTURES.tolerance;
    for fix in &FIXTURES.scalars.clamp {
        let actual = math::clamp(fix.value, fix.min, fix.max);
        assert_scalar(
            actual,
            fix.expected,
            tol,
            &format!(
                "scalars.clamp value={}, range=[{}, {}]",
                fix.value, fix.min, fix.max
            ),
        );
    }

    for fix in &FIXTURES.scalars.deg_to_rad {
        let actual = math::deg_to_rad(fix.value);
        assert_scalar(
            actual,
            fix.expected,
            tol,
            &format!("scalars.deg_to_rad value={}", fix.value),
        );
    }

    for fix in &FIXTURES.scalars.rad_to_deg {
        let actual = math::rad_to_deg(fix.value);
        assert_scalar(
            actual,
            fix.expected,
            tol,
            &format!("scalars.rad_to_deg value={}", fix.value),
        );
    }
}

#[test]
fn vec3_fixtures_cover_operations() {
    let tol = &FIXTURES.tolerance;
    for fix in &FIXTURES.vec3.add {
        let a = Vec3::from(fix.a);
        let b = Vec3::from(fix.b);
        let actual = a.add(&b);
        assert_vec3(
            actual,
            fix.expected,
            tol,
            &format!("vec3.add a={:?} b={:?}", fix.a, fix.b),
        );
    }

    for fix in &FIXTURES.vec3.dot {
        let a = Vec3::from(fix.a);
        let b = Vec3::from(fix.b);
        let actual = a.dot(&b);
        assert_scalar(
            actual,
            fix.expected,
            tol,
            &format!("vec3.dot a={:?} b={:?}", fix.a, fix.b),
        );
    }

    for fix in &FIXTURES.vec3.cross {
        let a = Vec3::from(fix.a);
        let b = Vec3::from(fix.b);
        let actual = a.cross(&b);
        assert_vec3(
            actual,
            fix.expected,
            tol,
            &format!("vec3.cross a={:?} b={:?}", fix.a, fix.b),
        );
    }

    for (idx, fix) in FIXTURES.vec3.length.iter().enumerate() {
        let value = Vec3::from(fix.value);
        let actual = value.length();
        match &fix.expected {
            FixtureValue::Scalar(exp) => assert_scalar(
                actual,
                *exp,
                tol,
                &format!("vec3.length#[{idx}] value={:?}", fix.value),
            ),
            FixtureValue::Vector(v) => panic!(
                "vec3.length fixture #[{idx}] (value={:?}) expected scalar but got vector {:?}",
                fix.value, v
            ),
        }
    }

    for (idx, fix) in FIXTURES.vec3.normalize.iter().enumerate() {
        let value = Vec3::from(fix.value);
        let actual = value.normalize();
        match &fix.expected {
            FixtureValue::Scalar(s) => panic!(
                "vec3.normalize fixture #[{idx}] (value={:?}) expected vector but got scalar {}",
                fix.value, s
            ),
            FixtureValue::Vector(exp) => assert_vec3(
                actual,
                *exp,
                tol,
                &format!("vec3.normalize#[{idx}] value={:?}", fix.value),
            ),
        }
    }
}

#[test]
fn mat4_fixtures_validate_transformations() {
    let tol = &FIXTURES.tolerance;
    for (i, fix) in FIXTURES.mat4.multiply.iter().enumerate() {
        let a = Mat4::from(fix.a);
        let b = Mat4::from(fix.b);
        let actual = a.multiply(&b);
        let context = format!("mat4.multiply[{}] a0={:.3} b0={:.3}", i, fix.a[0], fix.b[0]);
        assert_mat4(actual, fix.expected, tol, &context);
    }

    for fix in &FIXTURES.mat4.transform_point {
        let matrix = Mat4::from(fix.matrix);
        let vector = Vec3::from(fix.vector);
        // Fixture vectors are treated as points (homogeneous w = 1).
        let actual = matrix.transform_point(&vector);
        assert_vec3(
            actual,
            fix.expected,
            tol,
            &format!("mat4.transform_point vector={:?}", fix.vector),
        );
    }

    for fix in &FIXTURES.mat4.transform_direction {
        let matrix = Mat4::from(fix.matrix);
        let vector = Vec3::from(fix.vector);
        let actual = matrix.transform_direction(&vector);
        assert_vec3(
            actual,
            fix.expected,
            tol,
            &format!("mat4.transform_direction vector={:?}", fix.vector),
        );
    }
}

#[test]
fn quat_fixtures_validate_operations() {
    let tol = &FIXTURES.tolerance;
    for fix in &FIXTURES.quat.from_axis_angle {
        let axis = Vec3::from(fix.axis);
        let actual = Quat::from_axis_angle(axis, fix.angle);
        assert_quat(
            actual,
            fix.expected,
            tol,
            &format!(
                "quat.from_axis_angle axis={:?} angle={}",
                fix.axis, fix.angle
            ),
        );
    }

    for fix in &FIXTURES.quat.multiply {
        let a = Quat::from(fix.a);
        let b = Quat::from(fix.b);
        let actual = a.multiply(&b);
        assert_quat(
            actual,
            fix.expected,
            tol,
            &format!("quat.multiply a={:?} b={:?}", fix.a, fix.b),
        );
    }

    for fix in &FIXTURES.quat.normalize {
        let value = Quat::from(fix.value);
        let actual = value.normalize();
        assert_quat(
            actual,
            fix.expected,
            tol,
            &format!("quat.normalize value={:?}", fix.value),
        );
    }

    for fix in &FIXTURES.quat.to_mat4 {
        let value = Quat::from(fix.value);
        let actual = value.to_mat4();
        assert_mat4(
            actual,
            fix.expected,
            tol,
            &format!("quat.to_mat4 value={:?}", fix.value),
        );
    }
}

#[test]
fn prng_fixture_replays_sequence() {
    for fix in &FIXTURES.prng {
        let mut prng = Prng::from_seed(fix.seed[0], fix.seed[1]);

        let tol = &FIXTURES.tolerance;

        for (i, expected) in fix.expected_next.iter().enumerate() {
            let actual = prng.next_f32();
            assert_scalar(
                actual,
                *expected,
                tol,
                &format!("prng.expected_next seed={:?} index={i}", fix.seed),
            );
        }

        if let Some(int_fixture) = &fix.expected_ints {
            let mut prng = Prng::from_seed(fix.seed[0], fix.seed[1]);
            let actual: Vec<i32> = int_fixture
                .values
                .iter()
                .map(|_| prng.next_int(int_fixture.min, int_fixture.max))
                .collect();
            assert_eq!(
                actual, int_fixture.values,
                "prng.expected_ints seed={:?} expected {:?} got {:?}",
                fix.seed, int_fixture.values, actual
            );
        }
    }
}
