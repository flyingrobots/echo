// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! The same bounded-read interpreter can use plain or runtime-recording access.
use super::{ReadBasis, ReadError, ReadObstruction, ReadView};
use crate::edict_pure::{evaluate::Meter, EvaluationError};
use crate::{NodeKey, TypeId};

pub(crate) trait NodeAtomAccess {
    type Error: From<ReadError> + From<EvaluationError>;
    fn basis(&self) -> ReadBasis;
    fn contains(&self, node: NodeKey) -> bool;
    fn atom(
        &mut self,
        node: NodeKey,
        expected: TypeId,
        max_bytes: u64,
        meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, Self::Error>;
}

impl NodeAtomAccess for &ReadView<'_> {
    type Error = ReadError;
    fn basis(&self) -> ReadBasis {
        ReadView::basis(self)
    }
    fn contains(&self, node: NodeKey) -> bool {
        ReadView::contains(self, node)
    }
    fn atom(
        &mut self,
        node: NodeKey,
        expected: TypeId,
        max_bytes: u64,
        _meter: &mut Meter,
    ) -> Result<Result<&[u8], ReadObstruction>, Self::Error> {
        Ok(ReadView::atom(self, node, expected, max_bytes))
    }
}
