// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Evolving coordination log ("Braid") representation.

use thiserror::Error;

use crate::braid_shell::BraidMemberRef;
use crate::ident::Hash;
use crate::revelation::AuthorityDomainRef;

/// Concrete status of a coordination braid lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BraidStatus {
    /// Active coordination state, accepting member weaving.
    Active,
    /// Settlement has been finalized.
    Finalized,
    /// Braid has been collapsed from a plural state to a single derived state.
    Collapsed,
}

/// Error kinds returned during coordination braid lifecycle updates or folds.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BraidError {
    /// The event log was empty.
    #[error("empty event stream")]
    EmptyLog,
    /// The event log did not begin with BraidCreated.
    #[error("first event must be BraidCreated")]
    MissingCreated,
    /// The BraidCreated event appeared more than once.
    #[error("BraidCreated event can only appear once at the start of the log")]
    DuplicateCreated,
    /// The member sequence number was out of order.
    #[error("incoherent member sequence: expected {expected}, got {actual}")]
    IncoherentSequence {
        /// Expected sequence number.
        expected: u64,
        /// Actual sequence number.
        actual: u64,
    },
    /// An invalid transition was attempted for the current braid status.
    #[error("cannot transition braid state: cannot {action} in status {status:?}")]
    InvalidTransition {
        /// Attempted action or event kind.
        action: String,
        /// Current braid status.
        status: BraidStatus,
    },
}

/// Lifecycle events that define the evolution of a coordination braid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BraidEvent {
    /// Initial creation of the coordination braid.
    BraidCreated {
        /// Unique content-addressed identifier.
        braid_id: Hash,
        /// Authority domain under which this braid was initiated.
        creator_domain: AuthorityDomainRef,
    },
    /// A member strand was woven into the braid's speculative frontier.
    MemberWoven {
        /// Reference to the strand, which may be revealed or sealed.
        member_ref: BraidMemberRef,
        /// Monotonically increasing sequence number.
        sequence_num: u64,
    },
    /// A settlement was finalized, binding the current braid state.
    SettlementFinalized {
        /// Content digest of the final braid shell.
        settlement_digest: Hash,
    },
    /// A previously plural braid was collapsed into a single derived resolution.
    BraidCollapsed {
        /// Witness digest proving the collapse transition.
        collapse_witness: Hash,
        /// Content digest of the new derived braid shell.
        outcome_digest: Hash,
    },
}

/// Evolving state of a coordination braid reconstructed from its event log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Braid {
    /// Unique identifier of the braid.
    pub braid_id: Hash,
    /// Ordered event stream.
    pub events: Vec<BraidEvent>,
    /// Set of current member references in coordination order.
    pub members: Vec<BraidMemberRef>,
    /// Expected sequence number of the next member to be woven.
    pub next_sequence_num: u64,
    /// Digest of the latest finalized settlement, if any.
    pub latest_settlement: Option<Hash>,
    /// Current lifecycle status of the braid.
    pub status: BraidStatus,
}

impl Braid {
    /// Creates a new coordination braid state with a creation event.
    #[must_use]
    pub fn new(braid_id: Hash, creator_domain: AuthorityDomainRef) -> Self {
        let initial_event = BraidEvent::BraidCreated {
            braid_id,
            creator_domain,
        };
        let mut braid = Self {
            braid_id,
            events: Vec::new(),
            members: Vec::new(),
            next_sequence_num: 0,
            latest_settlement: None,
            status: BraidStatus::Active,
        };
        braid.apply(initial_event);
        braid
    }

    /// Appends an event to the log and updates the folded state.
    pub fn apply(&mut self, event: BraidEvent) {
        match &event {
            BraidEvent::BraidCreated { braid_id, .. } => {
                self.braid_id = *braid_id;
                self.status = BraidStatus::Active;
            }
            BraidEvent::MemberWoven {
                member_ref,
                sequence_num,
            } => {
                self.members.push(*member_ref);
                self.next_sequence_num = sequence_num + 1;
            }
            BraidEvent::SettlementFinalized { settlement_digest } => {
                self.latest_settlement = Some(*settlement_digest);
                self.status = BraidStatus::Finalized;
            }
            BraidEvent::BraidCollapsed { outcome_digest, .. } => {
                self.latest_settlement = Some(*outcome_digest);
                self.status = BraidStatus::Collapsed;
            }
        }
        self.events.push(event);
    }

