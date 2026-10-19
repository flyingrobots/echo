// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Fixture manifests: what the workload is, and what must hold afterwards.
//!
//! The two parts are digested separately:
//!
//! ```text
//! workload_id    = digest(canonical workload specification)
//! expectation_id = digest(workload_id + assertions + output format versions)
//! ```
//!
//! Goldens (expected roots, script digests, counts) live in [`Expectation`],
//! so changing a golden never changes the workload identity.

use echo_edict_canonical::{digest_canonical_value_bytes_v1, CanonicalValueV1 as Value};

use super::prng;

const WORKLOAD_DOMAIN: &str = "echo.bench.fixture.workload/v1";
const EXPECTATION_DOMAIN: &str = "echo.bench.fixture.expectation/v1";

/// How the scene and its history were made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Construction {
    /// The store was built directly, without admitted history.
    Direct,
    /// History was built by operations admitted through the runtime.
    AdmittedOperations,
}

impl Construction {
    /// Name recorded in manifests and reports.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::AdmittedOperations => "admitted-operations",
        }
    }
}

/// What kind of evidence a fixture rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    /// Derived from a recorded workload of the application.
    Observed,
    /// Derived from the application's code or design.
    Modeled,
    /// A shape worth measuring, not yet tied to a concrete user.
    Exploratory,
}

impl EvidenceClass {
    /// Name recorded in manifests and reports.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Modeled => "modeled",
            Self::Exploratory => "exploratory",
        }
    }
}

/// The canonical workload specification. Independent of Echo's implementation.
#[derive(Clone, Debug)]
pub struct WorkloadSpec {
    /// Fixture name, such as `C1` or `F5`.
    pub fixture: &'static str,
    /// Bumped whenever the fixture's generator logic changes.
    pub fixture_version: u32,
    /// Seed for [`prng::SplitMix64`].
    pub seed: u64,
    /// Construction method.
    pub construction: Construction,
    /// Evidence class.
    pub evidence: EvidenceClass,
    /// Requested dimensions, in a fixed order.
    pub dimensions: Vec<(&'static str, u64)>,
    /// Admission, retention, checkpoint and durability settings in force.
    pub settings: Vec<(&'static str, &'static str)>,
    /// History operations the fixture runs, by name.
    pub history_operations: Vec<&'static str>,
}

impl WorkloadSpec {
    fn canonical(&self) -> Value {
        let text = |s: &str| Value::Text(s.to_owned());
        Value::Map(vec![
            (text("fixture"), text(self.fixture)),
            (
                text("fixture_version"),
                Value::Integer(i128::from(self.fixture_version)),
            ),
            (
                text("generator"),
                Value::Map(vec![
                    (text("algorithm"), text(prng::ALGORITHM)),
                    (
                        text("version"),
                        Value::Integer(i128::from(prng::GENERATOR_VERSION)),
                    ),
                ]),
            ),
            (text("seed"), Value::Integer(i128::from(self.seed))),
            (text("construction"), text(self.construction.label())),
            (text("evidence"), text(self.evidence.label())),
            (
                text("dimensions"),
                Value::Array(
                    self.dimensions
                        .iter()
                        .map(|(k, v)| Value::Array(vec![text(k), Value::Integer(i128::from(*v))]))
                        .collect(),
                ),
            ),
            (
                text("settings"),
                Value::Array(
                    self.settings
                        .iter()
                        .map(|(k, v)| Value::Array(vec![text(k), text(v)]))
                        .collect(),
                ),
            ),
            (
                text("history_operations"),
                Value::Array(self.history_operations.iter().map(|op| text(op)).collect()),
            ),
        ])
    }

    /// The workload identity.
    pub fn workload_id(&self) -> [u8; 32] {
        digest_canonical_value_bytes_v1(WORKLOAD_DOMAIN, &self.canonical())
            .expect("workload specification should encode")
    }
}

/// Declared outcomes and goldens. Never part of the workload identity.
#[derive(Clone, Debug, Default)]
pub struct Expectation {
    /// Named assertions, as canonical text.
    pub assertions: Vec<(&'static str, String)>,
    /// Versions of output formats the assertions depend on.
    pub output_formats: Vec<(&'static str, u32)>,
}

impl Expectation {
    /// The expectation identity for `workload_id`.
    pub fn expectation_id(&self, workload_id: &[u8; 32]) -> [u8; 32] {
        let text = |s: &str| Value::Text(s.to_owned());
        let value = Value::Map(vec![
            (text("workload_id"), Value::Bytes(workload_id.to_vec())),
            (
                text("assertions"),
                Value::Array(
                    self.assertions
                        .iter()
                        .map(|(k, v)| Value::Array(vec![text(k), text(v)]))
                        .collect(),
                ),
            ),
            (
                text("output_formats"),
                Value::Array(
                    self.output_formats
                        .iter()
                        .map(|(k, v)| Value::Array(vec![text(k), Value::Integer(i128::from(*v))]))
                        .collect(),
                ),
            ),
        ]);
        digest_canonical_value_bytes_v1(EXPECTATION_DOMAIN, &value)
            .expect("expectation should encode")
    }

    /// The value of a named assertion, if declared.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.assertions
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Lowercase hex of a digest.
pub fn hex32(bytes: &[u8; 32]) -> String {
    hex::encode(bytes)
}
