// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent wire fixtures for the private bridge read-set image.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used)]
use warp_core::edict_effects::{EffectReadImageError, EffectReadSetView};

fn row(class: u8, outcome: u8, ty: u8) -> [u8; 192] {
    let mut row = [0; 192];
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
fn image(rows: &[[u8; 192]], pairs: &[[u8; 64]]) -> Vec<u64> {
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
fn invalid(rows: &[[u8; 192]], pairs: &[[u8; 64]]) {
    let words = image(rows, pairs);
    assert!(matches!(
        parse(&words),
        Err(EffectReadImageError::InvalidImage)
    ));
}
#[test]
fn fixed_wire_order_differs_from_native_slot_order_and_rows_borrow_the_input() {
    let mut range = row(3, 2, 0);
    range[184..192].copy_from_slice(&2_u64.to_le_bytes());
    let mut first = [0; 64];
    first[..32].fill(1);
    first[32..].fill(0x41);
    let mut second = [0; 64];
    second[..32].fill(2);
    second[32..].fill(0x42);
    let rows = [row(0, 0, 1), row(1, 0, 1), row(2, 0, 1), range];
    let words = image(&rows, &[first, second]);
    let view = parse(&words).expect("canonical wire order");
    assert_eq!(
        view.rows()
            .iter()
            .map(|row| row.slot.class)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
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
    for index in [1, 2, 3, 7, 72, 105, 111, 144, 176, 184] {
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
    let mut bad = row(3, 2, 0);
    bad[112] = 1;
    invalid(&[bad], &[]);
    let mut bad = row(3, 2, 0);
    bad[144] = 1;
    invalid(&[bad], &[]);
}
#[test]
fn edge_pairs_are_sorted_unique_and_packed_in_range_row_order() {
    let mut a = row(3, 2, 0);
    a[184..192].copy_from_slice(&1_u64.to_le_bytes());
    let mut b = a;
    b[72..104].fill(0x44);
    b[176..184].copy_from_slice(&1_u64.to_le_bytes());
    let mut pair = [0; 64];
    pair[..32].fill(1);
    let words = image(&[a, b], &[pair, pair]);
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
    invalid(&[a, b], &[pair, distinct]);
    let mut two = a;
    two[184..192].copy_from_slice(&2_u64.to_le_bytes());
    invalid(&[two], &[pair, pair]);
    let mut higher = pair;
    higher[..32].fill(2);
    invalid(&[two], &[higher, pair]);
    let mut bad = a;
    bad[176..184].copy_from_slice(&u64::MAX.to_le_bytes());
    invalid(&[bad], &[pair]);
    let mut bad = a;
    bad[184..192].copy_from_slice(&u64::MAX.to_le_bytes());
    invalid(&[bad], &[pair]);
    invalid(&[row(3, 2, 0)], &[pair]);
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
    let mut one = row(3, 2, 0);
    one[184..192].copy_from_slice(&65536_u64.to_le_bytes());
    let allowed = image(&[one], &pairs[..65536]);
    assert_eq!(parse(&allowed).expect("maximum range").pairs().len(), 65536);
    let mut too_many = one;
    too_many[184..192].copy_from_slice(&65537_u64.to_le_bytes());
    invalid(&[too_many], &pairs);
    let mut next = row(3, 2, 0);
    next[72..104].fill(0x44);
    next[176..184].copy_from_slice(&65536_u64.to_le_bytes());
    next[184..192].copy_from_slice(&1_u64.to_le_bytes());
    let total = image(&[one, next], &pairs);
    assert_eq!(
        parse(&total)
            .expect("two individually bounded ranges")
            .pairs()
            .len(),
        65537
    );
}
