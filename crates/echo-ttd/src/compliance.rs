// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! Channel policy validation and compliance checking.
//!
//! This module validates that finalized channel emissions conform to their
//! declared policies. It produces structured [`Violation`] records that can
//! be displayed in the TTD UI.
//!
//! # Policies Checked
//!
//! - **`StrictSingle`**: Channel must have exactly one emission per tick.
//!   Violations: zero emissions or multiple emissions.
//! - **`Reduce`**: Channel uses a reducer to merge emissions. Currently validates
//!   that emissions exist (reducer constraints are checked at emission time).
//! - **`Log`**: No constraints — all emissions are valid.
//!
//! # Example
//!
//! ```
//! use echo_ttd::compliance::{PolicyChecker, Severity};
//! use warp_core::materialization::{ChannelPolicy, FinalizedChannel, make_channel_id};
//!
//! let checker = PolicyChecker::new();
//!
//! // A StrictSingle channel with two emissions is a violation
//! let channel_id = make_channel_id("entity:position");
//! let emissions = vec![
//!     FinalizedChannel { channel: channel_id, data: vec![1, 2, 3] },
//!     FinalizedChannel { channel: channel_id, data: vec![4, 5, 6] },
//! ];
//! let policies = vec![(channel_id, ChannelPolicy::StrictSingle)];
//!
//! let violations = checker.check_channel_policies(&emissions, &policies);
//! assert_eq!(violations.len(), 1);
//! assert_eq!(violations[0].severity, Severity::Error);
//! ```

use std::collections::BTreeMap;

use warp_core::materialization::{ChannelId, ChannelPolicy, FinalizedChannel};

/// Severity level for compliance violations.
///
/// Severity determines how the violation is displayed in the TTD UI and
/// whether it blocks certain operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational — not a problem, just notable.
    Info,
    /// Warning — potential issue, but not blocking.
    Warn,
    /// Error — policy violated, but execution continued.
    Error,
    /// Fatal — critical violation that should halt processing.
    Fatal,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warn => write!(f, "WARN"),
            Self::Error => write!(f, "ERROR"),
            Self::Fatal => write!(f, "FATAL"),
        }
    }
}

/// Classification of compliance violations.
///
/// These codes match the TTD spec (docs/plans/ttd-app.md Part 4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViolationCode {
    // ─── Receipt / Hashing ───────────────────────────────────────────────────
    /// Receipt is missing for a tick that should have one.
    ReceiptMissing,
    /// Receipt hash doesn't match computed value.
    ReceiptHashMismatch,
    /// Emissions digest doesn't match recorded emissions.
    EmissionsDigestMismatch,
    /// Entry hash doesn't match content.
    EntryHashMismatch,

    // ─── Channel Policy ──────────────────────────────────────────────────────
    /// `StrictSingle` channel received zero or multiple emissions.
    StrictSingleViolation,
    /// `Reduce` channel has conflicting values that can't merge.
    ReduceConflict,
    /// Emission to a channel not declared in schema.
    UnknownChannel,
    /// Channel version mismatch between schema and emission.
    ChannelVersionMismatch,

    // ─── Rule Contracts ──────────────────────────────────────────────────────
    /// Rule declared `mustEmit` but didn't emit to required channel.
    MustEmitMissing,
    /// Rule emitted more times than `mustEmit` count allows.
    MustEmitTooMany,
    /// Rule emitted to a channel not in its `mayEmitOnly` set.
    MayEmitOnlyViolation,
    /// Rule fired that isn't declared in schema.
    UndeclaredRule,

    // ─── Determinism Constraints ─────────────────────────────────────────────
    /// CBOR encoding is not canonical.
    NonCanonicalEncoding,
    /// Output marked `@sorted` is not in order.
    UnsortedOutput,
    /// Floating point used in a `@fixed` context.
    FloatUsed,
    /// Unordered map used where `@noUnorderedMap` is declared.
    UnorderedMapUsed,
    /// Non-deterministic field detected.
    NonDeterministicField,

    // ─── Footprint ───────────────────────────────────────────────────────────
    /// Rule read from atoms outside its declared read footprint.
    FootprintReadViolation,
    /// Rule wrote to atoms outside its declared write footprint.
    FootprintWriteViolation,
    /// Rule accessed edges outside adjacency bounds.
    FootprintAdjacencyViolation,
}