    /// Reconstructs the braid state by folding over a stream of events.
    ///
    /// # Errors
    ///
    /// Returns a [`BraidError`] if the event log is empty, if the log does
    /// not begin with `BraidCreated`, or if any sequence numbering is incoherent.
    pub fn fold(events: impl IntoIterator<Item = BraidEvent>) -> Result<Self, BraidError> {
        let mut iter = events.into_iter();
        let first = iter.next().ok_or(BraidError::EmptyLog)?;

        let mut braid = match &first {
            BraidEvent::BraidCreated {
                braid_id,
                creator_domain,
            } => Self::new(*braid_id, *creator_domain),
            _ => return Err(BraidError::MissingCreated),
        };

        for event in iter {
            match &event {
                BraidEvent::BraidCreated { .. } => {
                    return Err(BraidError::DuplicateCreated);
                }
                BraidEvent::MemberWoven { sequence_num, .. } => {
                    if braid.status != BraidStatus::Active {
                        return Err(BraidError::InvalidTransition {
                            action: "weave member".to_string(),
                            status: braid.status,
                        });
                    }
                    if *sequence_num != braid.next_sequence_num {
                        return Err(BraidError::IncoherentSequence {
                            expected: braid.next_sequence_num,
                            actual: *sequence_num,
                        });
                    }
                }
                BraidEvent::SettlementFinalized { .. } => {
                    if braid.status != BraidStatus::Active {
                        return Err(BraidError::InvalidTransition {
                            action: "finalize settlement".to_string(),
                            status: braid.status,
                        });
                    }
                }
                BraidEvent::BraidCollapsed { .. } => {
                    if braid.status != BraidStatus::Finalized {
                        return Err(BraidError::InvalidTransition {
                            action: "collapse braid".to_string(),
                            status: braid.status,
                        });
                    }
                }
            }
            braid.apply(event);
        }
        Ok(braid)
    }

