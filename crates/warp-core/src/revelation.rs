// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Revelation posture for retained causal artifacts (Three-Tier Thinking
//! Room, AIΩN Paper VII §6.3; tracked by echo#538).
//!
//! Every retained shell-family artifact carries an explicit revelation
//! posture instead of implicit shared visibility:
//!
//! - [`RevelationPosture::Scratch`] — local, weakly retained, disposable.
//! - [`RevelationPosture::AuthorOnly`] — durable and replayable, sealed to
//!   the creating principal until explicitly promoted.
//! - [`RevelationPosture::Shared`] — collaboratively admitted visibility.
//!
//! Posture is load-bearing, not cosmetic. Two laws are enforced here:
//!
//! 1. **Promotion is explicit and witnessed.** Posture only widens through
//!    [`promote_posture`], which demands a witness digest; silent widening
//!    and any narrowing are obstructions, never no-ops.
//! 2. **Least-revealed-member invariant.** A composite artifact (for
//!    example a braid shell over member strands) cannot reveal more than
//!    its least-revealed member unless a witnessed redaction/promotion
//!    transform exists; [`shell_posture_obstruction`] is the single
//!    admission check for that rule.
//!
//! This module is the E0-lite core required by design packet 0026 before
//! any θ_braid shell lands; the full strand-creation posture system
//! remains echo#538.

use crate::ident::Hash;

/// Revelation tier for one retained causal artifact.
///
/// Ordering is revelation breadth: `Scratch < AuthorOnly < Shared`. The
/// default is [`RevelationPosture::AuthorOnly`] so nothing ships with
/// implicit shared visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum RevelationPosture {
    /// Local, weakly retained, disposable working tier.
    Scratch,
    /// Durable and replayable, sealed to the creating principal.
    #[default]
    AuthorOnly,
    /// Collaboratively admitted visibility.
    Shared,
}

impl RevelationPosture {
    /// Stable wire tag for canonical serialization and digest domains.
    #[must_use]
    pub fn canonical_tag(self) -> u8 {
        match self {
            Self::Scratch => 0x01,
            Self::AuthorOnly => 0x02,
            Self::Shared => 0x03,
        }
    }
}

/// Witnessed record that one artifact's posture lawfully widened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosturePromotion {
    /// Posture before the promotion act.
    pub from: RevelationPosture,
    /// Posture after the promotion act.
    pub to: RevelationPosture,
    /// Witness digest binding the explicit promotion act.
    pub witness: Hash,
}

/// Obstruction raised when a posture act is unlawful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostureObstruction {
    /// Posture may only widen; narrowing is never a promotion.
    NarrowingRefused {
        /// Posture the artifact currently holds.
        from: RevelationPosture,
        /// Narrower posture that was unlawfully requested.
        requested: RevelationPosture,
    },
    /// Promotion to the same posture is a no-op dressed as an act.
    AlreadyAtPosture {
        /// Posture the artifact already holds.
        posture: RevelationPosture,
    },
    /// A composite shell may not reveal more than its least-revealed member.
    ExceedsLeastRevealedMember {
        /// Posture requested for the composite shell.
        shell: RevelationPosture,
        /// Least-revealed posture among the shell's members.
        least_revealed_member: RevelationPosture,
    },
    /// A witness digest must never be a 32-byte shrug.
    EmptyWitness,
}

/// Witness digest with a quality bar: zero and empty-input digests refused.
///
/// The witnessed-act law is enforced by the type system: any API that takes
/// a `WitnessDigest` cannot be handed a shrug, because the shrug never
/// constructs. Shared by posture promotion and the braid shell family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WitnessDigest(Hash);

impl WitnessDigest {
    /// Wraps a witness digest, refusing shrug values.
    ///
    /// # Errors
    ///
    /// Returns [`PostureObstruction::EmptyWitness`] for the all-zero digest
    /// and the digest of empty input.
    pub fn new(hash: Hash) -> Result<Self, PostureObstruction> {
        if hash == [0; 32] || hash == crate::blake3_empty() {
            return Err(PostureObstruction::EmptyWitness);
        }
        Ok(Self(hash))
    }

    /// Returns the underlying digest.
    #[must_use]
    pub fn as_hash(&self) -> &Hash {
        &self.0
    }
}

/// Returns the least-revealed posture among `members`.
///
/// An empty member set has no revelation to leak, so it imposes no bound;
/// this returns `None` and callers treat the shell posture as the only
/// constraint.
#[must_use]
pub fn least_revealed<I>(members: I) -> Option<RevelationPosture>
where
    I: IntoIterator<Item = RevelationPosture>,
{
    members.into_iter().min()
}

/// Checks the least-revealed-member invariant for a composite shell.
///
/// Returns the obstruction when `shell` would reveal more than the
/// least-revealed member; `None` means the posture is admissible.
#[must_use]
pub fn shell_posture_obstruction<I>(
    shell: RevelationPosture,
    members: I,
) -> Option<PostureObstruction>
where
    I: IntoIterator<Item = RevelationPosture>,
{
    let floor = least_revealed(members)?;
    if shell > floor {
        return Some(PostureObstruction::ExceedsLeastRevealedMember {
            shell,
            least_revealed_member: floor,
        });
    }
    None
}

