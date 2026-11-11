// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The two target-profile namespaces that name Echo's effect intrinsics, as
//! the lowerer judges them.
//!
//! An intrinsic name is `<namespace>.<operation>`. `echo.dpo@1` and
//! `echo.dpo@2` name the same runtime intrinsics, so every table keys on the
//! operation alone. The second coordinate is an owner-approved, bounded
//! exception to the "One Current Version" rule (flyingrobots/echo#985).

/// One target-profile namespace for Echo's effect intrinsics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Namespace {
    /// The profile coordinate, which is also the intrinsic-name prefix.
    pub(crate) coordinate: &'static str,
    /// The coordinate, and digest domain, of the catalog the profile pins.
    pub(crate) catalog: &'static str,
}

const NAMESPACES: [Namespace; 2] = [
    Namespace {
        coordinate: "echo.dpo@1",
        catalog: "echo.dpo.intrinsics/v1",
    },
    Namespace {
        coordinate: "echo.dpo@2",
        catalog: "echo.dpo.intrinsics/v2",
    },
];

/// The namespace a target profile coordinate selects, if it is one of Echo's.
pub(crate) fn namespace(profile_coordinate: &str) -> Option<Namespace> {
    NAMESPACES
        .into_iter()
        .find(|namespace| namespace.coordinate == profile_coordinate)
}

/// The operation an exact intrinsic name selects in either namespace, such as
/// `ensure-node` for `echo.dpo@2.ensure-node`.
pub(crate) fn operation(intrinsic: &str) -> Option<&str> {
    NAMESPACES.iter().find_map(|namespace| {
        intrinsic
            .strip_prefix(namespace.coordinate)?
            .strip_prefix('.')
            .filter(|operation| !operation.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::{namespace, operation};

    #[test]
    fn only_the_two_namespaces_name_operations() {
        assert_eq!(operation("echo.dpo@1.ensure-node"), Some("ensure-node"));
        assert_eq!(operation("echo.dpo@2.ensure-node"), Some("ensure-node"));
        for foreign in [
            "echo.dpo@3.ensure-node",
            "echo.dpo@2.",
            "echo.dpo@2ensure-node",
        ] {
            assert_eq!(operation(foreign), None, "{foreign}");
        }
        assert_eq!(
            namespace("echo.dpo@2").map(|n| n.catalog),
            Some("echo.dpo.intrinsics/v2")
        );
        assert_eq!(namespace("echo.dpo@3"), None);
    }
}
