// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The two target-profile namespaces that name Echo's effect intrinsics.
//!
//! An intrinsic name is `<namespace>.<operation>`. `echo.dpo@1` and
//! `echo.dpo@2` name the same six runtime intrinsics: every table below keys
//! on the operation alone, so one implementation serves both names. The
//! second coordinate is an owner-approved, bounded exception to the "One
//! Current Version" rule (flyingrobots/echo#985): it exists so an effect
//! program can select a published profile whose catalog declares these
//! intrinsics, while `echo.dpo@1` and everything pinned to it stays as it was.

/// One target-profile namespace for Echo's effect intrinsics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Namespace {
    /// The profile coordinate, which is also the intrinsic-name prefix.
    pub(super) coordinate: &'static str,
    /// The coordinate, and digest domain, of the intrinsic catalog the
    /// profile pins.
    pub(super) catalog: &'static str,
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
pub(super) fn namespace(profile_coordinate: &str) -> Option<Namespace> {
    NAMESPACES
        .into_iter()
        .find(|namespace| namespace.coordinate == profile_coordinate)
}

/// The operation an exact intrinsic name selects in either namespace, such as
/// `ensure-node` for `echo.dpo@2.ensure-node`. Names outside both namespaces,
/// and empty operations, have none.
pub(super) fn operation(intrinsic: &str) -> Option<&str> {
    NAMESPACES.iter().find_map(|namespace| {
        intrinsic
            .strip_prefix(namespace.coordinate)?
            .strip_prefix('.')
            .filter(|operation| !operation.is_empty())
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn each_namespace_selects_its_own_catalog() {
        assert_eq!(
            namespace("echo.dpo@1").map(|n| n.catalog),
            Some("echo.dpo.intrinsics/v1")
        );
        assert_eq!(
            namespace("echo.dpo@2").map(|n| n.catalog),
            Some("echo.dpo.intrinsics/v2")
        );
        assert_eq!(namespace("echo.dpo@3"), None);
        assert_eq!(namespace("echo.dpo"), None);
    }

    #[test]
    fn operations_are_read_only_below_a_known_namespace() {
        assert_eq!(operation("echo.dpo@1.ensure-node"), Some("ensure-node"));
        assert_eq!(operation("echo.dpo@2.ensure-node"), Some("ensure-node"));
        for foreign in [
            "echo.dpo@3.ensure-node",
            "echo.dpo@2",
            "echo.dpo@2.",
            "echo.dpo@2ensure-node",
            "other.echo.dpo@2.ensure-node",
        ] {
            assert_eq!(operation(foreign), None, "{foreign}");
        }
    }
}
