// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Native Echo graph fact vocabulary.
//!
//! This module defines the first Echo-owned graph fact model. It intentionally
//! stops at object, edge, and property facts: storage, query, materialization,
//! scheduler, admission, witness, receipt, and reading behavior are outside this
//! slice. Receipts explain graph outcomes; they do not replace graph facts.

/// Domain tag for v0 graph fact digests.
pub const GRAPH_FACT_DIGEST_DOMAIN: &str = "echo.graph.fact.v0";

const TAG_ENTITY_CAUSAL: &str = "entity.causal";
const TAG_ENTITY_EXTERNAL: &str = "entity.external";
const TAG_ENTITY_RUNTIME_LOCAL: &str = "entity.runtime-local";
const TAG_ENTITY_TRANSACTION_LOCAL: &str = "entity.transaction-local";
const TAG_FACT_EDGE_EXISTS: &str = "fact.edge.exists";
const TAG_FACT_EDGE_RETIRED: &str = "fact.edge.retired";
const TAG_FACT_OBJECT_EXISTS: &str = "fact.object.exists";
const TAG_FACT_OBJECT_RETIRED: &str = "fact.object.retired";
const TAG_FACT_PROPERTY_CLEARED: &str = "fact.property.cleared";
const TAG_FACT_PROPERTY_SET: &str = "fact.property.set";
const TAG_OPTION_ABSENT: u8 = 0;
const TAG_OPTION_PRESENT: u8 = 1;

/// Durable Echo graph object identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CausalObjectId {
    /// Opaque causal object identifier bytes rendered as a stable string.
    pub id: String,
}

impl CausalObjectId {
    /// Creates a durable causal object id.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Scoped id for not-yet-committed objects inside one transaction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionLocalObjectId {
    /// Transaction scope id.
    pub transaction_id: String,
    /// Substep scope id inside the transaction.
    pub step_id: String,
    /// Local name for the staged object.
    pub local_name: String,
}

impl TransactionLocalObjectId {
    /// Creates a transaction-local object id.
    #[must_use]
    pub fn new(
        transaction_id: impl Into<String>,
        step_id: impl Into<String>,
        local_name: impl Into<String>,
    ) -> Self {
        Self {
            transaction_id: transaction_id.into(),
            step_id: step_id.into(),
            local_name: local_name.into(),
        }
    }
}

/// External adapter reference that must resolve before durable admission.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalEntityRef {
    /// External system or adapter namespace.
    pub system: String,
    /// External id inside the named system.
    pub id: String,
}

impl ExternalEntityRef {
    /// Creates an external entity reference.
    #[must_use]
    pub fn new(system: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            id: id.into(),
        }
    }
}

/// Runtime-local handle that is never durable graph identity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeLocalHandle {
    /// Runtime-local handle kind.
    pub kind: String,
    /// Runtime-local opaque id.
    pub id: String,
}

impl RuntimeLocalHandle {
    /// Creates a runtime-local handle.
    #[must_use]
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            id: id.into(),
        }
    }
}

/// Reference envelope for graph entities used by facts and staged plans.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityRef {
    /// Durable causal object reference.
    Causal(CausalObjectId),
    /// Transaction-local reference scoped to one atomic transaction.
    TransactionLocal(TransactionLocalObjectId),
    /// External adapter reference that must resolve below admission.
    External(ExternalEntityRef),
    /// Runtime-local handle that must not become durable graph identity.
    RuntimeLocal(RuntimeLocalHandle),
}

impl EntityRef {
    /// Returns the causal object id only when this reference is already durable.
    #[must_use]
    pub const fn as_causal_object_id(&self) -> Option<&CausalObjectId> {
        match self {
            Self::Causal(object_id) => Some(object_id),
            Self::TransactionLocal(_) | Self::External(_) | Self::RuntimeLocal(_) => None,
        }
    }

