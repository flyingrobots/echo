// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Runtime-owned mutation emitter ABI. Emission is not validation or a write.
use crate::edict_pure::{model::RuntimeType, EvaluationError as Error};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MutationKind {
    EnsureNode,
    CreateNode,
    ReplaceAtom,
    EnsureEdge,
    ReplaceEdge,
}
impl MutationKind {
    pub(super) fn from_intrinsic(name: &str) -> Option<Self> {
        match name {
            "echo.dpo@1.ensure-node" => Some(Self::EnsureNode),
            "echo.dpo@1.create-node" => Some(Self::CreateNode),
            "echo.dpo@1.replace-atom" => Some(Self::ReplaceAtom),
            "echo.dpo@1.ensure-edge" => Some(Self::EnsureEdge),
            "echo.dpo@1.replace-edge" => Some(Self::ReplaceEdge),
            _ => None,
        }
    }
    pub(super) fn fields(self) -> &'static [&'static str] {
        match self {
            Self::EnsureNode | Self::CreateNode => {
                &["warpId", "nodeId", "nodeType", "atomType", "atomBytes"]
            }
            Self::ReplaceAtom => &[
                "warpId",
                "nodeId",
                "expectedAtomDigest",
                "atomType",
                "atomBytes",
            ],
            Self::EnsureEdge => &["warpId", "edgeId", "fromId", "toId", "edgeType"],
            Self::ReplaceEdge => &["warpId", "edgeId", "expectedToId", "toId"],
        }
    }
    pub(super) fn validate_type(
        self,
        input: &RuntimeType,
        output: &RuntimeType,
    ) -> Result<(), Error> {
        let RuntimeType::Record(fields) = input.representation() else {
            return Err(Error::InvalidArtifact);
        };
        if fields.len()!=self.fields().len() || !self.fields().iter().all(|name|fields.iter().any(|(key,ty)| key==name && if *name=="atomBytes" {
            matches!(ty.representation(), RuntimeType::Bytes {min:0,max} if *max>0 && *max<=super::EFFECT_READ_STORAGE_CEILING_V1)
        } else {ty.representation()==&RuntimeType::Bytes {min:32,max:32}})) {
            return Err(Error::InvalidArtifact);
        }
        if !matches!(output,RuntimeType::Record(fields) if fields.is_empty()) {
            return Err(Error::UnsupportedProgram);
        }
        Ok(())
    }
}