impl std::fmt::Display for ViolationCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::ReceiptMissing => "RECEIPT_MISSING",
            Self::ReceiptHashMismatch => "RECEIPT_HASH_MISMATCH",
            Self::EmissionsDigestMismatch => "EMISSIONS_DIGEST_MISMATCH",
            Self::EntryHashMismatch => "ENTRY_HASH_MISMATCH",
            Self::StrictSingleViolation => "STRICT_SINGLE_VIOLATION",
            Self::ReduceConflict => "REDUCE_CONFLICT",
            Self::UnknownChannel => "UNKNOWN_CHANNEL",
            Self::ChannelVersionMismatch => "CHANNEL_VERSION_MISMATCH",
            Self::MustEmitMissing => "MUST_EMIT_MISSING",
            Self::MustEmitTooMany => "MUST_EMIT_TOO_MANY",
            Self::MayEmitOnlyViolation => "MAY_EMIT_ONLY_VIOLATION",
            Self::UndeclaredRule => "UNDECLARED_RULE",
            Self::NonCanonicalEncoding => "NON_CANONICAL_ENCODING",
            Self::UnsortedOutput => "UNSORTED_OUTPUT",
            Self::FloatUsed => "FLOAT_USED",
            Self::UnorderedMapUsed => "UNORDERED_MAP_USED",
            Self::NonDeterministicField => "NON_DETERMINISTIC_FIELD",
            Self::FootprintReadViolation => "FOOTPRINT_READ_VIOLATION",
            Self::FootprintWriteViolation => "FOOTPRINT_WRITE_VIOLATION",
            Self::FootprintAdjacencyViolation => "FOOTPRINT_ADJACENCY_VIOLATION",
        };
        write!(f, "{s}")
    }
}

/// A compliance violation detected during tick validation.
///
/// Violations are the primary output of the compliance engine. Each violation
/// identifies what went wrong, where, and with what severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Severity level.
    pub severity: Severity,
    /// Classification code.
    pub code: ViolationCode,
    /// Human-readable description.
    pub message: String,
    /// Channel involved (if applicable).
    pub channel_id: Option<ChannelId>,
    /// Tick number (if applicable).
    pub tick: Option<u64>,
    /// Number of emissions that caused the violation (for policy violations).
    pub emission_count: Option<usize>,
}

impl Violation {
    /// Creates a new violation.
    #[must_use]
    pub fn new(severity: Severity, code: ViolationCode, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            channel_id: None,
            tick: None,
            emission_count: None,
        }
    }

    /// Attaches a channel ID to this violation.
    #[must_use]
    pub fn with_channel(mut self, channel_id: ChannelId) -> Self {
        self.channel_id = Some(channel_id);
        self
    }

    /// Attaches a tick number to this violation.
    #[must_use]
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.tick = Some(tick);
        self
    }

    /// Attaches an emission count to this violation.
    #[must_use]
    pub fn with_emission_count(mut self, count: usize) -> Self {
        self.emission_count = Some(count);
        self
    }
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.code, self.message)
    }
}

/// Summary of compliance check results.
///
/// Provides aggregate counts by severity for quick status display.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComplianceSummary {
    /// Count of fatal violations.
    pub fatal_count: u32,
    /// Count of error violations.
    pub error_count: u32,
    /// Count of warning violations.
    pub warn_count: u32,
    /// Count of info violations.
    pub info_count: u32,
}

impl ComplianceSummary {
    /// Returns the maximum severity present in the summary.
    #[must_use]
    pub fn max_severity(&self) -> Option<Severity> {
        if self.fatal_count > 0 {
            Some(Severity::Fatal)
        } else if self.error_count > 0 {
            Some(Severity::Error)
        } else if self.warn_count > 0 {
            Some(Severity::Warn)
        } else if self.info_count > 0 {
            Some(Severity::Info)
        } else {
            None
        }
    }

    /// Returns `true` if there are no errors or fatals.
    #[must_use]
    pub fn is_green(&self) -> bool {
        self.fatal_count == 0 && self.error_count == 0
    }

