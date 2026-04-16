// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime nouns for Wesley-authored structured footprint binding.
//!
//! Wesley owns the static schema for:
//!
//! - slots
//! - relation bindings
//! - closure operators
//! - create/update surfaces
//! - forbidden surfaces
//!
//! Echo owns the runtime truth of binding those declarations to concrete graph
//! entities. This module is the first explicit home for that runtime binding
//! vocabulary: direct slots, relation-derived slots, and resolved closures.

use std::collections::BTreeMap;

use crate::ident::{NodeId, NodeKey, WarpId};

/// Concrete graph-native reference bound to one structured slot or closure item.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundNodeRef {
    /// Authored graph kind name from the Wesley contract.
    pub kind: String,
    /// Concrete graph node key resolved by Echo.
    pub node: NodeKey,
}

impl BoundNodeRef {
    /// Constructs a bound node reference from its authored kind and resolved node key.
    #[must_use]
    pub fn new(kind: impl Into<String>, node: NodeKey) -> Self {
        Self {
            kind: kind.into(),
            node,
        }
    }

    /// Convenience constructor for separate warp/node ids.
    #[must_use]
    pub fn from_ids(kind: impl Into<String>, warp_id: WarpId, node_id: NodeId) -> Self {
        Self::new(
            kind,
            NodeKey {
                warp_id,
                local_id: node_id,
            },
        )
    }
}

/// Direct slot bound from explicit invocation arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectSlotBinding {
    /// Authored slot name from the Wesley footprint contract.
    pub slot: String,
    /// Resolved graph-native node reference for that slot.
    pub binding: BoundNodeRef,
}

/// Slot derived from another bound slot through one declared relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationSlotBinding {
    /// Authored slot name from the Wesley footprint contract.
    pub slot: String,
    /// Upstream slot from which the relation was resolved.
    pub from_slot: String,
    /// Declared relation operator used for the bind.
    pub relation: String,
    /// Resolved graph-native node reference for the derived slot.
    pub binding: BoundNodeRef,
}

/// One resolved member of a declared closure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureMemberBinding {
    /// Authored graph kind name of this member.
    pub kind: String,
    /// Resolved graph-native node reference for the member.
    pub node: NodeKey,
}

impl ClosureMemberBinding {
    /// Constructs one closure member binding.
    #[must_use]
    pub fn new(kind: impl Into<String>, node: NodeKey) -> Self {
        Self {
            kind: kind.into(),
            node,
        }
    }
}

/// Resolved closure over runtime graph truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedClosureBinding {
    /// Authored closure slot name from the Wesley footprint contract.
    pub slot: String,
    /// Upstream slot from which the closure was derived.
    pub from_slot: String,
    /// Declared closure operator used to derive the members.
    pub operator: String,
    /// Resolved closure members in runtime order.
    pub members: Vec<ClosureMemberBinding>,
}

/// Resolved slot binding, either direct from args or relation-derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedSlotBinding {
    /// Slot bound directly from invocation arguments.
    Direct(DirectSlotBinding),
    /// Slot bound by following one declared relation from another bound slot.
    Relation(RelationSlotBinding),
}

impl ResolvedSlotBinding {
    /// Returns the authored slot name.
    #[must_use]
    pub fn slot(&self) -> &str {
        match self {
            Self::Direct(binding) => &binding.slot,
            Self::Relation(binding) => &binding.slot,
        }
    }

    /// Returns the resolved graph-native binding.
    #[must_use]
    pub fn binding(&self) -> &BoundNodeRef {
        match self {
            Self::Direct(binding) => &binding.binding,
            Self::Relation(binding) => &binding.binding,
        }
    }
}

/// Runtime binding failures for structured footprints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicBindingError {
    /// Attempted to bind the same slot twice.
    DuplicateSlot {
        /// Authored slot name that was bound more than once.
        slot: String,
    },
    /// Attempted to resolve the same closure slot twice.
    DuplicateClosure {
        /// Authored closure slot name that was resolved more than once.
        slot: String,
    },
}

/// Concrete runtime bindings for one structured footprint invocation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StructuredRuntimeBindings {
    slots: BTreeMap<String, ResolvedSlotBinding>,
    closures: BTreeMap<String, ResolvedClosureBinding>,
}

impl StructuredRuntimeBindings {
    /// Creates an empty runtime binding set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds one direct slot from invocation arguments.
    pub fn bind_direct_slot(
        &mut self,
        slot: impl Into<String>,
        binding: BoundNodeRef,
    ) -> Result<(), DynamicBindingError> {
        let slot = slot.into();
        let error_slot = slot.clone();
        match self.slots.insert(
            slot.clone(),
            ResolvedSlotBinding::Direct(DirectSlotBinding { slot, binding }),
        ) {
            Some(_) => Err(DynamicBindingError::DuplicateSlot { slot: error_slot }),
            None => Ok(()),
        }
    }

    /// Binds one relation-derived slot from an existing authored slot.
    pub fn bind_relation_slot(
        &mut self,
        slot: impl Into<String>,
        from_slot: impl Into<String>,
        relation: impl Into<String>,
        binding: BoundNodeRef,
    ) -> Result<(), DynamicBindingError> {
        let slot = slot.into();
        let error_slot = slot.clone();
        match self.slots.insert(
            slot.clone(),
            ResolvedSlotBinding::Relation(RelationSlotBinding {
                slot,
                from_slot: from_slot.into(),
                relation: relation.into(),
                binding,
            }),
        ) {
            Some(_) => Err(DynamicBindingError::DuplicateSlot { slot: error_slot }),
            None => Ok(()),
        }
    }

