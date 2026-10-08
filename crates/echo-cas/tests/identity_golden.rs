// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Raw Echo identity vectors retained from PR #723 at db3e5dfe.
//! Original fixture provenance names b3sum 1.8.5. This test does not execute Keep.

use echo_cas::blob_hash;

#[test]
fn raw_echo_identity_matches_retained_independent_vectors() {
    let ramp = (u8::MIN..=u8::MAX).collect::<Vec<_>>();
    for (bytes, expected) in [
        (
            &[][..],
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        ),
        (
            b"Keep exact bytes.\n".as_slice(),
            "3d4f69a42f4cde6629970fe61d548f60d475a0f9ac26a24be5fab204b6eb030a",
        ),
        (
            ramp.as_slice(),
            "4a495ba42461748eca8fdad618f976aa726cc2903de9fcb40735a786ac1c196b",
        ),
    ] {
        assert_eq!(blob_hash(bytes).to_string(), expected);
    }
}
