// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The closed, per-intrinsic translation from catalog failure names to the
//! runtime tags Echo records.
//!
//! Edict's ABI names an intrinsic's failures with identifiers. Echo's runtime
//! records versioned tags such as `echo.atom-missing/v1`. Each runtime
//! intrinsic pins its own pairs here: the key is the intrinsic and the catalog
//! name together, never a name alone, and there is no fallback that forwards an
//! unfamiliar string. Application aliases (an adapter's own failure names) map
//! onto catalog names in the application's adapter and never appear here.
use std::collections::BTreeMap;

use crate::edict_pure::EvaluationError as Error;

/// One failure of one runtime intrinsic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FailureName {
    /// Identifier the pinned catalog declares.
    pub(super) catalog: &'static str,
    /// Tag Echo's runtime records for it.
    pub(super) runtime: &'static str,
    /// Authority class the catalog must give it.
    pub(super) authority: &'static str,
}

const NODE_ATOM_READ_V1: [FailureName; 4] = [
    FailureName {
        catalog: "atomMissing",
        runtime: "echo.atom-missing/v1",
        authority: "resourceFault",
    },
    FailureName {
        catalog: "atomRequired",
        runtime: "echo.atom-required/v1",
        authority: "integrityFault",
    },
    FailureName {
        catalog: "atomTypeMismatch",
        runtime: "echo.atom-type-mismatch/v1",
        authority: "integrityFault",
    },
    FailureName {
        catalog: "atomByteBoundExceeded",
        runtime: "echo.atom-byte-bound-exceeded/v1",
        authority: "integrityFault",
    },
];

/// The complete failure set of an exact runtime intrinsic identity, or `None`
/// for an identity that is not a runtime intrinsic. Mutations declare none.
pub(super) fn failures(intrinsic: &str) -> Option<&'static [FailureName]> {
    match intrinsic {
        "echo.dpo@1.node-atom-read" => Some(&NODE_ATOM_READ_V1),
        "echo.dpo@1.ensure-node"
        | "echo.dpo@1.create-node"
        | "echo.dpo@1.replace-atom"
        | "echo.dpo@1.ensure-edge"
        | "echo.dpo@1.replace-edge" => Some(&[]),
        _ => None,
    }
}

/// Translates a lowered step's obstruction arms, keyed by catalog name, into
/// arms keyed by runtime tag. Refuses an unknown, missing or repeated name.
pub(super) fn runtime_arms(
    intrinsic: &str,
    arms: BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, Error> {
    let declared = failures(intrinsic).ok_or(Error::InvalidArtifact)?;
    if arms.len() != declared.len() {
        return Err(Error::InvalidArtifact);
    }
    let mut runtime = BTreeMap::new();
    for (name, callee) in arms {
        let failure = declared
            .iter()
            .find(|failure| failure.catalog == name)
            .ok_or(Error::InvalidArtifact)?;
        if runtime.insert(failure.runtime.to_owned(), callee).is_some() {
            return Err(Error::InvalidArtifact);
        }
    }
    Ok(runtime)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    const READ: &str = "echo.dpo@1.node-atom-read";

    fn arms(names: &[&str]) -> BTreeMap<String, String> {
        names
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    format!("jedit.text@1.{name}Obstruction"),
                )
            })
            .collect()
    }

    #[test]
    fn every_read_failure_has_exactly_one_runtime_meaning() {
        let read = failures(READ).expect("read");
        let mut catalog = read.iter().map(|f| f.catalog).collect::<Vec<_>>();
        let mut runtime = read.iter().map(|f| f.runtime).collect::<Vec<_>>();
        catalog.sort_unstable();
        catalog.dedup();
        runtime.sort_unstable();
        runtime.dedup();
        assert_eq!(catalog.len(), 4);
        assert_eq!(runtime.len(), 4);
        // The runtime's four read obstructions are exactly these tags.
        assert_eq!(
            runtime,
            [
                "echo.atom-byte-bound-exceeded/v1",
                "echo.atom-missing/v1",
                "echo.atom-required/v1",
                "echo.atom-type-mismatch/v1",
            ]
        );
    }

    #[test]
    fn the_table_is_scoped_by_intrinsic() {
        // The read's names mean nothing on a mutation or an unknown identity.
        assert_eq!(failures("echo.dpo@1.ensure-node"), Some(&[][..]));
        assert_eq!(failures("echo.dpo@1.delete-node"), None);
        assert!(runtime_arms("echo.dpo@1.ensure-node", arms(&["atomMissing"])).is_err());
        assert!(runtime_arms("echo.dpo@1.delete-node", arms(&[])).is_err());
    }

    #[test]
    fn unknown_missing_and_tag_spelled_arms_are_refused() {
        let all = [
            "atomMissing",
            "atomRequired",
            "atomTypeMismatch",
            "atomByteBoundExceeded",
        ];
        assert!(runtime_arms(READ, arms(&all)).is_ok());
        assert!(runtime_arms(READ, arms(&all[..3])).is_err());
        assert!(runtime_arms(
            READ,
            arms(&[
                "atomMissing",
                "atomRequired",
                "atomTypeMismatch",
                "atomAbsent"
            ])
        )
        .is_err());
        assert!(runtime_arms(
            READ,
            arms(&[
                "echo.atom-missing/v1",
                "atomRequired",
                "atomTypeMismatch",
                "atomByteBoundExceeded"
            ])
        )
        .is_err());
    }

    /// Jim's own name, the provider's catalog name and the recorded runtime
    /// tag are three distinct layers; only the last two meet in Echo's table.
    #[test]
    fn jims_missing_alias_traces_to_the_recorded_runtime_tag() {
        // Jim's adapter (the authored route's spelling): `missing` → `atomMissing`.
        let jim_local = "missing";
        let catalog_name = "atomMissing";
        // The Target IR arm Edict emits is keyed by the adapter's target.
        let lowered = [
            ("atomMissing", "jedit.text@1.MissingFact"),
            ("atomRequired", "jedit.text@1.NotAtom"),
            ("atomTypeMismatch", "jedit.text@1.TypeMismatch"),
            ("atomByteBoundExceeded", "jedit.text@1.TooLarge"),
        ]
        .map(|(name, callee)| (name.to_owned(), callee.to_owned()))
        .into_iter()
        .collect();
        let runtime = runtime_arms(READ, lowered).expect("closed translation");
        assert_eq!(
            runtime.get("echo.atom-missing/v1").map(String::as_str),
            Some("jedit.text@1.MissingFact"),
            "{jim_local} → {catalog_name} → echo.atom-missing/v1"
        );
        // Jim's alias is not an Echo name.
        assert!(failures(READ)
            .expect("read")
            .iter()
            .all(|failure| failure.catalog != jim_local));
    }
}
