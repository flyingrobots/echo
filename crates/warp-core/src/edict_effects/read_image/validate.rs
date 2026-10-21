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
    /// Input must be eight-byte aligned. Validation walks borrowed rows without
    /// allocating, copying the image, or hydrating owned collections.
    /// # Errors
    /// Refuses invalid structure, more than 4096 rows, ranges above 65536 pairs,
    /// an image above the fixed read-storage ceiling, or an unaligned buffer.
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
            .and_then(|n| n.checked_mul(192))
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
    for row in rows {
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
        }
        previous = Some(row);
    }
    if cursor != pairs.len() {
        return Err(Error::InvalidImage);
    }
    Ok(())
}