use crate::edict_pure::{
    evaluate::Meter,
    values::{bytes, exact_fields, field},
};
use crate::{AtomPayload, EdgeKey, Hash, NodeId, NodeKey, TypeId, WarpId};
use echo_edict_canonical::CanonicalValueV1 as Value;
use std::collections::BTreeMap;
/// A compiler-emitted mutation intent, not a validated write or patch.
///
/// Values are inspectable data. No public API accepts caller-constructed values
/// as evaluation evidence or grants mutation authority from this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectMutation {
    /// Create content, or later require the identical complete content identity.
    EnsureNode {
        /// Instance-scoped target.
        node: NodeKey,
        /// Declared node type.
        ty: TypeId,
        /// Both atom type and bytes.
        atom: AtomPayload,
    },
    /// Later require a vacant mutable node/atom target.
    CreateNode {
        /// Instance-scoped target.
        node: NodeKey,
        /// Declared node type.
        ty: TypeId,
        /// Both atom type and bytes.
        atom: AtomPayload,
    },
    /// Later compare the current typed atom digest before replacement.
    ReplaceAtom {
        /// Instance-scoped owner.
        node: NodeKey,
        /// Expected digest of both current atom type and bytes.
        expected: Hash,
        /// Replacement type and bytes.
        atom: AtomPayload,
    },
    /// Create an edge, or later require the identical edge and lawful identity.
    EnsureEdge {
        /// Instance-scoped edge target.
        edge: EdgeKey,
        /// Source local id in the edge's warp.
        from: NodeId,
        /// Target local id in the edge's warp.
        to: NodeId,
        /// Edge type, distinct from node types.
        ty: TypeId,
    },
    /// Later compare an existing edge's target before retargeting it.
    ReplaceEdge {
        /// Instance-scoped edge target.
        edge: EdgeKey,
        /// Expected current target local id.
        expected_to: NodeId,
        /// New target local id.
        to: NodeId,
    },
}
impl MutationKind {
    pub(super) fn emit(self, value: Value, meter: &mut Meter) -> Result<EffectMutation, Error> {
        exact_fields(&value, self.fields())?;
        for name in self.fields() {
            let value = bytes(field(&value, name)?)?;
            if *name == "atomBytes" {
                if value.len() as u64 > super::EFFECT_READ_STORAGE_CEILING_V1 {
                    return Err(Error::InvalidArtifact);
                }
            } else if value.len() != 32 {
                return Err(Error::InvalidArtifact);
            }
        }
        // One deterministic EffectRow reservation; payload ownership is moved
        // from its already charged expression value, never copied here.
        meter.allocate(280)?;
        let Value::Map(fields) = value else {
            return Err(Error::InvalidArtifact);
        };
        let mut fields = fields
            .into_iter()
            .map(|(key, value)| match key {
                Value::Text(key) => Ok((key, value)),
                _ => Err(Error::InvalidArtifact),
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let warp_id = WarpId(id(&mut fields, "warpId")?);
        Ok(match self {
            Self::EnsureNode | Self::CreateNode => {
                let node = NodeKey {
                    warp_id,
                    local_id: NodeId(id(&mut fields, "nodeId")?),
                };
                let ty = TypeId(id(&mut fields, "nodeType")?);
                let atom = atom(&mut fields)?;
                if self == Self::EnsureNode {
                    EffectMutation::EnsureNode { node, ty, atom }
                } else {
                    EffectMutation::CreateNode { node, ty, atom }
                }
            }
            Self::ReplaceAtom => {
                let node = NodeKey {
                    warp_id,
                    local_id: NodeId(id(&mut fields, "nodeId")?),
                };
                let expected = id(&mut fields, "expectedAtomDigest")?;
                EffectMutation::ReplaceAtom {
                    node,
                    expected,
                    atom: atom(&mut fields)?,
                }
            }
            Self::EnsureEdge => {
                let edge = EdgeKey {
                    warp_id,
                    local_id: crate::EdgeId(id(&mut fields, "edgeId")?),
                };
                EffectMutation::EnsureEdge {
                    edge,
                    from: NodeId(id(&mut fields, "fromId")?),
                    to: NodeId(id(&mut fields, "toId")?),
                    ty: TypeId(id(&mut fields, "edgeType")?),
                }
            }
            Self::ReplaceEdge => {
                let edge = EdgeKey {
                    warp_id,
                    local_id: crate::EdgeId(id(&mut fields, "edgeId")?),
                };
                EffectMutation::ReplaceEdge {
                    edge,
                    expected_to: NodeId(id(&mut fields, "expectedToId")?),
                    to: NodeId(id(&mut fields, "toId")?),
                }
            }
        })
    }
}
fn take(fields: &mut BTreeMap<String, Value>, name: &str) -> Result<Vec<u8>, Error> {
    match fields.remove(name) {
        Some(Value::Bytes(value)) => Ok(value),
        _ => Err(Error::InvalidArtifact),
    }
}
fn id(fields: &mut BTreeMap<String, Value>, name: &str) -> Result<[u8; 32], Error> {
    take(fields, name)?
        .try_into()
        .map_err(|_| Error::InvalidArtifact)
}
fn atom(fields: &mut BTreeMap<String, Value>) -> Result<AtomPayload, Error> {
    Ok(AtomPayload {
        type_id: TypeId(id(fields, "atomType")?),
        bytes: take(fields, "atomBytes")?.into(),
    })
}