    /// Records a resolved closure against its authored slot name.
    pub fn bind_closure(
        &mut self,
        slot: impl Into<String>,
        from_slot: impl Into<String>,
        operator: impl Into<String>,
        members: Vec<ClosureMemberBinding>,
    ) -> Result<(), DynamicBindingError> {
        let slot = slot.into();
        let error_slot = slot.clone();
        match self.closures.insert(
            slot.clone(),
            ResolvedClosureBinding {
                slot,
                from_slot: from_slot.into(),
                operator: operator.into(),
                members,
            },
        ) {
            Some(_) => Err(DynamicBindingError::DuplicateClosure { slot: error_slot }),
            None => Ok(()),
        }
    }

    /// Returns one resolved slot by authored name.
    #[must_use]
    pub fn slot(&self, slot: &str) -> Option<&ResolvedSlotBinding> {
        self.slots.get(slot)
    }

    /// Returns one resolved closure by authored name.
    #[must_use]
    pub fn closure(&self, slot: &str) -> Option<&ResolvedClosureBinding> {
        self.closures.get(slot)
    }

    /// Returns all resolved slots in deterministic authored-name order.
    pub fn slots(&self) -> impl Iterator<Item = &ResolvedSlotBinding> {
        self.slots.values()
    }

    /// Returns all resolved closures in deterministic authored-name order.
    pub fn closures(&self) -> impl Iterator<Item = &ResolvedClosureBinding> {
        self.closures.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ident::{make_node_id, make_warp_id};

    #[test]
    fn structured_runtime_bindings_store_direct_relation_and_closure_bindings() {
        let warp_id = make_warp_id("binding-warp");
        let worldline =
            BoundNodeRef::from_ids("BufferWorldline", warp_id, make_node_id("worldline"));
        let base_head = BoundNodeRef::from_ids("RopeHead", warp_id, make_node_id("base-head"));
        let branch = ClosureMemberBinding::new(
            "RopeBranch",
            NodeKey {
                warp_id,
                local_id: make_node_id("branch"),
            },
        );
        let leaf = ClosureMemberBinding::new(
            "RopeLeaf",
            NodeKey {
                warp_id,
                local_id: make_node_id("leaf"),
            },
        );
        let blob = ClosureMemberBinding::new(
            "TextBlob",
            NodeKey {
                warp_id,
                local_id: make_node_id("blob"),
            },
        );

        let mut bindings = StructuredRuntimeBindings::new();
        bindings
            .bind_direct_slot("worldline", worldline.clone())
            .expect("bind direct slot");
        bindings
            .bind_relation_slot("baseHead", "worldline", "CANONICAL_HEAD", base_head.clone())
            .expect("bind relation slot");
        bindings
            .bind_closure(
                "touchedRope",
                "baseHead",
                "ropeRangeClosure",
                vec![branch.clone(), leaf.clone(), blob.clone()],
            )
            .expect("bind closure");

        let worldline_binding = bindings.slot("worldline").expect("worldline slot");
        assert_eq!(worldline_binding.binding(), &worldline);
        assert_eq!(worldline_binding.slot(), "worldline");

        let base_head_binding = bindings.slot("baseHead").expect("baseHead slot");
        assert_eq!(base_head_binding.binding(), &base_head);
        match base_head_binding {
            ResolvedSlotBinding::Relation(binding) => {
                assert_eq!(binding.from_slot, "worldline");
                assert_eq!(binding.relation, "CANONICAL_HEAD");
            }
            ResolvedSlotBinding::Direct(_) => panic!("expected relation-derived slot"),
        }

        let closure = bindings
            .closure("touchedRope")
            .expect("touchedRope closure");
        assert_eq!(closure.from_slot, "baseHead");
        assert_eq!(closure.operator, "ropeRangeClosure");
        assert_eq!(closure.members, vec![branch, leaf, blob]);
    }

    #[test]
    fn structured_runtime_bindings_reject_duplicate_slot_and_closure_names() {
        let warp_id = make_warp_id("binding-warp");
        let mut bindings = StructuredRuntimeBindings::new();

        bindings
            .bind_direct_slot(
                "worldline",
                BoundNodeRef::from_ids("BufferWorldline", warp_id, make_node_id("worldline")),
            )
            .expect("initial direct slot bind");
        assert_eq!(
            bindings.bind_relation_slot(
                "worldline",
                "worldline",
                "CANONICAL_HEAD",
                BoundNodeRef::from_ids("RopeHead", warp_id, make_node_id("head")),
            ),
            Err(DynamicBindingError::DuplicateSlot {
                slot: "worldline".to_owned(),
            })
        );

        bindings
            .bind_closure("touchedRope", "worldline", "ropeRangeClosure", vec![])
            .expect("initial closure bind");
        assert_eq!(
            bindings.bind_closure("touchedRope", "worldline", "ropeRangeClosure", vec![]),
            Err(DynamicBindingError::DuplicateClosure {
                slot: "touchedRope".to_owned(),
            })
        );
    }
}
