// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Attachment-plane payloads and codec boundaries.
//!
//! Echo models Paper I/II “two-plane” WARP semantics:
//! - The **skeleton plane** is the explicit graph structure (nodes + edges +
//!   boundary ports). This is what matching, scheduling, rewriting, hashing,
//!   and slicing operate on.
//! - The **attachment plane** carries payloads attached to skeleton vertices
//!   and edges.
//!
//! Stage B0 (this module) implements **depth-0 attachments** only: typed atoms.
//! In Paper I terms, these are `Atom(p)` payloads for some opaque `p`.
//! In Echo, `p` is represented as the pair `(TypeId, Bytes)` to avoid “same
//! bytes, different meaning” collisions at the deterministic boundary.
//!
//! Stage B1 extends this with **descended attachments** via flattened
//! indirection:
//! - Attachments remain opaque atoms by default (`AtomPayload`).
//! - Attachments may also be `Descend(WarpId)` to refer to another instance.
//! - Descend links are explicit (not encoded inside bytes), satisfying the
//!   “no hidden edges” law.

use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;

use bytes::Bytes;
use thiserror::Error;

use crate::ident::{EdgeKey, NodeKey, TypeId, WarpId};

/// Attachment plane selector.
///
/// In Paper I notation, vertex attachments are `α` and edge attachments are `β`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttachmentPlane {
    /// Vertex/node attachment plane (`α`).
    Alpha,
    /// Edge attachment plane (`β`).
    Beta,
}

impl AttachmentPlane {
    const fn tag(self) -> u8 {
        match self {
            Self::Alpha => 1,
            Self::Beta => 2,
        }
    }
}

/// Owner identity for an attachment slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttachmentOwner {
    /// Attachment owned by a node.
    Node(NodeKey),
    /// Attachment owned by an edge.
    Edge(EdgeKey),
}

impl AttachmentOwner {
    const fn tag(self) -> u8 {
        match self {
            Self::Node(_) => 1,
            Self::Edge(_) => 2,
        }
    }

    /// Returns the [`WarpId`] of the owner (node or edge).
    pub(crate) fn warp_id(self) -> WarpId {
        match self {
            Self::Node(nk) => nk.warp_id,
            Self::Edge(ek) => ek.warp_id,
        }
    }
}

/// First-class identity for an attachment slot.
///
/// This is the key used for Stage B1 “descent chain” footprinting and slicing:
/// changes to an attachment slot (especially `Descend`) must invalidate matches
/// inside descendant instances deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AttachmentKey {
    /// Owner of the slot.
    pub owner: AttachmentOwner,
    /// Attachment plane selector.
    pub plane: AttachmentPlane,
}

impl AttachmentKey {
    /// Constructs a node-owned attachment key in the vertex/α plane.
    #[must_use]
    pub const fn node_alpha(node: NodeKey) -> Self {
        Self {
            owner: AttachmentOwner::Node(node),
            plane: AttachmentPlane::Alpha,
        }
    }

    /// Constructs an edge-owned attachment key in the edge/β plane.
    #[must_use]
    pub const fn edge_beta(edge: EdgeKey) -> Self {
        Self {
            owner: AttachmentOwner::Edge(edge),
            plane: AttachmentPlane::Beta,
        }
    }

    pub(crate) const fn tag(self) -> (u8, u8) {
        (self.owner.tag(), self.plane.tag())
    }

    /// Returns `true` if the plane is valid for the owner type.
    ///
    /// - Node owners require `AttachmentPlane::Alpha`
    /// - Edge owners require `AttachmentPlane::Beta`
    #[must_use]
    pub fn is_plane_valid(&self) -> bool {
        matches!(
            (&self.owner, &self.plane),
            (AttachmentOwner::Node(_), AttachmentPlane::Alpha)
                | (AttachmentOwner::Edge(_), AttachmentPlane::Beta)
        )
    }
}

/// Attachment value stored in the attachment plane.
///
/// Depth-0 attachments are always [`AttachmentValue::Atom`].
/// Stage B1 introduces [`AttachmentValue::Descend`] to model recursive WARPs as
/// flattened indirection.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AttachmentValue {
    /// Depth-0 atom payload.
    Atom(AtomPayload),
    /// Flattened indirection to another WARP instance.
    Descend(WarpId),
}

