// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Config service and storage port for Echo tools.

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Storage port for raw config blobs (keyed by logical name).
pub trait ConfigStore {
    /// Load a raw config blob. Returns `NotFound` when missing.
    fn load_raw(&self, key: &str) -> Result<Vec<u8>, ConfigError>;
    /// Persist a raw config blob.
    fn save_raw(&self, key: &str, data: &[u8]) -> Result<(), ConfigError>;
}

/// Error type for config operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Key not present in store.
    #[error("not found")]
    NotFound,
    /// I/O error while reading/writing.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization/deserialization failure.
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    /// Catch-all error variant.
    #[error("other: {0}")]
    Other(String),
}

/// Thin service that serializes config values and delegates storage to a `ConfigStore`.
pub struct ConfigService<S> {
    store: S,
}

impl<S> ConfigService<S> {
    /// Create a new service using the given store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Consume the service and return the inner store.
    pub fn into_inner(self) -> S {
        self.store
    }
}

impl<S> ConfigService<S>
where
    S: ConfigStore,
{
    /// Load and deserialize a config value for `key`. Returns `Ok(None)` if missing.
    pub fn load<T>(&self, key: &str) -> Result<Option<T>, ConfigError>
    where
        T: DeserializeOwned,
    {
        match self.store.load_raw(key) {
            Ok(bytes) => {
                if bytes.is_empty() {
                    return Ok(None);
                }
                let value = serde_json::from_slice(&bytes)?;
                Ok(Some(value))
            }
            Err(ConfigError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Serialize and persist a config value for `key`.
    pub fn save<T>(&self, key: &str, value: &T) -> Result<(), ConfigError>
    where
        T: Serialize,
    {
        let data = serde_json::to_vec_pretty(value)?;
        self.store.save_raw(key, &data)
    }
}
