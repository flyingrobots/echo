// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! High-integrity protocol drills for PR 1.

#[cfg(test)]
mod tests {
    use crate::eint_v2::*;
    use crate::ttdr_v2::*;
    use blake3::hash;

    /// DRILL: Header Integrity
    ///
    /// Verify that truncating or swapping headers between different TTDR
    /// variants (simulated by flags/counts) breaks the checksum or parsing.
    #[test]
    fn drill_ttdr_header_integrity() {
        let frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::new(true, false, false, false, ReceiptMode::Full),
                schema_hash: [0xAA; 32],
                worldline_id: [0xBB; 32],
                tick: 1,
                commit_hash: [0xCC; 32],
                patch_digest: [0xDD; 32],
                state_root: [0xEE; 32],
                emissions_digest: [0xFF; 32],
                op_emission_index_digest: [0x00; 32],
                parent_count: 0,
                channel_count: 0,
            },
            parent_hashes: vec![],
            channel_digests: vec![],
        };

        let mut encoded = encode_ttdr_v2(&frame).unwrap();

        // 1. Truncation check
        let truncated = &encoded[..encoded.len() - 1];
        assert!(decode_ttdr_v2(truncated).is_err());

        // 2. Magic corruption
        encoded[0] = b'X';
        assert!(decode_ttdr_v2(&encoded).is_err());
        encoded[0] = TTDR_MAGIC[0]; // restore

        // 3. Version corruption
        encoded[4] = 99;
        assert!(decode_ttdr_v2(&encoded).is_err());
    }

    /// DRILL: Domain Separation
    ///
    /// Verify that hashing the same content as an EINT vs. a TTDR frame
    /// yields different commitments (structural collision prevention).
    #[test]
    fn drill_domain_separation() {
        let schema_hash = [0x11; 32];
        let payload = b"common_payload";

        // EINT v2 uses BLAKE3 of payload in its header
        let eint_bytes = encode_eint_v2(schema_hash, 1, 1, EintFlags::default(), payload).unwrap();

        // TTDR v2 uses various hashes in its header
        let ttdr_frame = TtdrFrame {
            header: TtdrHeader {
                version: TTDR_VERSION,
                flags: TtdrFlags::default(),
                schema_hash,
                worldline_id: [0; 32],
                tick: 0,
                commit_hash: [0; 32],
                patch_digest: hash(payload).into(), // put payload hash here
                state_root: [0; 32],
                emissions_digest: [0; 32],
                op_emission_index_digest: [0; 32],
                parent_count: 0,
                channel_count: 0,
            },
            parent_hashes: vec![],
            channel_digests: vec![],
        };
        let ttdr_bytes = encode_ttdr_v2(&ttdr_frame).unwrap();

        // The full frames must be distinct even if they share some hashes
        assert_ne!(eint_bytes, ttdr_bytes);
        assert_ne!(hash(&eint_bytes), hash(&ttdr_bytes));
    }

    /// DRILL: Decoder Fuzzer
    ///
    /// Feed randomized bytes to decode_ttdr_v2 ensuring no panics.
    #[test]
    fn drill_ttdr_decoder_fuzzer() {
        use rand::prelude::*;
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..1000 {
            let len = rng.gen_range(0..1024);
            let mut data = vec![0u8; len];
            rng.fill_bytes(&mut data);

            // Should either return Ok or Err, but NEVER panic
            let _ = decode_ttdr_v2(&data);
        }
    }
}
