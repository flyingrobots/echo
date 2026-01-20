// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use warp_core::math::Prng;

#[test]
fn next_int_golden_regression() {
    let mut prng = Prng::from_seed(0xDEAD_BEEF, 0xFACE_FEED);
    let values: Vec<i32> = (0..3).map(|_| prng.next_int(i32::MIN, i32::MAX)).collect();
    assert_eq!(values, vec![1_501_347_292, 1_946_982_111, -117_316_573]);
}
