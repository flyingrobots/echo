// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent wire fixtures for the private bridge read-set image.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used)]
use warp_core::edict_effects::{EffectReadImageError, EffectReadSetView};

const ROW: usize = 200;

fn row(class: u8, outcome: u8, ty: u8) -> [u8; ROW] {
    let mut row = [0; ROW];
    row[0] = class;
    row[104] = outcome;
    row[8..40].fill(0x11);
    row[40..72].fill(0x22);
    row[112..144].fill(ty);
    if outcome == 0 || outcome == 3 {
        row[144..176].fill(0x55);
    }
    if class == 3 {
        row[72..104].fill(0x33);
    }
    row
}
/// The present node row of the fixtures' range source, carrying the size of
/// the bucket its ranges scanned.
fn source(bucket_len: u64) -> [u8; ROW] {
    let mut row = row(0, 0, 1);
    row[105] = 1;
    row[192..200].copy_from_slice(&bucket_len.to_le_bytes());
    row
}
/// A range row from the fixtures' source retaining `pairs` pairs from `start`.
fn range(start: u64, pairs: u64) -> [u8; ROW] {
    let mut row = row(3, 2, 0);
    row[176..184].copy_from_slice(&start.to_le_bytes());
    row[184..192].copy_from_slice(&pairs.to_le_bytes());
    row
}
fn image(rows: &[[u8; ROW]], pairs: &[[u8; 64]]) -> Vec<u64> {
    let mut bytes = (rows.len() as u64).to_le_bytes().to_vec();
    for row in rows {
        bytes.extend_from_slice(row);
    }
    bytes.extend_from_slice(&(pairs.len() as u64).to_le_bytes());
    for pair in pairs {
        bytes.extend_from_slice(pair);
    }
    bytes
        .chunks_exact(8)
        .map(|part| u64::from_ne_bytes(part.try_into().expect("eight bytes")))
        .collect()
}
fn parse(words: &[u64]) -> Result<EffectReadSetView<'_>, EffectReadImageError> {
    EffectReadSetView::parse(bytemuck::cast_slice(words))
}
fn invalid(rows: &[[u8; ROW]], pairs: &[[u8; 64]]) {
    let words = image(rows, pairs);
    assert!(matches!(
        parse(&words),
        Err(EffectReadImageError::InvalidImage)
    ));
}
fn valid(rows: &[[u8; ROW]], pairs: &[[u8; 64]]) {
    let words = image(rows, pairs);
    assert_eq!(
        parse(&words).map(|view| view.rows().len()),
        Ok(rows.len()),
        "a canonical image"
    );
}
#[test]
fn fixed_wire_order_differs_from_native_slot_order_and_rows_borrow_the_input() {
    let mut first = [0; 64];
    first[..32].fill(1);
    first[32..].fill(0x41);
    let mut second = [0; 64];
    second[..32].fill(2);
    second[32..].fill(0x42);
    let rows = [source(2), row(1, 0, 1), row(2, 0, 1), range(0, 2)];
    let words = image(&rows, &[first, second]);
    let view = parse(&words).expect("canonical wire order");
    assert_eq!(
        view.rows()
            .iter()
            .map(|row| row.slot.class)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert_eq!(view.rows()[0].bucket_len(), 2);
    assert_eq!(view.pairs()[0].edge_local_id, [1; 32]);
    assert_eq!(view.pairs()[1].target_local_id, [0x42; 32]);
    assert_eq!(
        view.rows().as_ptr().cast::<u8>(),
        bytemuck::cast_slice::<u64, u8>(&words)[8..].as_ptr()
    );
    invalid(&[rows[0], rows[2], rows[1], rows[3]], &[first, second]);
}
#[test]
fn only_distinct_absence_types_may_repeat_a_slot() {
    let a = row(0, 1, 1);
    let b = row(0, 1, 2);
    let words = image(&[a, b], &[]);
    assert_eq!(parse(&words).expect("two absent types").rows().len(), 2);
    invalid(&[a, a], &[]);
    invalid(&[b, a], &[]);
    invalid(&[row(0, 0, 1), row(0, 0, 2)], &[]);
    invalid(&[row(0, 0, 1), b], &[]);
}
#[test]
fn tags_padding_and_unused_fields_have_one_encoding() {
    let base = row(0, 1, 1);
    for index in [1, 2, 3, 7, 72, 105, 106, 111, 144, 176, 184, 192, 199] {
        let mut bad = base;
        bad[index] = 1;
        invalid(&[bad], &[]);
    }
    let mut bad = base;
    bad[0] = 4;
    invalid(&[bad], &[]);
    let mut bad = base;
    bad[104] = 4;
    invalid(&[bad], &[]);
    invalid(&[row(0, 3, 1)], &[]);
    let mut bad = row(1, 3, 1);
    bad[1] = 1;
    bad[2] = 1;
    invalid(&[bad], &[]);
    valid(&[source(0), range(0, 0)], &[]);
    let mut bad = range(0, 0);
    bad[112] = 1;
    invalid(&[source(0), bad], &[]);
    let mut bad = range(0, 0);
    bad[144] = 1;
    invalid(&[source(0), bad], &[]);
}
#[test]
fn edge_pairs_are_sorted_unique_and_packed_in_range_row_order() {
    let a = range(0, 1);
    let mut b = range(1, 1);
    b[72..104].fill(0x44);
    let mut pair = [0; 64];
    pair[..32].fill(1);
    let words = image(&[source(1), a, b], &[pair, pair]);
    assert_eq!(
        parse(&words)
            .expect("same edge ID in separate ranges")
            .pairs()
            .len(),
        2
    );
    b[176..184].fill(0);
    let mut distinct = pair;
    distinct[..32].fill(2);
    invalid(&[source(1), a, b], &[pair, distinct]);
    let two = range(0, 2);
    invalid(&[source(2), two], &[pair, pair]);
    let mut higher = pair;
    higher[..32].fill(2);
    invalid(&[source(2), two], &[higher, pair]);
    invalid(&[source(1), range(u64::MAX, 1)], &[pair]);
    invalid(&[source(u64::MAX), range(0, u64::MAX)], &[pair]);
    invalid(&[source(1), range(0, 0)], &[pair]);
}
/// Oracle: the read-set wire contract. A range scans its source's whole
/// bucket, so a range's retained pairs never outnumber that bucket, and a
/// range read from a present source requires that source's witnessed bucket.
#[test]
fn a_range_requires_its_source_row_and_cannot_outnumber_its_bucket() {
    let mut pair = [0; 64];
    pair[..32].fill(1);
    let mut next = pair;
    next[..32].fill(2);
    valid(&[source(2), range(0, 2)], &[pair, next]);
    valid(&[source(5), range(0, 2)], &[pair, next]);
    invalid(&[source(1), range(0, 2)], &[pair, next]);
    invalid(&[range(0, 0)], &[]);
    invalid(&[row(0, 0, 1), range(0, 0)], &[]);
    valid(&[row(0, 1, 1), range(0, 0)], &[]);
    invalid(&[row(0, 1, 1), range(0, 1)], &[pair]);
    let mut other = source(1);
    other[40..72].fill(0x44);
    invalid(&[other, range(0, 1)], &[pair]);
}
/// Oracle: the read-set wire contract. Only a present node row has a bucket
/// to witness. A witnessed bucket with no range row is the bucket that
/// decided a refused scan, and a witnessed bucket may be empty.
#[test]
fn only_a_present_node_row_witnesses_a_bucket() {
    valid(&[source(0)], &[]);
    valid(&[source(65)], &[]);
    for (class, outcome) in [(0, 1), (1, 0), (1, 3), (2, 0)] {
        let mut bad = row(class, outcome, 1);
        bad[105] = 1;
        invalid(&[bad], &[]);
    }
    let mut bad = range(0, 0);
    bad[105] = 1;
    invalid(&[source(0), bad], &[]);
    let mut bad = row(0, 0, 1);
    bad[192] = 1;
    invalid(&[bad], &[]);
    let mut bad = source(0);
    bad[105] = 2;
    invalid(&[bad], &[]);
}
/// The superseded 192-byte row layout is refused, not read with new meaning:
/// a one-row image in that layout, byte for byte, has no valid framing.
#[test]
fn an_image_in_the_superseded_row_layout_is_refused() {
    let mut old = 1_u64.to_le_bytes().to_vec();
    old.extend_from_slice(&row(0, 0, 1)[..192]);
    old.extend_from_slice(&0_u64.to_le_bytes());
    assert_eq!(old.len(), 208);
    let words: Vec<u64> = old
        .chunks_exact(8)
        .map(|part| u64::from_ne_bytes(part.try_into().expect("eight bytes")))
        .collect();
    assert!(matches!(
        parse(&words),
        Err(EffectReadImageError::InvalidImage)
    ));
}
#[test]
fn counts_alignment_and_tail_are_validated_before_any_row_view() {
    let empty = image(&[], &[]);
    assert!(parse(&empty).expect("empty set").rows().is_empty());
    let mut tail = empty.clone();
    tail.push(0);
    assert!(matches!(
        parse(&tail),
        Err(EffectReadImageError::InvalidImage)
    ));
    for count in [4097, u64::MAX] {
        let mut bad = empty.clone();
        bad[0] = count.to_le();
        assert!(matches!(
            parse(&bad),
            Err(EffectReadImageError::InvalidImage)
        ));
    }
    for len in 0..16 {
        assert!(matches!(
            EffectReadSetView::parse(&bytemuck::cast_slice::<u64, u8>(&empty)[..len]),
            Err(EffectReadImageError::InvalidImage)
        ));
    }
    let padded = [0_u64; 3];
    assert!(matches!(
        EffectReadSetView::parse(&bytemuck::cast_slice::<u64, u8>(&padded)[1..17]),
        Err(EffectReadImageError::UnalignedImage)
    ));
}

#[test]
fn row_limit_applies_to_complete_otherwise_canonical_images() {
    let rows = (0_u32..4097)
        .map(|id| {
            let mut r = row(0, 1, 0);
            r[140..144].copy_from_slice(&id.to_be_bytes());
            r
        })
        .collect::<Vec<_>>();
    let allowed = image(&rows[..4096], &[]);
    assert_eq!(
        parse(&allowed)
            .expect("full allowed row count")
            .rows()
            .len(),
        4096
    );
    invalid(&rows, &[]);
}
#[test]
fn pair_limit_is_per_range_and_does_not_shrink_the_total_pair_section() {
    let pairs = (0_u32..65537)
        .map(|id| {
            let mut pair = [0; 64];
            pair[28..32].copy_from_slice(&id.to_be_bytes());
            pair
        })
        .collect::<Vec<_>>();
    let one = range(0, 65536);
    let allowed = image(&[source(65536), one], &pairs[..65536]);
    assert_eq!(parse(&allowed).expect("maximum range").pairs().len(), 65536);
    invalid(&[source(65537), range(0, 65537)], &pairs);
    let mut next = range(65536, 1);
    next[72..104].fill(0x44);
    let total = image(&[source(65536), one, next], &pairs);
    assert_eq!(
        parse(&total)
            .expect("two individually bounded ranges")
            .pairs()
            .len(),
        65537
    );
}