/// Typed, opaque payload attached to a node or edge.
///
/// This is the depth-0 “atom” payload for the attachment plane:
/// `AtomPayload = Atom(TypeId, Bytes)`.
///
/// Laws / invariants:
/// - `type_id` is part of the deterministic boundary and must participate in
///   canonical encodings and digests.
/// - `bytes` are opaque to the core store and must not be treated as hidden
///   skeleton structure. Any dependency that matters for matching, causality,
///   slicing, or rewrite applicability must be expressed as explicit skeleton
///   nodes/edges/ports.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AtomPayload {
    /// Type identifier describing how to interpret `bytes`.
    pub type_id: TypeId,
    /// Opaque payload bytes.
    pub bytes: Bytes,
}

impl AtomPayload {
    /// Constructs a new typed atom payload.
    #[must_use]
    pub fn new(type_id: TypeId, bytes: Bytes) -> Self {
        Self { type_id, bytes }
    }

    /// Attempts to decode the payload as `T` using codec `C`.
    ///
    /// # Errors
    /// Returns [`DecodeError::TypeMismatch`] when the `type_id` does not match
    /// `C::TYPE_ID`, or forwards the codec's strict decode error.
    ///
    /// # Determinism
    /// This helper is deterministic: it has no side effects and depends only on
    /// the payload bytes and `type_id`.
    pub fn decode_with<C, T>(&self) -> Result<T, DecodeError>
    where
        C: Codec<T>,
    {
        if self.type_id != C::TYPE_ID {
            return Err(DecodeError::TypeMismatch {
                expected: C::TYPE_ID,
                found: self.type_id,
            });
        }
        C::decode_strict(&self.bytes)
    }

    /// Attempts to decode the payload for use in a rule matcher.
    ///
    /// This encodes Echo’s v0 decode failure semantics:
    /// - type mismatch or decode failure ⇒ “rule does not apply” (`None`)
    ///
    /// Use this helper in matchers to ensure that decode failures never trigger
    /// partial effects; if a matcher returns `true`, the executor should be able
    /// to decode the same payload deterministically.
    #[must_use]
    pub fn decode_for_match<C, T>(&self) -> Option<T>
    where
        C: Codec<T>,
    {
        self.decode_with::<C, T>().ok()
    }
}

/// Error returned by strict payload decoding.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DecodeError {
    /// The payload `type_id` did not match the codec's expected type id.
    #[error("payload type mismatch: expected {expected:?}, found {found:?}")]
    TypeMismatch {
        /// Expected type identifier.
        expected: TypeId,
        /// Found type identifier.
        found: TypeId,
    },
    /// No codec was registered for the payload `TypeId`.
    #[error("no codec registered for payload type id: {0:?}")]
    UnknownTypeId(TypeId),
    /// The byte content was invalid for the expected type.
    #[error("invalid payload bytes")]
    InvalidBytes,
}

/// Canonical codec for a typed payload `T`.
///
/// This trait defines the boundary between opaque attachment bytes and typed
/// application data. The core rewrite engine does not depend on decoding; rules
/// and views may decode payloads explicitly when needed.
///
/// Contract:
/// - `encode_canon` must produce a stable, canonical byte representation.
/// - `decode_strict` must either return the unique decoded `T` or a deterministic
///   [`DecodeError`].
pub trait Codec<T> {
    /// Type id committed into the attachment atom.
    const TYPE_ID: TypeId;

    /// Encodes `value` into a canonical byte representation.
    fn encode_canon(value: &T) -> Bytes;

    /// Decodes bytes strictly into `T`.
    ///
    /// Implementations must be deterministic and must not consult ambient state
    /// (time, randomness, global mutable config, etc).
    ///
    /// # Errors
    /// Returns an error if `bytes` is not a valid canonical encoding of `T`.
    fn decode_strict(bytes: &Bytes) -> Result<T, DecodeError>;
}

/// Object-safe codec wrapper for dynamic decoding based on `TypeId`.
///
/// This exists primarily for tooling/view layers that need to decode arbitrary
/// payloads at runtime (e.g., inspectors). The core rewrite engine does not use
/// this registry on hot paths.
pub trait ErasedCodec: Send + Sync {
    /// Type id handled by this codec.
    fn type_id(&self) -> TypeId;
    /// Human-readable type name for debugging and tooling.
    fn type_name(&self) -> &'static str;
    /// Strictly decode bytes into a type-erased value.
    ///
    /// # Errors
    /// Returns an error if `bytes` is not a valid canonical encoding for this
    /// codec's type.
    fn decode_any(&self, bytes: &Bytes) -> Result<Box<dyn Any>, DecodeError>;
}

struct ErasedCodecImpl<T, C> {
    type_name: &'static str,
    _marker: PhantomData<(T, C)>,
}

impl<T, C> ErasedCodecImpl<T, C> {
    const fn new(type_name: &'static str) -> Self {
        Self {
            type_name,
            _marker: PhantomData,
        }
    }
}

