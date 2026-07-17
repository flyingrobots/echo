// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(
    dead_code,
    reason = "shared support is compiled into separate integration-test crates"
)]
//! Executable-law closure applied after owning-CDDL corpus admission.
//!
//! This parser does not replace schema admission. It binds each already
//! admitted declarative case to one reviewed tuple and executor owner without
//! treating the declaration itself as execution evidence.

use std::collections::BTreeSet;

use edict_syntax::{decode_canonical_cbor, CanonicalValue};

pub const CONFORMANCE_CORPUS_BYTES: &[u8] = include_bytes!(
    "../../../../schemas/edict-provider/generated/v1/resources/resource.conformance-corpus.cbor"
);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExecutableContract {
    CompletedPackageParity,
    AmbientCapabilityPreflightDenied,
    NoncanonicalTargetIrOutputDenied,
}

impl ExecutableContract {
    fn parse(value: &str) -> Result<Self, CorpusContractError> {
        match value {
            "completed-package-parity" => Ok(Self::CompletedPackageParity),
            "ambient-capability-preflight-denied" => Ok(Self::AmbientCapabilityPreflightDenied),
            "noncanonical-target-ir-output-denied" => Ok(Self::NoncanonicalTargetIrOutputDenied),
            _ => Err(CorpusContractError::new(
                CorpusContractErrorKind::UnknownContract,
            )),
        }
    }