    fn push_digest_input(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Causal(object_id) => {
                push_tag(bytes, TAG_ENTITY_CAUSAL);
                push_labeled_str(bytes, "causal_object_id", &object_id.id);
            }
            Self::TransactionLocal(local_id) => {
                push_tag(bytes, TAG_ENTITY_TRANSACTION_LOCAL);
                push_labeled_str(bytes, "transaction_id", &local_id.transaction_id);
                push_labeled_str(bytes, "step_id", &local_id.step_id);
                push_labeled_str(bytes, "local_name", &local_id.local_name);
            }
            Self::External(external_ref) => {
                push_tag(bytes, TAG_ENTITY_EXTERNAL);
                push_labeled_str(bytes, "external_system", &external_ref.system);
                push_labeled_str(bytes, "external_id", &external_ref.id);
            }
            Self::RuntimeLocal(handle) => {
                push_tag(bytes, TAG_ENTITY_RUNTIME_LOCAL);
                push_labeled_str(bytes, "runtime_handle_kind", &handle.kind);
                push_labeled_str(bytes, "runtime_handle_id", &handle.id);
            }
        }
    }
}

/// Echo graph object kind.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphObjectKind {
    /// Object kind namespace.
    pub namespace: String,
    /// Object kind name.
    pub name: String,
    /// Optional object kind version.
    pub version: Option<String>,
}

impl GraphObjectKind {
    /// Creates an object kind without a version.
    #[must_use]
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: None,
        }
    }

    /// Creates an object kind with an explicit version.
    #[must_use]
    pub fn with_version(
        namespace: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: Some(version.into()),
        }
    }

    fn push_digest_input(&self, bytes: &mut Vec<u8>) {
        push_labeled_str(bytes, "object_kind.namespace", &self.namespace);
        push_labeled_str(bytes, "object_kind.name", &self.name);
        push_labeled_optional_str(bytes, "object_kind.version", self.version.as_deref());
    }
}

/// Echo graph edge kind.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphEdgeKind {
    /// Edge kind namespace.
    pub namespace: String,
    /// Edge kind name.
    pub name: String,
    /// Optional edge kind version.
    pub version: Option<String>,
}

impl GraphEdgeKind {
    /// Creates an edge kind without a version.
    #[must_use]
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: None,
        }
    }

    /// Creates an edge kind with an explicit version.
    #[must_use]
    pub fn with_version(
        namespace: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            version: Some(version.into()),
        }
    }

    fn push_digest_input(&self, bytes: &mut Vec<u8>) {
        push_labeled_str(bytes, "edge_kind.namespace", &self.namespace);
        push_labeled_str(bytes, "edge_kind.name", &self.name);
        push_labeled_optional_str(bytes, "edge_kind.version", self.version.as_deref());
    }
}

/// Digest-addressed graph value reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphValueRef {
    /// Digest of the referenced value bytes.
    pub value_digest: [u8; 32],
    /// Codec identity for the referenced value bytes.
    pub codec: String,
    /// Optional semantic type marker for the value.
    pub semantic_type: Option<String>,
}

impl GraphValueRef {
    /// Creates a graph value reference.
    #[must_use]
    pub fn new(value_digest: [u8; 32], codec: impl Into<String>) -> Self {
        Self {
            value_digest,
            codec: codec.into(),
            semantic_type: None,
        }
    }

    /// Creates a graph value reference with an explicit semantic type marker.
    #[must_use]
    pub fn with_semantic_type(
        value_digest: [u8; 32],
        codec: impl Into<String>,
        semantic_type: impl Into<String>,
    ) -> Self {
        Self {
            value_digest,
            codec: codec.into(),
            semantic_type: Some(semantic_type.into()),
        }
    }

    fn push_digest_input(&self, bytes: &mut Vec<u8>) {
        push_labeled_bytes(bytes, "value.digest", &self.value_digest);
        push_labeled_str(bytes, "value.codec", &self.codec);
        push_labeled_optional_str(bytes, "value.semantic_type", self.semantic_type.as_deref());
    }
}

/// Native Echo graph fact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphFact {
    /// A causal graph object exists with a specific object kind.
    ObjectExists {
        /// Durable object id.
        object_id: CausalObjectId,
        /// Object kind.
        object_kind: GraphObjectKind,
    },
    /// A causal graph object has been retired.
    ObjectRetired {
        /// Durable object id.
        object_id: CausalObjectId,
    },
    /// A causal graph edge exists from one entity to another.
    EdgeExists {
        /// Durable edge object id.
        edge_id: CausalObjectId,
        /// Source entity reference.
        from: EntityRef,
        /// Target entity reference.
        to: EntityRef,
        /// Edge kind.
        edge_kind: GraphEdgeKind,
    },
    /// A causal graph edge has been retired.
    EdgeRetired {
        /// Durable edge object id.
        edge_id: CausalObjectId,
    },
    /// A property value is set on an entity.
    PropertySet {
        /// Entity receiving the property.
        entity: EntityRef,
        /// Property key.
        key: String,
        /// Digest-addressed property value.
        value: GraphValueRef,
    },
    /// A property value is cleared from an entity.
    PropertyCleared {
        /// Entity losing the property.
        entity: EntityRef,
        /// Property key.
        key: String,
    },
}

