// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tests for the TTD controller and privacy redaction.

use echo_wasm_bindings::{PrivacyMask, TtdController, Value};

#[test]
fn test_ttd_session_lifecycle() {
    let controller = TtdController::new();
    let token = controller.open_session();

    assert_eq!(token.0, 1);

    let closed = controller.close_session(token);
    assert!(closed);

    let closed_again = controller.close_session(token);
    assert!(!closed_again);
}

#[test]
fn test_privacy_redaction() {
    let controller = TtdController::new();
    let token = controller.open_session();

    let secret_val = Value::Str("secret password".into());

    // Default is Public
    let r1 = controller
        .redact_value(token, "password", secret_val.clone())
        .unwrap();
    assert_eq!(r1, secret_val);

    // Set to Private
    controller
        .set_privacy_mask(token, "password".into(), PrivacyMask::Private)
        .unwrap();
    let r2 = controller
        .redact_value(token, "password", secret_val.clone())
        .unwrap();
    assert_eq!(r2, Value::Null);

    // Set to Pseudonymized
    controller
        .set_privacy_mask(token, "password".into(), PrivacyMask::Pseudonymized)
        .unwrap();
    let r3 = controller
        .redact_value(token, "password", secret_val.clone())
        .unwrap();
    if let Value::Str(s) = r3 {
        assert!(s.contains("hash("));
    } else {
        panic!("expected pseudonymous string");
    }

    // Number pseudonymization
    let secret_num = Value::Num(0x12345678);
    controller
        .set_privacy_mask(token, "id".into(), PrivacyMask::Pseudonymized)
        .unwrap();
    let r4 = controller.redact_value(token, "id", secret_num).unwrap();
    assert_eq!(r4, Value::Num(0x12345600)); // Truncated last byte
}

#[test]
fn test_invalid_token_errors() {
    let controller = TtdController::new();
    let bogus_token = echo_wasm_abi::SessionToken(999);

    let res = controller.redact_value(bogus_token, "any", Value::Null);
    assert!(res.is_err());
}