    const fn expected(self) -> ExpectedDeclaration {
        match self {
            Self::CompletedPackageParity => ExpectedDeclaration {
                id: "package-parity",
                crossing: "pipeline",
                stimulus: "baseline",
                required_disposition: "accepted",
                owner: ExecutorOwner::Package,
            },
            Self::AmbientCapabilityPreflightDenied => ExpectedDeclaration {
                id: "ambient-capability-denial",
                crossing: "component-preflight",
                stimulus: "ambient-capabilities-denied",
                required_disposition: "rejected",
                owner: ExecutorOwner::Host,
            },
            Self::NoncanonicalTargetIrOutputDenied => ExpectedDeclaration {
                id: "noncanonical-output",
                crossing: "host-output-admission",
                stimulus: "noncanonical-cbor-output",
                required_disposition: "rejected",
                owner: ExecutorOwner::Host,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutorOwner {
    Package,
    Host,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredCase {
    id: String,
    crossing: String,
    stimulus: String,
    required_disposition: String,
    contract: ExecutableContract,
}

impl DeclaredCase {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn crossing(&self) -> &str {
        &self.crossing
    }

    pub fn stimulus(&self) -> &str {
        &self.stimulus
    }

    pub fn required_disposition(&self) -> &str {
        &self.required_disposition
    }

    pub const fn contract(&self) -> ExecutableContract {
        self.contract
    }

    pub const fn owner(&self) -> ExecutorOwner {
        self.contract.expected().owner
    }

    fn matches_expected(&self) -> bool {
        let expected = self.contract.expected();
        self.id == expected.id
            && self.crossing == expected.crossing
            && self.stimulus == expected.stimulus
            && self.required_disposition == expected.required_disposition
    }
}

#[derive(Clone, Copy)]
struct ExpectedDeclaration {
    id: &'static str,
    crossing: &'static str,
    stimulus: &'static str,
    required_disposition: &'static str,
    owner: ExecutorOwner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorpusContractErrorKind {
    CanonicalCborInvalid,
    RootClosureInvalid,
    EmptyCases,
    CaseClosureInvalid,
    OutcomeClosureInvalid,
    FieldInvalid,
    UnknownContract,
    DeclarationMismatch,
    DuplicateCaseId,
    DuplicateContract,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorpusContractError {
    kind: CorpusContractErrorKind,
}

impl CorpusContractError {
    const fn new(kind: CorpusContractErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> CorpusContractErrorKind {
        self.kind
    }
}

pub fn decode_declared_cases(bytes: &[u8]) -> Result<Vec<DeclaredCase>, CorpusContractError> {
    let root = decode_canonical_cbor(bytes)
        .map_err(|_| CorpusContractError::new(CorpusContractErrorKind::CanonicalCborInvalid))?;
    let root = map_entries(&root, CorpusContractErrorKind::RootClosureInvalid)?;
    if root.len() != 6
        || required_text(root, "apiVersion")? != "echo.edict-provider.conformance-corpus/v1"
        || required_text(root, "class")? != "declarative"
    {
        return Err(CorpusContractError::new(
            CorpusContractErrorKind::RootClosureInvalid,
        ));
    }
    for field in ["operations", "capabilities", "semanticEffects"] {
        map_entries(
            required_field(root, field)?,
            CorpusContractErrorKind::RootClosureInvalid,
        )?;
    }

    let cases = map_entries(
        required_field(root, "cases")?,
        CorpusContractErrorKind::RootClosureInvalid,
    )?;
    if cases.is_empty() {
        return Err(CorpusContractError::new(
            CorpusContractErrorKind::EmptyCases,
        ));
    }

    let mut case_ids = BTreeSet::new();
    let mut contracts = BTreeSet::new();
    let mut declared = Vec::with_capacity(cases.len());
    for (case_id, value) in cases {
        let CanonicalValue::Text(case_id) = case_id else {
            return Err(CorpusContractError::new(
                CorpusContractErrorKind::FieldInvalid,
            ));
        };
        if !case_ids.insert(case_id.clone()) {
            return Err(CorpusContractError::new(
                CorpusContractErrorKind::DuplicateCaseId,
            ));
        }

        let fields = map_entries(value, CorpusContractErrorKind::CaseClosureInvalid)?;
        if fields.len() != 3 {
            return Err(CorpusContractError::new(
                CorpusContractErrorKind::CaseClosureInvalid,
            ));
        }
        let outcome = map_entries(
            required_field(fields, "requiredOutcome")?,
            CorpusContractErrorKind::OutcomeClosureInvalid,
        )?;
        if outcome.len() != 2 {
            return Err(CorpusContractError::new(
                CorpusContractErrorKind::OutcomeClosureInvalid,
            ));
        }

        let contract = ExecutableContract::parse(required_text(outcome, "contract")?)?;
        if !contracts.insert(contract) {
            return Err(CorpusContractError::new(
                CorpusContractErrorKind::DuplicateContract,
            ));
        }
        let case = DeclaredCase {
            id: case_id.clone(),
            crossing: required_text(fields, "crossing")?.to_owned(),
            stimulus: required_text(fields, "stimulus")?.to_owned(),
            required_disposition: required_text(outcome, "disposition")?.to_owned(),
            contract,
        };
        if !case.matches_expected() {
            return Err(CorpusContractError::new(
                CorpusContractErrorKind::DeclarationMismatch,
            ));
        }
        declared.push(case);
    }
    declared.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    Ok(declared)
}

fn map_entries(
    value: &CanonicalValue,
    kind: CorpusContractErrorKind,
) -> Result<&[(CanonicalValue, CanonicalValue)], CorpusContractError> {
    let CanonicalValue::Map(entries) = value else {
        return Err(CorpusContractError::new(kind));
    };
    Ok(entries)
}

fn required_field<'a>(
    entries: &'a [(CanonicalValue, CanonicalValue)],
    field: &str,
) -> Result<&'a CanonicalValue, CorpusContractError> {
    let mut matches = entries.iter().filter_map(|(key, value)| {
        (key == &CanonicalValue::Text(field.to_owned())).then_some(value)
    });
    let Some(value) = matches.next() else {
        return Err(CorpusContractError::new(
            CorpusContractErrorKind::FieldInvalid,
        ));
    };
    if matches.next().is_some() {
        return Err(CorpusContractError::new(
            CorpusContractErrorKind::FieldInvalid,
        ));
    }
    Ok(value)
}

fn required_text<'a>(
    entries: &'a [(CanonicalValue, CanonicalValue)],
    field: &str,
) -> Result<&'a str, CorpusContractError> {
    let CanonicalValue::Text(value) = required_field(entries, field)? else {
        return Err(CorpusContractError::new(
            CorpusContractErrorKind::FieldInvalid,
        ));
    };
    Ok(value)
}
