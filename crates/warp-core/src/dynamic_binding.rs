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

/// Runtime failures while resolving concrete structured bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicBindingRuntimeError {
    /// A direct slot could not bind because the referenced target does not exist.
    MissingDirectSlotTarget {
        /// Authored slot name that failed to resolve.
        slot: String,
        /// Invocation-supplied id or label used for the lookup.
        reference: String,
    },
    /// A relation-derived slot could not bind because its source slot was absent.
    MissingRelationSource {
        /// Authored slot name that failed to resolve.
        slot: String,
        /// Upstream authored slot name that should have been bound first.
        from_slot: String,
    },
    /// A relation-derived slot could not bind because the declared relation had no target.
    MissingRelationTarget {
        /// Authored slot name that failed to resolve.
        slot: String,
        /// Upstream authored slot name used for the relation.
        from_slot: String,
        /// Declared relation that produced no target.
        relation: String,
    },
    /// A closure source slot was absent at binding time.
    MissingClosureSource {
        /// Authored closure slot name that failed to resolve.
        slot: String,
        /// Upstream authored slot name that should have been bound first.
        from_slot: String,
    },
    /// A declared closure operator was not recognized by the runtime resolver.
    UnknownClosureOperator {
        /// Authored closure slot name that failed to resolve.
        slot: String,
        /// Closure operator name that the runtime did not recognize.
        operator: String,
    },
    /// A closure range request was invalid for the resolved basis head.
    InvalidClosureRange {
        /// Authored closure slot name that failed to resolve.
        slot: String,
        /// Requested start byte.
        start: usize,
        /// Requested end byte.
        end: usize,
        /// Available byte length on the basis head.
        limit: usize,
    },
    /// A base head did not belong to the requested worldline.
    BasisHeadMismatch {
        /// Requested worldline binding.
        worldline: String,
        /// Requested basis head binding.
        head: String,
    },
    /// Lower-level binding insertion failed after resolution.
    BindingCollision(DynamicBindingError),
}

impl From<DynamicBindingError> for DynamicBindingRuntimeError {
    fn from(value: DynamicBindingError) -> Self {
        Self::BindingCollision(value)
    }
}

/// Request for resolving one declared range-scoped closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeClosureBindingRequest<'a> {
    /// Authored closure slot name.
    pub slot: &'a str,
    /// Authored source slot from which the closure is derived.
    pub from_slot: &'a str,
    /// Declared closure operator.
    pub operator: &'a str,
    /// Optional second authored slot needed by the operator.
    pub related_slot: Option<&'a str>,
    /// Requested range start.
    pub start: usize,
    /// Requested range end.
    pub end: usize,
}

/// Runtime adapter that resolves structured footprint bindings against graph truth.
pub trait StructuredBindingRuntime {
    /// Resolves one direct slot from invocation-supplied reference material.
    fn resolve_direct_slot(
        &self,
        slot: &str,
        kind: &str,
        reference: &str,
    ) -> Result<BoundNodeRef, DynamicBindingRuntimeError>;

    /// Resolves one relation-derived slot from an already bound source slot.
    fn resolve_relation_slot(
        &self,
        slot: &str,
        from_slot: &str,
        relation: &str,
        source: &BoundNodeRef,
        expected_kind: &str,
    ) -> Result<BoundNodeRef, DynamicBindingRuntimeError>;

    /// Resolves one range-scoped closure from an already bound source slot.
    fn resolve_range_closure(
        &self,
        request: &RangeClosureBindingRequest<'_>,
        source: &BoundNodeRef,
        related: Option<&BoundNodeRef>,
    ) -> Result<Vec<ClosureMemberBinding>, DynamicBindingRuntimeError>;
}

/// Internal resolver that accumulates structured runtime bindings.
#[derive(Debug)]
pub struct StructuredBindingResolver<'a, R> {
    runtime: &'a R,
    bindings: StructuredRuntimeBindings,
}