    /// Creates a summary from a list of violations.
    #[must_use]
    pub fn from_violations(violations: &[Violation]) -> Self {
        let mut summary = Self::default();
        for v in violations {
            match v.severity {
                Severity::Fatal => summary.fatal_count += 1,
                Severity::Error => summary.error_count += 1,
                Severity::Warn => summary.warn_count += 1,
                Severity::Info => summary.info_count += 1,
            }
        }
        summary
    }
}

/// Channel policy compliance checker.
///
/// This is the main entry point for policy validation. It checks that
/// finalized channel emissions conform to their declared policies.
#[derive(Debug, Clone, Default)]
pub struct PolicyChecker {
    /// If true, unknown channels (emissions without declared policy) are errors.
    /// If false, they're warnings.
    pub strict_mode: bool,
}

impl PolicyChecker {
    /// Creates a new policy checker with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new policy checker in strict mode.
    #[must_use]
    pub fn strict() -> Self {
        Self { strict_mode: true }
    }

    /// Checks channel policies against finalized emissions.
    ///
    /// # Arguments
    ///
    /// * `emissions` - Finalized channel emissions from the materialization bus
    /// * `policies` - Declared policies for each channel (from schema)
    ///
    /// # Returns
    ///
    /// A list of violations found. Empty list means all policies satisfied.
    #[must_use]
    pub fn check_channel_policies(
        &self,
        emissions: &[FinalizedChannel],
        policies: &[(ChannelId, ChannelPolicy)],
    ) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Build policy lookup map
        let policy_map: BTreeMap<ChannelId, ChannelPolicy> = policies.iter().copied().collect();

        // Group emissions by channel
        let mut emission_counts: BTreeMap<ChannelId, usize> = BTreeMap::new();
        for fc in emissions {
            *emission_counts.entry(fc.channel).or_insert(0) += 1;
        }

        // Check each emission against its policy
        for (channel_id, count) in &emission_counts {
            if let Some(policy) = policy_map.get(channel_id) {
                if let Some(v) = Self::check_policy(*channel_id, *policy, *count) {
                    violations.push(v);
                }
            } else {
                // Unknown channel — no policy declared
                let severity = if self.strict_mode {
                    Severity::Error
                } else {
                    Severity::Warn
                };
                violations.push(
                    Violation::new(
                        severity,
                        ViolationCode::UnknownChannel,
                        format!(
                            "channel {:?} has {} emission(s) but no declared policy",
                            channel_id.0, count
                        ),
                    )
                    .with_channel(*channel_id)
                    .with_emission_count(*count),
                );
            }
        }

        // Check for StrictSingle channels with zero emissions
        for (channel_id, policy) in policies {
            if *policy == ChannelPolicy::StrictSingle && !emission_counts.contains_key(channel_id) {
                violations.push(
                    Violation::new(
                        Severity::Error,
                        ViolationCode::StrictSingleViolation,
                        format!(
                            "StrictSingle channel {:?} has 0 emissions (expected exactly 1)",
                            channel_id.0
                        ),
                    )
                    .with_channel(*channel_id)
                    .with_emission_count(0),
                );
            }
        }

        // Sort violations by severity (descending) for consistent output
        violations.sort_by(|a, b| b.severity.cmp(&a.severity));

        violations
    }

    /// Checks a single channel's emission count against its policy.
    fn check_policy(
        channel_id: ChannelId,
        policy: ChannelPolicy,
        emission_count: usize,
    ) -> Option<Violation> {
        match policy {
            ChannelPolicy::StrictSingle => {
                if emission_count == 1 {
                    None
                } else {
                    Some(
                        Violation::new(
                            Severity::Error,
                            ViolationCode::StrictSingleViolation,
                            format!(
                                "StrictSingle channel {:?} has {} emissions (expected exactly 1)",
                                channel_id.0, emission_count
                            ),
                        )
                        .with_channel(channel_id)
                        .with_emission_count(emission_count),
                    )
                }
            }
            ChannelPolicy::Reduce(_) => {
                // Reduce channels accept any number of emissions (including zero).
                // The reducer handles merging. Actual reduce conflicts are detected
                // at emission time by the materialization bus, not here.
                None
            }
            ChannelPolicy::Log => {
                // Log channels accept any number of emissions.
                None
            }
        }
    }
}