impl GraphFact {
    /// Computes the deterministic fact digest.
    #[must_use]
    pub fn digest(&self) -> FactDigest {
        let bytes = self.build_digest_input_bytes();
        FactDigest(*blake3::hash(&bytes).as_bytes())
    }

    /// Builds the canonical digest input bytes for this graph fact.
    #[must_use]
    pub fn build_digest_input_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(256);
        push_tag(&mut bytes, GRAPH_FACT_DIGEST_DOMAIN);

        match self {
            Self::ObjectExists {
                object_id,
                object_kind,
            } => {
                push_tag(&mut bytes, TAG_FACT_OBJECT_EXISTS);
                push_labeled_str(&mut bytes, "object_id", &object_id.id);
                object_kind.push_digest_input(&mut bytes);
            }
            Self::ObjectRetired { object_id } => {
                push_tag(&mut bytes, TAG_FACT_OBJECT_RETIRED);
                push_labeled_str(&mut bytes, "object_id", &object_id.id);
            }
            Self::EdgeExists {
                edge_id,
                from,
                to,
                edge_kind,
            } => {
                push_tag(&mut bytes, TAG_FACT_EDGE_EXISTS);
                push_labeled_str(&mut bytes, "edge_id", &edge_id.id);
                push_tag(&mut bytes, "edge.from");
                from.push_digest_input(&mut bytes);
                push_tag(&mut bytes, "edge.to");
                to.push_digest_input(&mut bytes);
                edge_kind.push_digest_input(&mut bytes);
            }
            Self::EdgeRetired { edge_id } => {
                push_tag(&mut bytes, TAG_FACT_EDGE_RETIRED);
                push_labeled_str(&mut bytes, "edge_id", &edge_id.id);
            }
            Self::PropertySet { entity, key, value } => {
                push_tag(&mut bytes, TAG_FACT_PROPERTY_SET);
                push_tag(&mut bytes, "property.entity");
                entity.push_digest_input(&mut bytes);
                push_labeled_str(&mut bytes, "property.key", key);
                value.push_digest_input(&mut bytes);
            }
            Self::PropertyCleared { entity, key } => {
                push_tag(&mut bytes, TAG_FACT_PROPERTY_CLEARED);
                push_tag(&mut bytes, "property.entity");
                entity.push_digest_input(&mut bytes);
                push_labeled_str(&mut bytes, "property.key", key);
            }
        }

        bytes
    }
}

/// Deterministic digest of a native Echo graph fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactDigest(pub [u8; 32]);

impl FactDigest {
    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

fn push_tag(bytes: &mut Vec<u8>, tag: &str) {
    push_labeled_bytes(bytes, "tag", tag.as_bytes());
}

fn push_labeled_str(bytes: &mut Vec<u8>, field_tag: &str, value: &str) {
    push_labeled_bytes(bytes, field_tag, value.as_bytes());
}

fn push_labeled_optional_str(bytes: &mut Vec<u8>, field_tag: &str, value: Option<&str>) {
    push_field_tag(bytes, field_tag);
    match value {
        Some(value) => {
            bytes.push(TAG_OPTION_PRESENT);
            push_len_prefixed(bytes, value.as_bytes());
        }
        None => bytes.push(TAG_OPTION_ABSENT),
    }
}

fn push_labeled_bytes(bytes: &mut Vec<u8>, field_tag: &str, value: &[u8]) {
    push_field_tag(bytes, field_tag);
    push_len_prefixed(bytes, value);
}

fn push_field_tag(bytes: &mut Vec<u8>, field_tag: &str) {
    push_len_prefixed(bytes, field_tag.as_bytes());
}

fn push_len_prefixed(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}
