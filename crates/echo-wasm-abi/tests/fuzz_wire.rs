use proptest::prelude::*;
use echo_wasm_abi::unpack_intent_v1;

proptest! {
    #[test]
    fn fuzz_unpack_intent_v1_no_panics(bytes in prop::collection::vec(any::<u8>(), 0..1024)) {
        // The goal is simply to ensure this does not panic.
        let _ = unpack_intent_v1(&bytes);
    }

    #[test]
    fn fuzz_unpack_valid_structure_garbage_payload(
        op_id in any::<u32>(),
        len in 0u32..1000,
        payload in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        let mut data = Vec::new();
        data.extend_from_slice(b"EINT");
        data.extend_from_slice(&op_id.to_le_bytes());
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&payload);

        let res = unpack_intent_v1(&data);
        
        if payload.len() == len as usize {
            // Should succeed
             prop_assert!(res.is_ok());
             let (out_op, out_vars) = res.unwrap();
             prop_assert_eq!(out_op, op_id);
             prop_assert_eq!(out_vars, &payload[..]);
        } else {
             // Should fail cleanly
             prop_assert!(res.is_err());
        }
    }
}