impl<'a, R> StructuredBindingResolver<'a, R>
where
    R: StructuredBindingRuntime,
{
    /// Creates a new resolver over `runtime`.
    #[must_use]
    pub fn new(runtime: &'a R) -> Self {
        Self {
            runtime,
            bindings: StructuredRuntimeBindings::new(),
        }
    }

    /// Returns the currently accumulated runtime bindings.
    #[must_use]
    pub fn bindings(&self) -> &StructuredRuntimeBindings {
        &self.bindings
    }

    /// Consumes the resolver and returns the accumulated runtime bindings.
    #[must_use]
    pub fn into_bindings(self) -> StructuredRuntimeBindings {
        self.bindings
    }

    /// Resolves and records one direct slot from invocation input.
    pub fn bind_direct_slot(
        &mut self,
        slot: &str,
        kind: &str,
        reference: &str,
    ) -> Result<(), DynamicBindingRuntimeError> {
        let binding = self.runtime.resolve_direct_slot(slot, kind, reference)?;
        self.bindings.bind_direct_slot(slot, binding)?;
        Ok(())
    }

    /// Resolves and records one relation-derived slot from an existing binding.
    pub fn bind_relation_slot(
        &mut self,
        slot: &str,
        kind: &str,
        from_slot: &str,
        relation: &str,
    ) -> Result<(), DynamicBindingRuntimeError> {
        let source = self.bindings.slot(from_slot).ok_or_else(|| {
            DynamicBindingRuntimeError::MissingRelationSource {
                slot: slot.to_owned(),
                from_slot: from_slot.to_owned(),
            }
        })?;
        let binding = self.runtime.resolve_relation_slot(
            slot,
            from_slot,
            relation,
            source.binding(),
            kind,
        )?;
        self.bindings
            .bind_relation_slot(slot, from_slot, relation, binding)?;
        Ok(())
    }

    /// Resolves and records one declared range-scoped closure.
    pub fn bind_range_closure(
        &mut self,
        request: RangeClosureBindingRequest<'_>,
    ) -> Result<(), DynamicBindingRuntimeError> {
        let source = self.bindings.slot(request.from_slot).ok_or_else(|| {
            DynamicBindingRuntimeError::MissingClosureSource {
                slot: request.slot.to_owned(),
                from_slot: request.from_slot.to_owned(),
            }
        })?;
        let related = request
            .related_slot
            .map(|slot_name| {
                self.bindings.slot(slot_name).ok_or_else(|| {
                    DynamicBindingRuntimeError::MissingClosureSource {
                        slot: request.slot.to_owned(),
                        from_slot: slot_name.to_owned(),
                    }
                })
            })
            .transpose()?;
        let members = self.runtime.resolve_range_closure(
            &request,
            source.binding(),
            related.map(ResolvedSlotBinding::binding),
        )?;
        self.bindings
            .bind_closure(request.slot, request.from_slot, request.operator, members)?;
        Ok(())
    }
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
    use std::collections::BTreeMap;

    #[derive(Debug, Clone)]
    struct MockWorldline {
        id: String,
        binding: BoundNodeRef,
        canonical_head_id: Option<String>,
    }

    #[derive(Debug, Clone)]
    struct MockHead {
        id: String,
        binding: BoundNodeRef,
        worldline_id: String,
        byte_length: usize,
        rope_members: Vec<MockRopeMember>,
    }

    #[derive(Debug, Clone)]
    struct MockRopeMember {
        start: usize,
        end: usize,
        binding: ClosureMemberBinding,
    }

    #[derive(Debug, Clone)]
    struct MockAnchor {
        basis_head_id: String,
        start: usize,
        end: usize,
        binding: ClosureMemberBinding,
    }

    #[derive(Debug, Clone)]
    struct MockTextRuntime {
        worldlines: BTreeMap<String, MockWorldline>,
        heads: BTreeMap<String, MockHead>,
        anchors: Vec<MockAnchor>,
    }

    #[derive(Debug, Clone)]
    struct ReplaceRangeAsTickBindingRequest {
        worldline_id: String,
        base_head_id: String,
        start_byte: usize,
        end_byte: usize,
    }

    #[derive(Debug, Clone)]
    struct CreateCheckpointBindingRequest {
        worldline_id: String,
    }

    fn overlap(start: usize, end: usize, other_start: usize, other_end: usize) -> bool {
        start < other_end && other_start < end
    }

    impl StructuredBindingRuntime for MockTextRuntime {
        fn resolve_direct_slot(
            &self,
            slot: &str,
            _kind: &str,
            reference: &str,
        ) -> Result<BoundNodeRef, DynamicBindingRuntimeError> {
            match slot {
                "worldline" => self
                    .worldlines
                    .get(reference)
                    .map(|worldline| worldline.binding.clone())
                    .ok_or_else(|| DynamicBindingRuntimeError::MissingDirectSlotTarget {
                        slot: slot.to_owned(),
                        reference: reference.to_owned(),
                    }),
                "baseHead" => self
                    .heads
                    .get(reference)
                    .map(|head| head.binding.clone())
                    .ok_or_else(|| DynamicBindingRuntimeError::MissingDirectSlotTarget {
                        slot: slot.to_owned(),
                        reference: reference.to_owned(),
                    }),
                _ => Err(DynamicBindingRuntimeError::MissingDirectSlotTarget {
                    slot: slot.to_owned(),
                    reference: reference.to_owned(),
                }),
            }
        }

        fn resolve_relation_slot(
            &self,
            slot: &str,
            from_slot: &str,
            relation: &str,
            source: &BoundNodeRef,
            _expected_kind: &str,
        ) -> Result<BoundNodeRef, DynamicBindingRuntimeError> {
            if from_slot != "worldline" || relation != "CANONICAL_HEAD" {
                return Err(DynamicBindingRuntimeError::MissingRelationTarget {
                    slot: slot.to_owned(),
                    from_slot: from_slot.to_owned(),
                    relation: relation.to_owned(),
                });
            }

            let worldline = self
                .worldlines
                .values()
                .find(|worldline| worldline.binding == *source)
                .ok_or_else(|| DynamicBindingRuntimeError::MissingRelationTarget {
                    slot: slot.to_owned(),
                    from_slot: from_slot.to_owned(),
                    relation: relation.to_owned(),
                })?;

            let canonical_head_id = worldline.canonical_head_id.clone().ok_or_else(|| {
                DynamicBindingRuntimeError::MissingRelationTarget {
                    slot: slot.to_owned(),
                    from_slot: from_slot.to_owned(),
                    relation: relation.to_owned(),
                }
            })?;

            self.heads
                .get(&canonical_head_id)
                .map(|head| head.binding.clone())
                .ok_or_else(|| DynamicBindingRuntimeError::MissingRelationTarget {
                    slot: slot.to_owned(),
                    from_slot: from_slot.to_owned(),
                    relation: relation.to_owned(),
                })
        }

        fn resolve_range_closure(
            &self,
            request: &RangeClosureBindingRequest<'_>,
            source: &BoundNodeRef,
            related: Option<&BoundNodeRef>,
        ) -> Result<Vec<ClosureMemberBinding>, DynamicBindingRuntimeError> {
            match request.operator {
                "ropeRangeClosure" => {
                    let base_head = self
                        .heads
                        .values()
                        .find(|head| head.binding == *source)
                        .ok_or_else(|| DynamicBindingRuntimeError::MissingDirectSlotTarget {
                            slot: request.slot.to_owned(),
                            reference: source.kind.clone(),
                        })?;

                    if request.start > request.end || request.end > base_head.byte_length {
                        return Err(DynamicBindingRuntimeError::InvalidClosureRange {
                            slot: request.slot.to_owned(),
                            start: request.start,
                            end: request.end,
                            limit: base_head.byte_length,
                        });
                    }

                    Ok(base_head
                        .rope_members
                        .iter()
                        .filter(|member| {
                            overlap(request.start, request.end, member.start, member.end)
                        })
                        .map(|member| member.binding.clone())
                        .collect())
                }
                "anchorsIntersectingEditWindow" => {
                    let worldline = self
                        .worldlines
                        .values()
                        .find(|worldline| worldline.binding == *source)
                        .ok_or_else(|| DynamicBindingRuntimeError::MissingDirectSlotTarget {
                            slot: request.slot.to_owned(),
                            reference: source.kind.clone(),
                        })?;
                    let related = related.ok_or_else(|| {
                        DynamicBindingRuntimeError::MissingClosureSource {
                            slot: request.slot.to_owned(),
                            from_slot: "baseHead".to_owned(),
                        }
                    })?;
                    let base_head = self
                        .heads
                        .values()
                        .find(|head| head.binding == *related)
                        .ok_or_else(|| DynamicBindingRuntimeError::MissingDirectSlotTarget {
                            slot: request.slot.to_owned(),
                            reference: related.kind.clone(),
                        })?;

                    if base_head.worldline_id != worldline.id {
                        return Err(DynamicBindingRuntimeError::BasisHeadMismatch {
                            worldline: worldline.id.clone(),
                            head: base_head.id.clone(),
                        });
                    }

                    Ok(self
                        .anchors
                        .iter()
                        .filter(|anchor| {
                            anchor.basis_head_id == base_head.id
                                && overlap(request.start, request.end, anchor.start, anchor.end)
                        })
                        .map(|anchor| anchor.binding.clone())
                        .collect())
                }
                _ => Err(DynamicBindingRuntimeError::UnknownClosureOperator {
                    slot: request.slot.to_owned(),
                    operator: request.operator.to_owned(),
                }),
            }
        }
    }

    fn bind_replace_range_as_tick(
        runtime: &MockTextRuntime,
        request: &ReplaceRangeAsTickBindingRequest,
    ) -> Result<StructuredRuntimeBindings, DynamicBindingRuntimeError> {
        let mut resolver = StructuredBindingResolver::new(runtime);
        resolver.bind_direct_slot("worldline", "BufferWorldline", &request.worldline_id)?;
        resolver.bind_direct_slot("baseHead", "RopeHead", &request.base_head_id)?;
        resolver.bind_range_closure(RangeClosureBindingRequest {
            slot: "touchedRope",
            from_slot: "baseHead",
            operator: "ropeRangeClosure",
            related_slot: None,
            start: request.start_byte,
            end: request.end_byte,
        })?;
        resolver.bind_range_closure(RangeClosureBindingRequest {
            slot: "affectedAnchors",
            from_slot: "worldline",
            operator: "anchorsIntersectingEditWindow",
            related_slot: Some("baseHead"),
            start: request.start_byte,
            end: request.end_byte,
        })?;
        Ok(resolver.into_bindings())
    }

    fn bind_create_checkpoint(
        runtime: &MockTextRuntime,
        request: &CreateCheckpointBindingRequest,
    ) -> Result<StructuredRuntimeBindings, DynamicBindingRuntimeError> {
        let mut resolver = StructuredBindingResolver::new(runtime);
        resolver.bind_direct_slot("worldline", "BufferWorldline", &request.worldline_id)?;
        resolver.bind_relation_slot("currentHead", "RopeHead", "worldline", "CANONICAL_HEAD")?;
        Ok(resolver.into_bindings())
    }

    fn mock_runtime() -> MockTextRuntime {
        let warp_id = make_warp_id("binding-warp");
        let worldline_id = "wl:buf-1".to_owned();
        let canonical_head_id = "head:current".to_owned();
        let stale_head_id = "head:stale".to_owned();

        let worldline = MockWorldline {
            id: worldline_id.clone(),
            binding: BoundNodeRef::from_ids(
                "BufferWorldline",
                warp_id,
                make_node_id("buffer-worldline"),
            ),
            canonical_head_id: Some(canonical_head_id.clone()),
        };
        let current_head = MockHead {
            id: canonical_head_id.clone(),
            binding: BoundNodeRef::from_ids("RopeHead", warp_id, make_node_id("head-current")),
            worldline_id: worldline_id.clone(),
            byte_length: 20,
            rope_members: vec![
                MockRopeMember {
                    start: 0,
                    end: 5,
                    binding: ClosureMemberBinding::new(
                        "RopeLeaf",
                        NodeKey {
                            warp_id,
                            local_id: make_node_id("leaf-0-5"),
                        },
                    ),
                },
                MockRopeMember {
                    start: 5,
                    end: 10,
                    binding: ClosureMemberBinding::new(
                        "RopeBranch",
                        NodeKey {
                            warp_id,
                            local_id: make_node_id("branch-5-10"),
                        },
                    ),
                },
                MockRopeMember {
                    start: 10,
                    end: 15,
                    binding: ClosureMemberBinding::new(
                        "TextBlob",
                        NodeKey {
                            warp_id,
                            local_id: make_node_id("blob-10-15"),
                        },
                    ),
                },
            ],
        };
        let stale_head = MockHead {
            id: stale_head_id,
            binding: BoundNodeRef::from_ids("RopeHead", warp_id, make_node_id("head-stale")),
            worldline_id: "wl:other".to_owned(),
            byte_length: 8,
            rope_members: vec![],
        };

        MockTextRuntime {
            worldlines: BTreeMap::from([(worldline_id, worldline)]),
            heads: BTreeMap::from([
                (canonical_head_id.clone(), current_head),
                ("head:stale".to_owned(), stale_head),
            ]),
            anchors: vec![
                MockAnchor {
                    basis_head_id: canonical_head_id.clone(),
                    start: 4,
                    end: 7,
                    binding: ClosureMemberBinding::new(
                        "Anchor",
                        NodeKey {
                            warp_id,
                            local_id: make_node_id("anchor-overlap"),
                        },
                    ),
                },
                MockAnchor {
                    basis_head_id: canonical_head_id,
                    start: 15,
                    end: 18,
                    binding: ClosureMemberBinding::new(
                        "Anchor",
                        NodeKey {
                            warp_id,
                            local_id: make_node_id("anchor-outside"),
                        },
                    ),
                },
            ],
        }
    }

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

    #[test]
    fn mock_replace_range_runtime_binds_direct_slots_and_resolves_declared_closures() {
        let runtime = mock_runtime();
        let bindings = bind_replace_range_as_tick(
            &runtime,
            &ReplaceRangeAsTickBindingRequest {
                worldline_id: "wl:buf-1".to_owned(),
                base_head_id: "head:current".to_owned(),
                start_byte: 4,
                end_byte: 12,
            },
        )
        .expect("bind replace-range runtime");

        let worldline = bindings.slot("worldline").expect("worldline");
        assert_eq!(worldline.binding().kind, "BufferWorldline");

        let base_head = bindings.slot("baseHead").expect("baseHead");
        assert_eq!(base_head.binding().kind, "RopeHead");

        let touched_rope = bindings.closure("touchedRope").expect("touchedRope");
        assert_eq!(touched_rope.operator, "ropeRangeClosure");
        assert_eq!(touched_rope.members.len(), 3);
        assert_eq!(
            touched_rope
                .members
                .iter()
                .map(|member| member.kind.as_str())
                .collect::<Vec<_>>(),
            vec!["RopeLeaf", "RopeBranch", "TextBlob"]
        );

        let affected_anchors = bindings
            .closure("affectedAnchors")
            .expect("affectedAnchors");
        assert_eq!(affected_anchors.operator, "anchorsIntersectingEditWindow");
        assert_eq!(affected_anchors.members.len(), 1);
        assert_eq!(affected_anchors.members[0].kind, "Anchor");
    }

    #[test]
    fn mock_checkpoint_runtime_binds_relation_derived_current_head() {
        let runtime = mock_runtime();
        let bindings = bind_create_checkpoint(
            &runtime,
            &CreateCheckpointBindingRequest {
                worldline_id: "wl:buf-1".to_owned(),
            },
        )
        .expect("bind checkpoint");

        let worldline = bindings.slot("worldline").expect("worldline");
        assert_eq!(worldline.binding().kind, "BufferWorldline");

        let current_head = bindings.slot("currentHead").expect("currentHead");
        match current_head {
            ResolvedSlotBinding::Relation(binding) => {
                assert_eq!(binding.from_slot, "worldline");
                assert_eq!(binding.relation, "CANONICAL_HEAD");
                assert_eq!(binding.binding.kind, "RopeHead");
            }
            ResolvedSlotBinding::Direct(_) => panic!("expected relation-derived currentHead"),
        }
    }

    #[test]
    fn mock_runtime_reports_invalid_replace_range_bindings() {
        let runtime = mock_runtime();

        assert_eq!(
            bind_replace_range_as_tick(
                &runtime,
                &ReplaceRangeAsTickBindingRequest {
                    worldline_id: "wl:missing".to_owned(),
                    base_head_id: "head:current".to_owned(),
                    start_byte: 0,
                    end_byte: 1,
                },
            ),
            Err(DynamicBindingRuntimeError::MissingDirectSlotTarget {
                slot: "worldline".to_owned(),
                reference: "wl:missing".to_owned(),
            })
        );

        assert_eq!(
            bind_replace_range_as_tick(
                &runtime,
                &ReplaceRangeAsTickBindingRequest {
                    worldline_id: "wl:buf-1".to_owned(),
                    base_head_id: "head:stale".to_owned(),
                    start_byte: 0,
                    end_byte: 1,
                },
            ),
            Err(DynamicBindingRuntimeError::BasisHeadMismatch {
                worldline: "wl:buf-1".to_owned(),
                head: "head:stale".to_owned(),
            })
        );

        assert_eq!(
            bind_replace_range_as_tick(
                &runtime,
                &ReplaceRangeAsTickBindingRequest {
                    worldline_id: "wl:buf-1".to_owned(),
                    base_head_id: "head:current".to_owned(),
                    start_byte: 12,
                    end_byte: 21,
                },
            ),
            Err(DynamicBindingRuntimeError::InvalidClosureRange {
                slot: "touchedRope".to_owned(),
                start: 12,
                end: 21,
                limit: 20,
            })
        );
    }

    #[test]
    fn mock_runtime_reports_missing_relation_derived_checkpoint_head() {
        let mut runtime = mock_runtime();
        runtime
            .worldlines
            .get_mut("wl:buf-1")
            .expect("worldline")
            .canonical_head_id = None;

        assert_eq!(
            bind_create_checkpoint(
                &runtime,
                &CreateCheckpointBindingRequest {
                    worldline_id: "wl:buf-1".to_owned(),
                },
            ),
            Err(DynamicBindingRuntimeError::MissingRelationTarget {
                slot: "currentHead".to_owned(),
                from_slot: "worldline".to_owned(),
                relation: "CANONICAL_HEAD".to_owned(),
            })
        );
    }
}