/// Performs one explicit, witnessed posture promotion.
///
/// Promotion only widens posture. Narrowing and same-posture requests are
/// obstructions: a posture change must always be a real, witnessed act. The
/// witness arrives as a [`WitnessDigest`], so a shrug witness cannot reach
/// this function — the type system holds the door.
///
/// # Errors
///
/// Returns [`PostureObstruction::NarrowingRefused`] when `to` is narrower
/// than `from`, and [`PostureObstruction::AlreadyAtPosture`] when `to`
/// equals `from`.
pub fn promote_posture(
    from: RevelationPosture,
    to: RevelationPosture,
    witness: WitnessDigest,
) -> Result<PosturePromotion, PostureObstruction> {
    if to < from {
        return Err(PostureObstruction::NarrowingRefused {
            from,
            requested: to,
        });
    }
    if to == from {
        return Err(PostureObstruction::AlreadyAtPosture { posture: from });
    }
    Ok(PosturePromotion {
        from,
        to,
        witness: *witness.as_hash(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn witness() -> WitnessDigest {
        WitnessDigest::new([0xA7; 32]).unwrap()
    }

    #[test]
    fn witness_digest_refuses_shrug_values() {
        assert_eq!(
            WitnessDigest::new([0; 32]),
            Err(PostureObstruction::EmptyWitness)
        );
        assert_eq!(
            WitnessDigest::new(crate::blake3_empty()),
            Err(PostureObstruction::EmptyWitness)
        );
        assert!(WitnessDigest::new([0x99; 32]).is_ok());
    }

    #[test]
    fn posture_defaults_to_author_only() {
        assert_eq!(RevelationPosture::default(), RevelationPosture::AuthorOnly);
    }

    #[test]
    fn revelation_breadth_orders_scratch_below_author_only_below_shared() {
        assert!(RevelationPosture::Scratch < RevelationPosture::AuthorOnly);
        assert!(RevelationPosture::AuthorOnly < RevelationPosture::Shared);
    }

    #[test]
    fn canonical_tags_are_stable() {
        assert_eq!(RevelationPosture::Scratch.canonical_tag(), 0x01);
        assert_eq!(RevelationPosture::AuthorOnly.canonical_tag(), 0x02);
        assert_eq!(RevelationPosture::Shared.canonical_tag(), 0x03);
    }

    #[test]
    fn least_revealed_finds_the_floor() {
        assert_eq!(
            least_revealed([
                RevelationPosture::Shared,
                RevelationPosture::Scratch,
                RevelationPosture::AuthorOnly,
            ]),
            Some(RevelationPosture::Scratch)
        );
        assert_eq!(least_revealed([]), None);
    }

    #[test]
    fn shell_cannot_reveal_more_than_least_revealed_member() {
        let obstruction = shell_posture_obstruction(
            RevelationPosture::Shared,
            [RevelationPosture::Shared, RevelationPosture::AuthorOnly],
        );

        assert_eq!(
            obstruction,
            Some(PostureObstruction::ExceedsLeastRevealedMember {
                shell: RevelationPosture::Shared,
                least_revealed_member: RevelationPosture::AuthorOnly,
            })
        );
    }

    #[test]
    fn shell_at_or_below_member_floor_is_admissible() {
        assert_eq!(
            shell_posture_obstruction(
                RevelationPosture::AuthorOnly,
                [RevelationPosture::Shared, RevelationPosture::AuthorOnly],
            ),
            None
        );
        assert_eq!(
            shell_posture_obstruction(RevelationPosture::Scratch, [RevelationPosture::AuthorOnly],),
            None
        );
    }

    #[test]
    fn empty_member_set_imposes_no_floor() {
        assert_eq!(
            shell_posture_obstruction(RevelationPosture::Shared, []),
            None
        );
    }

    #[test]
    fn promotion_widens_with_witness() {
        assert_eq!(
            promote_posture(
                RevelationPosture::AuthorOnly,
                RevelationPosture::Shared,
                witness(),
            ),
            Ok(PosturePromotion {
                from: RevelationPosture::AuthorOnly,
                to: RevelationPosture::Shared,
                witness: *witness().as_hash(),
            })
        );
    }

    #[test]
    fn narrowing_is_refused_not_silently_applied() {
        assert_eq!(
            promote_posture(
                RevelationPosture::Shared,
                RevelationPosture::AuthorOnly,
                witness(),
            ),
            Err(PostureObstruction::NarrowingRefused {
                from: RevelationPosture::Shared,
                requested: RevelationPosture::AuthorOnly,
            })
        );
    }

    #[test]
    fn promotion_to_same_posture_is_an_obstruction_not_a_noop() {
        assert_eq!(
            promote_posture(
                RevelationPosture::AuthorOnly,
                RevelationPosture::AuthorOnly,
                witness(),
            ),
            Err(PostureObstruction::AlreadyAtPosture {
                posture: RevelationPosture::AuthorOnly,
            })
        );
    }
}
