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
    match super::intrinsic_names::operation(intrinsic)? {
        "node-atom-read" => Some(&NODE_ATOM_READ_V1),
        "ensure-node" | "create-node" | "replace-atom" | "ensure-edge" | "replace-edge" => {
            Some(&[])
        }
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
    fn echo_dpo_2_mutations_have_the_same_empty_failure_sets() {
        // amblicit#311: the five mutations are callable without a failure map
        // because their failure sets are empty under either namespace.
        for operation in [
            "ensure-node",
            "create-node",
            "replace-atom",
            "ensure-edge",
            "replace-edge",
        ] {
            for namespace in ["echo.dpo@1", "echo.dpo@2"] {
                let name = format!("{namespace}.{operation}");
                assert_eq!(failures(&name), Some(&[][..]), "{name}");
                assert_eq!(
                    runtime_arms(&name, arms(&[])),
                    Ok(BTreeMap::new()),
                    "{name}"
                );
            }
        }
        assert_eq!(
            failures("echo.dpo@2.node-atom-read").map(<[FailureName]>::len),
            Some(4)
        );
        assert!(runtime_arms("echo.dpo@2.node-atom-read", arms(&[])).is_err());
    }

    /// echo#982: the two graph reads are effect intrinsics under either
    /// namespace, and each catalog name records exactly the tag the bounded
    /// read route already records for the same failure, with the authority
    /// class that route fixed for it (resource limits, an insufficient
    /// selector and malformed storage stay three classes).
    #[test]
    fn graph_read_failures_name_the_bounded_read_tags_and_authority_classes() {
        use crate::edict_read::GraphReadObstruction as O;
        let edge = [
            ("edgeSourceMissing", O::EdgeSourceMissing, "resourceFault"),
            ("edgeBoundExceeded", O::EdgeBoundExceeded, "resourceFault"),
            (
                "edgeScanBoundExceeded",
                O::EdgeScanBoundExceeded,
                "resourceFault",
            ),
        ];
        let closure = [
            ("closureStartMissing", O::ClosureStartMissing, "resourceFault"),
            (
                "closureNodeBoundExceeded",
                O::ClosureNodeBoundExceeded,
                "resourceFault",
            ),
            (
                "closureDepthBoundExceeded",
                O::ClosureDepthBoundExceeded,
                "resourceFault",
            ),
            (
                "closureScanBoundExceeded",
                O::ClosureScanBoundExceeded,
                "resourceFault",
            ),
            (
                "closureWorkBoundExceeded",
                O::ClosureWorkBoundExceeded,
                "resourceFault",
            ),
            (
                "closureSelectorInsufficient",
                O::ClosureSelectorInsufficient,
                "domainMappable",
            ),
            (
                "closureStorageMalformed",
                O::ClosureStorageMalformed,
                "integrityFault",
            ),
        ];
        for namespace in ["echo.dpo@1", "echo.dpo@2"] {
            for (operation, expected) in [
                ("outgoing-edge-read", &edge[..]),
                ("closure-read", &closure[..]),
            ] {
                let name = format!("{namespace}.{operation}");
                let declared = failures(&name).unwrap_or_else(|| panic!("{name}"));
                let actual = declared
                    .iter()
                    .map(|f| (f.catalog, f.runtime, f.authority))
                    .collect::<Vec<_>>();
                let wanted = expected
                    .iter()
                    .map(|(catalog, cause, authority)| (*catalog, cause.runtime_tag(), *authority))
                    .collect::<Vec<_>>();
                assert_eq!(actual, wanted, "{name}");
            }
        }
    }

    #[test]
    fn a_graph_read_step_must_map_every_one_of_its_failures() {
        let edge = "echo.dpo@2.outgoing-edge-read";
        let all = ["edgeSourceMissing", "edgeBoundExceeded", "edgeScanBoundExceeded"];
        let runtime = runtime_arms(edge, arms(&all)).expect("all three mapped");
        assert_eq!(
            runtime.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "echo.edge-bound-exceeded/v1",
                "echo.edge-scan-bound-exceeded/v1",
                "echo.edge-source-missing/v1",
            ]
        );
        assert!(runtime_arms(edge, arms(&all[..2])).is_err());
        // A node read's failure names mean nothing on a graph read.
        assert!(runtime_arms(
            edge,
            arms(&["edgeSourceMissing", "edgeBoundExceeded", "atomMissing"])
        )
        .is_err());
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
