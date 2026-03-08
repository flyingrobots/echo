// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! TTD Controller and Session management for WASM.

use echo_wasm_abi::{PrivacyMask, SessionToken, TtdError, Value};
use std::collections::BTreeMap;
use std::sync::Mutex;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Controller for TTD sessions.
///
/// Manages active sessions and provides field-level redaction based on privacy masks.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct TtdController {
    sessions: Mutex<BTreeMap<SessionToken, TtdSession>>,
    next_token: Mutex<u64>,
}

struct TtdSession {
    #[allow(dead_code)]
    token: SessionToken,
    privacy_policy: BTreeMap<String, PrivacyMask>,
}

// ---------------------------------------------------------------------------
// Host API (plain Rust, no wasm_bindgen)
// ---------------------------------------------------------------------------

impl TtdController {
    /// Create a new TTD controller.
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(BTreeMap::new()),
            next_token: Mutex::new(1),
        }
    }

    /// Open a new TTD session.
    pub fn open_session(&self) -> SessionToken {
        let mut next_token = self.next_token.lock().unwrap();
        let token = SessionToken(*next_token);
        *next_token += 1;

        let session = TtdSession {
            token,
            privacy_policy: BTreeMap::new(),
        };

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(token, session);
        token
    }

    /// Close an active TTD session.
    pub fn close_session(&self, token: SessionToken) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(&token).is_some()
    }

    /// Set privacy mask for a field in a session.
    pub fn set_privacy_mask(
        &self,
        token: SessionToken,
        field: String,
        mask: PrivacyMask,
    ) -> Result<(), TtdError> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(&token).ok_or(TtdError::InvalidToken)?;
        session.privacy_policy.insert(field, mask);
        Ok(())
    }

    /// Redact a value based on the session's privacy policy.
    pub fn redact_value(
        &self,
        token: SessionToken,
        field: &str,
        value: Value,
    ) -> Result<Value, TtdError> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(&token).ok_or(TtdError::InvalidToken)?;

        let mask = session
            .privacy_policy
            .get(field)
            .copied()
            .unwrap_or(PrivacyMask::Public);

        Ok(match mask {
            PrivacyMask::Public => value,
            PrivacyMask::Pseudonymized => {
                match value {
                    Value::Str(s) => Value::Str(format!("hash({})", s.len())), // Mock pseudonymization
                    Value::Num(n) => Value::Num(n & !0xFF),                    // Truncate last byte
                    other => other,
                }
            }
            PrivacyMask::Private => Value::Null,
        })
    }
}

impl Default for TtdController {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// WASM wrappers (JsValue conversion for complex types)
// ---------------------------------------------------------------------------

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl TtdController {
    /// Create a new TTD controller (WASM constructor).
    #[wasm_bindgen(constructor)]
    pub fn new_wasm() -> Self {
        Self::new()
    }

    /// Open a new TTD session. Returns the session token as a `u64`.
    #[wasm_bindgen(js_name = openSession)]
    pub fn open_session_wasm(&self) -> u64 {
        self.open_session().0
    }

    /// Close an active TTD session by token.
    #[wasm_bindgen(js_name = closeSession)]
    pub fn close_session_wasm(&self, token: u64) -> bool {
        self.close_session(SessionToken(token))
    }

    /// Set privacy mask for a field. Mask: 0 = Public, 1 = Pseudonymized, 2 = Private.
    #[wasm_bindgen(js_name = setPrivacyMask)]
    pub fn set_privacy_mask_wasm(
        &self,
        token: u64,
        field: String,
        mask: u8,
    ) -> Result<(), JsError> {
        let mask = match mask {
            0 => PrivacyMask::Public,
            1 => PrivacyMask::Pseudonymized,
            2 => PrivacyMask::Private,
            _ => return Err(JsError::new("invalid privacy mask (expected 0, 1, or 2)")),
        };
        self.set_privacy_mask(SessionToken(token), field, mask)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Redact a value based on the session's privacy policy.
    #[wasm_bindgen(js_name = redactValue)]
    pub fn redact_value_wasm(
        &self,
        token: u64,
        field: String,
        value: JsValue,
    ) -> Result<JsValue, JsError> {
        let val: Value =
            serde_wasm_bindgen::from_value(value).map_err(|e| JsError::new(&e.to_string()))?;
        let result = self
            .redact_value(SessionToken(token), &field, val)
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&result).map_err(|e| JsError::new(&e.to_string()))
    }
}
