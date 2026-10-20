// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Result classes and work accounting.
//!
//! Only a [`ResultClass::Valid`] result may enter a performance comparison.
//! Invalid and incomplete runs keep their timings for diagnosis only, and an
//! unsupported required capability never counts as passing coverage.

/// How an execution is classified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultClass {
    /// Conformance, capability coverage and every declared outcome hold.
    Valid,
    /// At least one declared outcome failed; each entry says which.
    Invalid(Vec<String>),
    /// A required capability is missing.
    Unsupported(Vec<String>),
    /// The run stopped before its declared work finished.
    Incomplete(String),
}

impl ResultClass {
    /// Name used in reports.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Invalid(_) => "invalid",
            Self::Unsupported(_) => "unsupported",
            Self::Incomplete(_) => "incomplete",
        }
    }

    /// Whether this result may enter a performance comparison.
    pub const fn comparable(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

/// Collects failed outcome checks and yields a class.
#[derive(Debug, Default)]
pub struct Checks {
    failures: Vec<String>,
}

impl Checks {
    /// Records `what` as failed unless `ok`.
    pub fn check(&mut self, ok: bool, what: impl Into<String>) {
        if !ok {
            self.failures.push(what.into());
        }
    }

    /// `Valid` when nothing failed, else `Invalid` with every failure.
    pub fn class(self) -> ResultClass {
        if self.failures.is_empty() {
            ResultClass::Valid
        } else {
            ResultClass::Invalid(self.failures)
        }
    }
}

/// Work accounting. The stages are cumulative, not disjoint buckets:
/// committed work was also admitted, and admitted work was also submitted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Accounting {
    /// Distinct operation occurrences in the script.
    pub occurrences: u64,
    /// Delivery attempts made; equals occurrences when nothing is redelivered.
    pub delivery_attempts: u64,
    /// Submissions the app surface accepted.
    pub submitted: u64,
    /// Submissions staged for execution.
    pub admitted: u64,
    /// Occurrences whose outcome is applied and committed.
    pub committed: u64,
    /// Occurrences refused at admission or execution.
    pub refused: u64,
    /// Occurrences not finished when the run ended.
    pub unfinished: u64,
}