    /// Returns the current coordination frontier (active woven members).
    #[must_use]
    pub fn frontier(&self) -> &[BraidMemberRef] {
        &self.members
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::strand::make_strand_id;

    fn authority_ref() -> AuthorityDomainRef {
        AuthorityDomainRef {
            origin_id: crate::revelation::OriginId::from_bytes([0x10; 32]),
            domain_id: crate::revelation::AuthorityDomainId::from_bytes([0x20; 32]),
        }
    }

    #[test]
    fn test_braid_lifecycle_and_folding() {
        let braid_id = [0xAA; 32];
        let auth = authority_ref();

        let mut braid = Braid::new(braid_id, auth);
        assert_eq!(braid.braid_id, braid_id);
        assert_eq!(braid.next_sequence_num, 0);
        assert_eq!(braid.status, BraidStatus::Active);
        assert!(braid.members.is_empty());

        let m1 = BraidMemberRef::Revealed(make_strand_id("strand-1"));
        braid.apply(BraidEvent::MemberWoven {
            member_ref: m1,
            sequence_num: 0,
        });
        assert_eq!(braid.next_sequence_num, 1);
        assert_eq!(braid.members, vec![m1]);
        assert_eq!(braid.status, BraidStatus::Active);

        let m2 = BraidMemberRef::Revealed(make_strand_id("strand-2"));
        braid.apply(BraidEvent::MemberWoven {
            member_ref: m2,
            sequence_num: 1,
        });
        assert_eq!(braid.next_sequence_num, 2);
        assert_eq!(braid.members, vec![m1, m2]);
        assert_eq!(braid.frontier(), &[m1, m2]);

        let settlement = [0x5E; 32];
        braid.apply(BraidEvent::SettlementFinalized {
            settlement_digest: settlement,
        });
        assert_eq!(braid.latest_settlement, Some(settlement));
        assert_eq!(braid.status, BraidStatus::Finalized);

        let collapse_witness = [0x33; 32];
        let collapse_outcome = [0x88; 32];
        braid.apply(BraidEvent::BraidCollapsed {
            collapse_witness,
            outcome_digest: collapse_outcome,
        });
        assert_eq!(braid.latest_settlement, Some(collapse_outcome));
        assert_eq!(braid.status, BraidStatus::Collapsed);
    }

    #[test]
    fn test_braid_fold_validation() {
        let braid_id = [0xAA; 32];
        let auth = authority_ref();
        let m1 = BraidMemberRef::Revealed(make_strand_id("strand-1"));
        let m2 = BraidMemberRef::Revealed(make_strand_id("strand-2"));
        let settlement = [0x5E; 32];
        let collapse_witness = [0x33; 32];
        let collapse_outcome = [0x88; 32];

        // Valid sequence
        let events = vec![
            BraidEvent::BraidCreated {
                braid_id,
                creator_domain: auth,
            },
            BraidEvent::MemberWoven {
                member_ref: m1,
                sequence_num: 0,
            },
            BraidEvent::MemberWoven {
                member_ref: m2,
                sequence_num: 1,
            },
            BraidEvent::SettlementFinalized {
                settlement_digest: settlement,
            },
            BraidEvent::BraidCollapsed {
                collapse_witness,
                outcome_digest: collapse_outcome,
            },
        ];
        let braid = Braid::fold(events).unwrap();
        assert_eq!(braid.braid_id, braid_id);
        assert_eq!(braid.members, vec![m1, m2]);
        assert_eq!(braid.status, BraidStatus::Collapsed);

        // Invalid: missing initial BraidCreated
        let bad_events_no_created = vec![BraidEvent::MemberWoven {
            member_ref: m1,
            sequence_num: 0,
        }];
        assert_eq!(
            Braid::fold(bad_events_no_created),
            Err(BraidError::MissingCreated)
        );

        // Invalid: duplicate BraidCreated
        let bad_events_dup_created = vec![
            BraidEvent::BraidCreated {
                braid_id,
                creator_domain: auth,
            },
            BraidEvent::BraidCreated {
                braid_id,
                creator_domain: auth,
            },
        ];
        assert_eq!(
            Braid::fold(bad_events_dup_created),
            Err(BraidError::DuplicateCreated)
        );

        // Invalid: out-of-order sequence
        let bad_events_out_of_order = vec![
            BraidEvent::BraidCreated {
                braid_id,
                creator_domain: auth,
            },
            BraidEvent::MemberWoven {
                member_ref: m1,
                sequence_num: 1, // Expected 0
            },
        ];
        assert_eq!(
            Braid::fold(bad_events_out_of_order),
            Err(BraidError::IncoherentSequence {
                expected: 0,
                actual: 1
            })
        );

        // Invalid: MemberWoven after finalized
        let bad_events_weave_after_finalized = vec![
            BraidEvent::BraidCreated {
                braid_id,
                creator_domain: auth,
            },
            BraidEvent::SettlementFinalized {
                settlement_digest: settlement,
            },
            BraidEvent::MemberWoven {
                member_ref: m1,
                sequence_num: 0,
            },
        ];
        assert_eq!(
            Braid::fold(bad_events_weave_after_finalized),
            Err(BraidError::InvalidTransition {
                action: "weave member".to_string(),
                status: BraidStatus::Finalized
            })
        );

        // Invalid: BraidCollapsed before finalized
        let bad_events_collapse_before_finalized = vec![
            BraidEvent::BraidCreated {
                braid_id,
                creator_domain: auth,
            },
            BraidEvent::BraidCollapsed {
                collapse_witness,
                outcome_digest: collapse_outcome,
            },
        ];
        assert_eq!(
            Braid::fold(bad_events_collapse_before_finalized),
            Err(BraidError::InvalidTransition {
                action: "collapse braid".to_string(),
                status: BraidStatus::Active
            })
        );
    }
}
