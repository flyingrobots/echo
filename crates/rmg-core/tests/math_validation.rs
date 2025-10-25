use once_cell::sync::Lazy;
use serde::Deserialize;

use rmg_core::math::{self, Mat4, Prng, Quat, Vec3};

static FIXTURES: Lazy<MathFixtures> = Lazy::new(|| {
    let raw = include_str!("fixtures/math-fixtures.json");
    serde_json::from_str(raw).expect("math fixtures")
});

#[derive(Debug, Deserialize)]
struct MathFixtures {
    scalars: ScalarFixtures,
    vec3: Vec3Fixtures,
    mat4: Mat4Fixtures,
    quat: QuatFixtures,
    prng: Vec<PrngFixture>,
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
    transform_vec3: Vec<Mat4Vec3Fixture>,
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

const EPS: f32 = 1e-6;

fn assert_scalar(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPS,
        "expected {expected}, got {actual}"
    );
}

fn assert_vec3(actual: Vec3, expected: [f32; 3]) {
    let arr = actual.to_array();
    for (i, (a, e)) in arr.iter().zip(expected.iter()).enumerate() {
        assert!((a - e).abs() <= EPS, "index {i}: expected {e}, got {a}");
    }
}

fn assert_quat(actual: Quat, expected: [f32; 4]) {
    let arr = actual.to_array();
    for (i, (a, e)) in arr.iter().zip(expected.iter()).enumerate() {
        assert!((a - e).abs() <= EPS, "index {i}: expected {e}, got {a}");
    }
}

fn assert_mat4(actual: Mat4, expected: [f32; 16]) {
    let arr = actual.to_array();
    for (i, (a, e)) in arr.iter().zip(expected.iter()).enumerate() {
        assert!((a - e).abs() <= EPS, "index {i}: expected {e}, got {a}");
    }
}

#[test]
fn scalar_fixtures_all_match() {
    for fix in &FIXTURES.scalars.clamp {
        let actual = math::clamp(fix.value, fix.min, fix.max);
        assert_scalar(actual, fix.expected);
    }

    for fix in &FIXTURES.scalars.deg_to_rad {
        let actual = math::deg_to_rad(fix.value);
        assert_scalar(actual, fix.expected);
    }

    for fix in &FIXTURES.scalars.rad_to_deg {
        let actual = math::rad_to_deg(fix.value);
        assert_scalar(actual, fix.expected);
    }
}

#[test]
fn vec3_fixtures_cover_operations() {
    for fix in &FIXTURES.vec3.add {
        let a = Vec3::from(fix.a);
        let b = Vec3::from(fix.b);
        let actual = a.add(&b);
        assert_vec3(actual, fix.expected);
    }

    for fix in &FIXTURES.vec3.dot {
        let a = Vec3::from(fix.a);
        let b = Vec3::from(fix.b);
        let actual = a.dot(&b);
        assert_scalar(actual, fix.expected);
    }

    for fix in &FIXTURES.vec3.cross {
        let a = Vec3::from(fix.a);
        let b = Vec3::from(fix.b);
        let actual = a.cross(&b);
        assert_vec3(actual, fix.expected);
    }

    for fix in &FIXTURES.vec3.length {
        let value = Vec3::from(fix.value);
        let actual = value.length();
        match &fix.expected {
            FixtureValue::Scalar(exp) => assert_scalar(actual, *exp),
            FixtureValue::Vector(_) => panic!("length fixture expected scalar"),
        }
    }

    for fix in &FIXTURES.vec3.normalize {
        let value = Vec3::from(fix.value);
        let actual = value.normalize();
        match &fix.expected {
            FixtureValue::Scalar(_) => panic!("normalize fixture expected vector"),
            FixtureValue::Vector(exp) => assert_vec3(actual, *exp),
        }
    }
}

#[test]
fn mat4_fixtures_validate_transformations() {
    for fix in &FIXTURES.mat4.multiply {
        let a = Mat4::from(fix.a);
        let b = Mat4::from(fix.b);
        let actual = a.multiply(&b);
        assert_mat4(actual, fix.expected);
    }

    for fix in &FIXTURES.mat4.transform_vec3 {
        let matrix = Mat4::from(fix.matrix);
        let vector = Vec3::from(fix.vector);
        let actual = matrix.transform_point(&vector);
        assert_vec3(actual, fix.expected);
    }
}

#[test]
fn quat_fixtures_validate_operations() {
    for fix in &FIXTURES.quat.from_axis_angle {
        let axis = Vec3::from(fix.axis);
        let actual = Quat::from_axis_angle(axis, fix.angle);
        assert_quat(actual, fix.expected);
    }

    for fix in &FIXTURES.quat.multiply {
        let a = Quat::from(fix.a);
        let b = Quat::from(fix.b);
        let actual = a.multiply(&b);
        assert_quat(actual, fix.expected);
    }

    for fix in &FIXTURES.quat.normalize {
        let value = Quat::from(fix.value);
        let actual = value.normalize();
        assert_quat(actual, fix.expected);
    }

    for fix in &FIXTURES.quat.to_mat4 {
        let value = Quat::from(fix.value);
        let actual = value.to_mat4();
        assert_mat4(actual, fix.expected);
    }
}

#[test]
fn prng_fixture_replays_sequence() {
    for fix in &FIXTURES.prng {
        let mut prng = Prng::from_seed(fix.seed[0], fix.seed[1]);

        for (i, expected) in fix.expected_next.iter().enumerate() {
            let actual = prng.next_f32();
            assert!(
                (actual - expected).abs() <= EPS,
                "index {i}: expected {expected}, got {actual}"
            );
        }

        if let Some(int_fixture) = &fix.expected_ints {
            for (i, expected) in int_fixture.values.iter().enumerate() {
                let actual = prng.next_int(int_fixture.min, int_fixture.max);
                assert_eq!(
                    actual, *expected,
                    "int index {i}: expected {expected}, got {actual}"
                );
            }
        }
    }
}