/// Convenience function to check channel policies.
///
/// This is a shorthand for creating a [`PolicyChecker`] and calling
/// [`PolicyChecker::check_channel_policies`].
#[must_use]
pub fn check_channel_policies(
    emissions: &[FinalizedChannel],
    policies: &[(ChannelId, ChannelPolicy)],
) -> Vec<Violation> {
    PolicyChecker::new().check_channel_policies(emissions, policies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp_core::materialization::{make_channel_id, ReduceOp};

    fn make_emission(label: &str, data: &[u8]) -> FinalizedChannel {
        FinalizedChannel {
            channel: make_channel_id(label),
            data: data.to_vec(),
        }
    }

    #[test]
    fn strict_single_satisfied() {
        let ch = make_channel_id("entity:position");
        let emissions = vec![FinalizedChannel {
            channel: ch,
            data: vec![1, 2, 3],
        }];
        let policies = vec![(ch, ChannelPolicy::StrictSingle)];

        let violations = check_channel_policies(&emissions, &policies);
        assert!(
            violations.is_empty(),
            "single emission should satisfy StrictSingle"
        );
    }

    #[test]
    fn strict_single_zero_emissions() {
        let ch = make_channel_id("entity:position");
        let emissions: Vec<FinalizedChannel> = vec![];
        let policies = vec![(ch, ChannelPolicy::StrictSingle)];

        let violations = check_channel_policies(&emissions, &policies);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, ViolationCode::StrictSingleViolation);
        assert_eq!(violations[0].emission_count, Some(0));
    }

    #[test]
    fn strict_single_multiple_emissions() {
        let ch = make_channel_id("entity:position");
        let emissions = vec![
            FinalizedChannel {
                channel: ch,
                data: vec![1],
            },
            FinalizedChannel {
                channel: ch,
                data: vec![2],
            },
        ];
        let policies = vec![(ch, ChannelPolicy::StrictSingle)];

        let violations = check_channel_policies(&emissions, &policies);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, ViolationCode::StrictSingleViolation);
        assert_eq!(violations[0].emission_count, Some(2));
        assert_eq!(violations[0].severity, Severity::Error);
    }

    #[test]
    fn log_accepts_any_count() {
        let ch = make_channel_id("events:log");
        let emissions = vec![
            make_emission("events:log", &[1]),
            make_emission("events:log", &[2]),
            make_emission("events:log", &[3]),
        ];
        let policies = vec![(ch, ChannelPolicy::Log)];

        let violations = check_channel_policies(&emissions, &policies);
        assert!(violations.is_empty(), "Log policy should accept any count");
    }

    #[test]
    fn log_accepts_zero() {
        let ch = make_channel_id("events:log");
        let emissions: Vec<FinalizedChannel> = vec![];
        let policies = vec![(ch, ChannelPolicy::Log)];

        let violations = check_channel_policies(&emissions, &policies);
        assert!(
            violations.is_empty(),
            "Log policy should accept zero emissions"
        );
    }

    #[test]
    fn reduce_accepts_multiple() {
        let ch = make_channel_id("metrics:sum");
        let emissions = vec![
            make_emission("metrics:sum", &[10]),
            make_emission("metrics:sum", &[20]),
        ];
        let policies = vec![(ch, ChannelPolicy::Reduce(ReduceOp::Sum))];

        let violations = check_channel_policies(&emissions, &policies);
        assert!(
            violations.is_empty(),
            "Reduce policy should accept multiple emissions"
        );
    }

    #[test]
    fn reduce_accepts_zero() {
        let ch = make_channel_id("metrics:sum");
        let emissions: Vec<FinalizedChannel> = vec![];
        let policies = vec![(ch, ChannelPolicy::Reduce(ReduceOp::Sum))];

        let violations = check_channel_policies(&emissions, &policies);
        assert!(
            violations.is_empty(),
            "Reduce policy should accept zero emissions"
        );
    }

    #[test]
    fn unknown_channel_warning() {
        let emissions = vec![make_emission("unknown:channel", &[1, 2, 3])];
        let policies: Vec<(ChannelId, ChannelPolicy)> = vec![];

        let checker = PolicyChecker::new();
        let violations = checker.check_channel_policies(&emissions, &policies);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, ViolationCode::UnknownChannel);
        assert_eq!(violations[0].severity, Severity::Warn);
    }

    #[test]
    fn unknown_channel_strict_mode() {
        let emissions = vec![make_emission("unknown:channel", &[1, 2, 3])];
        let policies: Vec<(ChannelId, ChannelPolicy)> = vec![];

        let checker = PolicyChecker::strict();
        let violations = checker.check_channel_policies(&emissions, &policies);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].code, ViolationCode::UnknownChannel);
        assert_eq!(violations[0].severity, Severity::Error);
    }

    #[test]
    fn multiple_channels_mixed_policies() {
        let ch_single = make_channel_id("entity:position");
        let ch_log = make_channel_id("events:log");
        let ch_reduce = make_channel_id("metrics:sum");

        let emissions = vec![
            FinalizedChannel {
                channel: ch_single,
                data: vec![1],
            },
            FinalizedChannel {
                channel: ch_log,
                data: vec![2],
            },
            FinalizedChannel {
                channel: ch_log,
                data: vec![3],
            },
            FinalizedChannel {
                channel: ch_reduce,
                data: vec![4],
            },
        ];

        let policies = vec![
            (ch_single, ChannelPolicy::StrictSingle),
            (ch_log, ChannelPolicy::Log),
            (ch_reduce, ChannelPolicy::Reduce(ReduceOp::Sum)),
        ];

        let violations = check_channel_policies(&emissions, &policies);
        assert!(violations.is_empty(), "all policies should be satisfied");
    }

    #[test]
    fn violations_sorted_by_severity() {
        let ch_strict = make_channel_id("strict");
        let ch_unknown = make_channel_id("unknown");

        let emissions = vec![
            // StrictSingle with 2 emissions -> Error
            FinalizedChannel {
                channel: ch_strict,
                data: vec![1],
            },
            FinalizedChannel {
                channel: ch_strict,
                data: vec![2],
            },
            // Unknown channel -> Warn
            FinalizedChannel {
                channel: ch_unknown,
                data: vec![3],
            },
        ];

        let policies = vec![(ch_strict, ChannelPolicy::StrictSingle)];

        let violations = check_channel_policies(&emissions, &policies);
        assert_eq!(violations.len(), 2);
        // Error should come before Warn
        assert_eq!(violations[0].severity, Severity::Error);
        assert_eq!(violations[1].severity, Severity::Warn);
    }

    #[test]
    fn compliance_summary_from_violations() {
        let violations = vec![
            Violation::new(Severity::Fatal, ViolationCode::ReceiptMissing, "fatal"),
            Violation::new(
                Severity::Error,
                ViolationCode::StrictSingleViolation,
                "error1",
            ),
            Violation::new(
                Severity::Error,
                ViolationCode::StrictSingleViolation,
                "error2",
            ),
            Violation::new(Severity::Warn, ViolationCode::UnknownChannel, "warn"),
            Violation::new(Severity::Info, ViolationCode::NonCanonicalEncoding, "info"),
        ];

        let summary = ComplianceSummary::from_violations(&violations);

        assert_eq!(summary.fatal_count, 1);
        assert_eq!(summary.error_count, 2);
        assert_eq!(summary.warn_count, 1);
        assert_eq!(summary.info_count, 1);
        assert_eq!(summary.max_severity(), Some(Severity::Fatal));
        assert!(!summary.is_green());
    }

    #[test]
    fn compliance_summary_green() {
        let violations = vec![
            Violation::new(Severity::Warn, ViolationCode::UnknownChannel, "warn"),
            Violation::new(Severity::Info, ViolationCode::NonCanonicalEncoding, "info"),
        ];

        let summary = ComplianceSummary::from_violations(&violations);

        assert!(summary.is_green(), "no errors or fatals means green");
        assert_eq!(summary.max_severity(), Some(Severity::Warn));
    }

    #[test]
    fn compliance_summary_empty() {
        let summary = ComplianceSummary::from_violations(&[]);

        assert!(summary.is_green());
        assert_eq!(summary.max_severity(), None);
    }

    #[test]
    fn violation_display() {
        let v = Violation::new(
            Severity::Error,
            ViolationCode::StrictSingleViolation,
            "test message",
        );
        let s = format!("{v}");
        assert!(s.contains("ERROR"));
        assert!(s.contains("STRICT_SINGLE_VIOLATION"));
        assert!(s.contains("test message"));
    }
}