impl<T, C> ErasedCodec for ErasedCodecImpl<T, C>
where
    T: Any + Send + Sync,
    C: Codec<T> + Send + Sync,
{
    fn type_id(&self) -> TypeId {
        C::TYPE_ID
    }

    fn type_name(&self) -> &'static str {
        self.type_name
    }

    fn decode_any(&self, bytes: &Bytes) -> Result<Box<dyn Any>, DecodeError> {
        let value = C::decode_strict(bytes)?;
        Ok(Box::new(value))
    }
}

/// Minimal codec registry keyed by payload `TypeId`.
///
/// The registry supports dynamic decode for tooling layers. It is intentionally
/// small and is not used by the core scheduler/matcher unless an application
/// layer explicitly consults it.
#[derive(Default)]
pub struct CodecRegistry {
    codecs: HashMap<TypeId, Box<dyn ErasedCodec>>,
}

/// Errors returned when registering codecs.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Attempted to register two codecs for the same `TypeId`.
    #[error("duplicate codec registration for type id: {0:?}")]
    DuplicateTypeId(TypeId),
}

impl CodecRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers codec `C` for payload type `T`.
    ///
    /// `type_name` is a human-readable identifier for debugging/tooling.
    ///
    /// # Errors
    /// Returns [`RegistryError::DuplicateTypeId`] if a codec is already registered
    /// for the same `TypeId`.
    pub fn register<T, C>(&mut self, type_name: &'static str) -> Result<(), RegistryError>
    where
        T: Any + Send + Sync,
        C: Codec<T> + Send + Sync + 'static,
    {
        let type_id = C::TYPE_ID;
        if self.codecs.contains_key(&type_id) {
            return Err(RegistryError::DuplicateTypeId(type_id));
        }
        self.codecs
            .insert(type_id, Box::new(ErasedCodecImpl::<T, C>::new(type_name)));
        Ok(())
    }

    /// Returns the codec registered for `type_id` (if any).
    #[must_use]
    pub fn get(&self, type_id: &TypeId) -> Option<&dyn ErasedCodec> {
        self.codecs.get(type_id).map(std::convert::AsRef::as_ref)
    }

    /// Decodes a typed atom payload using the codec registered for its `type_id`.
    ///
    /// # Errors
    /// Returns [`DecodeError::TypeMismatch`] if the payload `type_id` is not
    /// registered.
    pub fn decode_atom(&self, payload: &AtomPayload) -> Result<Box<dyn Any>, DecodeError> {
        let Some(codec) = self.get(&payload.type_id) else {
            return Err(DecodeError::UnknownTypeId(payload.type_id));
        };
        codec.decode_any(&payload.bytes)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    struct U8Codec;

    impl Codec<u8> for U8Codec {
        const TYPE_ID: TypeId = TypeId([0xA5; 32]);

        fn encode_canon(value: &u8) -> Bytes {
            Bytes::from(vec![*value])
        }

        fn decode_strict(bytes: &Bytes) -> Result<u8, DecodeError> {
            if bytes.len() != 1 {
                return Err(DecodeError::InvalidBytes);
            }
            Ok(*bytes.first().ok_or(DecodeError::InvalidBytes)?)
        }
    }

    #[test]
    fn atom_decode_with_enforces_type_id() {
        let payload = AtomPayload::new(U8Codec::TYPE_ID, U8Codec::encode_canon(&7u8));
        assert_eq!(payload.decode_with::<U8Codec, u8>().unwrap(), 7u8);

        let wrong = AtomPayload::new(TypeId([0x5A; 32]), U8Codec::encode_canon(&7u8));
        assert!(
            matches!(
                wrong.decode_with::<U8Codec, u8>(),
                Err(DecodeError::TypeMismatch { .. })
            ),
            "type mismatch must be reported deterministically"
        );
    }

    #[test]
    fn atom_decode_for_match_is_deterministic_and_conservative() {
        // v0 semantics: any decode failure -> rule does not apply (None).
        let good = AtomPayload::new(U8Codec::TYPE_ID, U8Codec::encode_canon(&9u8));
        assert_eq!(good.decode_for_match::<U8Codec, u8>(), Some(9u8));

        let wrong_type = AtomPayload::new(TypeId([0x00; 32]), U8Codec::encode_canon(&9u8));
        assert_eq!(wrong_type.decode_for_match::<U8Codec, u8>(), None);

        let bad_bytes = AtomPayload::new(U8Codec::TYPE_ID, Bytes::from_static(&[]));
        assert_eq!(bad_bytes.decode_for_match::<U8Codec, u8>(), None);
    }
}
