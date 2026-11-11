// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
use super::{EffectEdgePairRow, EffectReadImageError as Error, EffectReadRow};

/// Allocation-free borrowed view after canonical structural validation.
///
/// Validation proves framing and canonical form, not provenance, type-law
/// authorization, dependency completeness or a historical Action decision.
#[derive(Debug, Clone, Copy)]
pub struct EffectReadSetView<'a> {
    bytes: &'a [u8],
    rows: &'a [EffectReadRow],
    pairs: &'a [EffectEdgePairRow],
}
impl<'a> EffectReadSetView<'a> {
    /// Checks counts, exact sizes, alignment, tags, unused fields, sorting and packing.
    ///
    /// It also checks each range against its source's node row: a present
    /// source must witness its bucket with at least the range's pair count,
    /// and an absent source yields no pairs. Input must be eight-byte aligned.
    /// Validation walks borrowed rows without allocating, copying the image,
    /// or hydrating owned collections; each range adds one binary search.
    /// # Errors
    /// Refuses invalid structure, more than 4096 rows, ranges above 65536 pairs,
    /// a range its source row contradicts, an image above the fixed
    /// read-storage ceiling, or an unaligned buffer.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < 16
            || u64::try_from(bytes.len()).map_err(|_| Error::InvalidImage)?
                > super::super::EFFECT_READ_STORAGE_CEILING_V1
        {
            return Err(Error::InvalidImage);
        }
        if bytes.as_ptr().align_offset(8) != 0 {
            return Err(Error::UnalignedImage);
        }
        let count = word(&bytes[..8])?;
        if count > 4096 {
            return Err(Error::InvalidImage);
        }
        let row_end = usize::try_from(count)
            .ok()
            .and_then(|n| n.checked_mul(200))
            .and_then(|n| n.checked_add(8))
            .ok_or(Error::InvalidImage)?;
        let pair_start = row_end
            .checked_add(8)
            .filter(|end| *end <= bytes.len())
            .ok_or(Error::InvalidImage)?;
        let pair_count = word(&bytes[row_end..pair_start])?;
        let end = usize::try_from(pair_count)
            .ok()
            .and_then(|n| n.checked_mul(64))
            .and_then(|n| n.checked_add(pair_start))
            .ok_or(Error::InvalidImage)?;
        if end != bytes.len() {
            return Err(Error::InvalidImage);
        }
        let rows: &[EffectReadRow] =
            bytemuck::try_cast_slice(&bytes[8..row_end]).map_err(|_| Error::UnalignedImage)?;
        let pairs: &[EffectEdgePairRow] =
            bytemuck::try_cast_slice(&bytes[pair_start..]).map_err(|_| Error::UnalignedImage)?;
        validate_rows(rows, pairs)?;
        Ok(Self { bytes, rows, pairs })
    }
    /// Rows borrowed directly from the validated image.
    #[must_use]
    pub const fn rows(&self) -> &'a [EffectReadRow] {
        self.rows
    }
    /// Packed pairs borrowed directly from the validated image.
    #[must_use]
    pub const fn pairs(&self) -> &'a [EffectEdgePairRow] {
        self.pairs
    }
    /// Confirms every witnessed bucket size against the state at a basis.
    ///
    /// A bucket's size decides a scan's bound and charge, so a replayed or
    /// audited read can confirm what it was charged for only if each recorded
    /// size equals the source's whole outgoing bucket in `state`. The caller
    /// supplies the state at the image's evaluation basis; this checks bucket
    /// sizes only, not values, provenance or the basis identity. It walks the
    /// borrowed rows once, does one constant-time bucket lookup per witnessed
    /// bucket and allocates nothing.
    ///
    /// # Errors
    /// [`Error::BucketSizeMismatch`] when a recorded size differs from the
    /// bucket in `state`, including when the source node is absent there.
    pub fn confirm_bucket_sizes(&self, state: &crate::WorldlineState) -> Result<(), Error> {
        for row in self.rows.iter().filter(|row| row.bucket == 1) {
            let local = crate::NodeId(row.slot.local_id);
            let actual = state
                .store(&crate::WarpId(row.slot.warp_id))
                .filter(|store| store.node(&local).is_some())
                .and_then(|store| u64::try_from(store.edges_from(&local).len()).ok());
            if actual != Some(row.bucket_len()) {
                return Err(Error::BucketSizeMismatch);
            }
        }
        Ok(())
    }
    /// Domain-separated digest of these exact validated bytes.
    #[must_use]
    pub fn digest(&self) -> crate::Hash {
        super::digest(self.bytes)
    }
}
fn word(bytes: &[u8]) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(
        bytes.try_into().map_err(|_| Error::InvalidImage)?,
    ))
}
fn validate_rows(rows: &[EffectReadRow], pairs: &[EffectEdgePairRow]) -> Result<(), Error> {
    let mut previous: Option<&EffectReadRow> = None;
    let mut cursor = 0_usize;
    for (index, row) in rows.iter().enumerate() {
        if !row.valid() {
            return Err(Error::InvalidImage);
        }
        if let Some(previous) = previous {
            if previous.compare(row) != std::cmp::Ordering::Less
                || (previous.slot == row.slot && (previous.outcome != 1 || row.outcome != 1))
            {
                return Err(Error::InvalidImage);
            }
        }
        if row.outcome == 2 {
            let start = usize::try_from(row.pairs.start()).map_err(|_| Error::InvalidImage)?;
            let len = usize::try_from(row.pairs.len()).map_err(|_| Error::InvalidImage)?;
            if start != cursor || len > 65536 {
                return Err(Error::InvalidImage);
            }
            cursor = cursor
                .checked_add(len)
                .filter(|end| *end <= pairs.len())
                .ok_or(Error::InvalidImage)?;
            if pairs[start..cursor]
                .windows(2)
                .any(|pair| pair[0].edge_local_id >= pair[1].edge_local_id)
            {
                return Err(Error::InvalidImage);
            }
            // Rows before this one are already checked and sorted, so its
            // source's node rows, which sort first, lie among them.
            source_bucket_holds(&rows[..index], row, len)?;
        }
        previous = Some(row);
    }
    if cursor != pairs.len() {
        return Err(Error::InvalidImage);
    }
    Ok(())
}
/// Checks a range row against its source's node row among the sorted `before`.
///
/// A range scans its source's whole bucket, so a present source must witness
/// that bucket and hold at least the range's `pairs`; an absent source has no
/// bucket and yields no pairs. A range without its source's row is refused.
fn source_bucket_holds(
    before: &[EffectReadRow],
    range: &EffectReadRow,
    pairs: usize,
) -> Result<(), Error> {
    let key = |row: &EffectReadRow| (row.slot.class, row.slot.warp_id, row.slot.local_id);
    let source = (0, range.slot.warp_id, range.slot.local_id);
    // One present row, or one or more absent rows sorted by named type.
    let first = before
        .get(before.partition_point(|row| key(row) < source))
        .filter(|row| key(row) == source)
        .ok_or(Error::InvalidImage)?;
    let pairs = u64::try_from(pairs).map_err(|_| Error::InvalidImage)?;
    let holds = match first.outcome {
        0 => first.bucket == 1 && pairs <= first.bucket_len(),
        _ => pairs == 0,
    };
    if holds {
        Ok(())
    } else {
        Err(Error::InvalidImage)
    }
}
