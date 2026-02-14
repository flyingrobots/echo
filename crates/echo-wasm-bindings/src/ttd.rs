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

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl TtdController {
    /// Create a new TTD controller.
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
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
