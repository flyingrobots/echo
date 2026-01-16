// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! In-memory config store fake for testing without filesystem I/O.

use echo_app_core::config::{ConfigError, ConfigStore};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory implementation of [`ConfigStore`] for testing.
///
/// This fake allows tests to verify config save/load behavior without
/// touching the filesystem. It also tracks call counts for verification.
///
/// # Example
///
/// ```
/// use echo_dry_tests::InMemoryConfigStore;
/// use echo_app_core::config::{ConfigService, ConfigStore};
///
/// let store = InMemoryConfigStore::new();
/// let service = ConfigService::new(store.clone());
///
/// service.save("prefs", &serde_json::json!({"theme": "dark"})).unwrap();
/// assert_eq!(store.load_count(), 0);
/// assert_eq!(store.save_count(), 1);
/// ```
#[derive(Clone, Default)]
pub struct InMemoryConfigStore {
    inner: Arc<Mutex<InMemoryConfigStoreInner>>,
}

#[derive(Default)]
struct InMemoryConfigStoreInner {
    data: HashMap<String, Vec<u8>>,
    load_count: usize,
    save_count: usize,
    fail_on_load: bool,
    fail_on_save: bool,
}

impl InMemoryConfigStore {
    /// Create a new empty in-memory config store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a store pre-populated with the given key-value pairs.
    pub fn with_data(data: HashMap<String, Vec<u8>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InMemoryConfigStoreInner {
                data,
                ..Default::default()
            })),
        }
    }

    /// Configure the store to fail on load operations.
    pub fn set_fail_on_load(&self, fail: bool) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.fail_on_load = fail;
    }

    /// Configure the store to fail on save operations.
    pub fn set_fail_on_save(&self, fail: bool) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.fail_on_save = fail;
    }

    /// Get the number of times `load_raw` was called.
    pub fn load_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .load_count
    }

    /// Get the number of times `save_raw` was called.
    pub fn save_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .save_count
    }

    /// Get all keys that have been saved.
    pub fn keys(&self) -> Vec<String> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .data
            .keys()
            .cloned()
            .collect()
    }

    /// Check if a key exists in the store.
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .data
            .contains_key(key)
    }

    /// Clear all stored data and reset counters.
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.data.clear();
        inner.load_count = 0;
        inner.save_count = 0;
        inner.fail_on_load = false;
        inner.fail_on_save = false;
    }
}

impl ConfigStore for InMemoryConfigStore {
    fn load_raw(&self, key: &str) -> Result<Vec<u8>, ConfigError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.load_count += 1;

        if inner.fail_on_load {
            return Err(ConfigError::Other("simulated load failure".into()));
        }

        inner.data.get(key).cloned().ok_or(ConfigError::NotFound)
    }

    fn save_raw(&self, key: &str, data: &[u8]) -> Result<(), ConfigError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.save_count += 1;

        if inner.fail_on_save {
            return Err(ConfigError::Other("simulated save failure".into()));
        }

        inner.data.insert(key.to_string(), data.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_save_load() {
        let store = InMemoryConfigStore::new();
        store.save_raw("test", b"hello").unwrap();
        let loaded = store.load_raw("test").unwrap();
        assert_eq!(loaded, b"hello");
        assert_eq!(store.save_count(), 1);
        assert_eq!(store.load_count(), 1);
    }

    #[test]
    fn load_missing_key_returns_not_found() {
        let store = InMemoryConfigStore::new();
        let result = store.load_raw("missing");
        assert!(matches!(result, Err(ConfigError::NotFound)));
    }

    #[test]
    fn fail_on_load_returns_error() {
        let store = InMemoryConfigStore::new();
        store.save_raw("test", b"data").unwrap();
        store.set_fail_on_load(true);
        let result = store.load_raw("test");
        assert!(matches!(result, Err(ConfigError::Other(_))));
    }

    #[test]
    fn fail_on_save_returns_error() {
        let store = InMemoryConfigStore::new();
        store.set_fail_on_save(true);
        let result = store.save_raw("test", b"data");
        assert!(matches!(result, Err(ConfigError::Other(_))));
    }
}
