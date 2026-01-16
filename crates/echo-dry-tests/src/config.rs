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

    /// Get the number of times `load_raw` was called (attempted, not successful).
    ///
    /// This counter is incremented at the start of each `load_raw` call,
    /// before any failure checks. It counts all attempts, including those
    /// that fail due to `set_fail_on_load(true)` or missing keys.
    pub fn load_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .load_count
    }

    /// Get the number of times `save_raw` was called (attempted, not successful).
    ///
    /// This counter is incremented at the start of each `save_raw` call,
    /// before any failure checks. It counts all attempts, including those
    /// that fail due to `set_fail_on_save(true)`.
    pub fn save_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .save_count
    }

    /// Return all keys currently present in the store.
    ///
    /// This includes keys from any source:
    /// - Keys added via `save_raw` calls
    /// - Keys pre-populated via [`with_data()`](Self::with_data)
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

    /// Reset the store to its initial empty state.
    ///
    /// Clears all fields:
    /// - `data`: All stored key-value pairs are removed
    /// - `load_count`: Reset to 0
    /// - `save_count`: Reset to 0
    /// - `fail_on_load`: Reset to false
    /// - `fail_on_save`: Reset to false
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

    #[test]
    fn with_data_prepopulates_store() {
        let mut initial_data = HashMap::new();
        initial_data.insert("key1".to_string(), b"value1".to_vec());
        initial_data.insert("key2".to_string(), b"value2".to_vec());

        let store = InMemoryConfigStore::with_data(initial_data);

        assert_eq!(store.load_raw("key1").unwrap(), b"value1");
        assert_eq!(store.load_raw("key2").unwrap(), b"value2");
        // load_count should reflect the loads we just did
        assert_eq!(store.load_count(), 2);
        // save_count should be 0 since we only used with_data
        assert_eq!(store.save_count(), 0);
    }

    #[test]
    fn with_data_empty_hashmap_creates_empty_store() {
        let store = InMemoryConfigStore::with_data(HashMap::new());

        assert!(store.keys().is_empty());
        let result = store.load_raw("any_key");
        assert!(matches!(result, Err(ConfigError::NotFound)));
    }

    #[test]
    fn keys_returns_all_stored_keys() {
        let store = InMemoryConfigStore::new();
        store.save_raw("alpha", b"a").unwrap();
        store.save_raw("beta", b"b").unwrap();
        store.save_raw("gamma", b"c").unwrap();

        let mut keys = store.keys();
        keys.sort();
        assert_eq!(keys, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn keys_returns_empty_vec_for_empty_store() {
        let store = InMemoryConfigStore::new();
        assert!(store.keys().is_empty());
    }

    #[test]
    fn contains_key_returns_true_for_existing_key() {
        let store = InMemoryConfigStore::new();
        store.save_raw("exists", b"data").unwrap();

        assert!(store.contains_key("exists"));
    }

    #[test]
    fn contains_key_returns_false_for_missing_key() {
        let store = InMemoryConfigStore::new();
        store.save_raw("exists", b"data").unwrap();

        assert!(!store.contains_key("does_not_exist"));
    }

    #[test]
    fn contains_key_returns_false_for_empty_store() {
        let store = InMemoryConfigStore::new();
        assert!(!store.contains_key("any_key"));
    }

    #[test]
    fn reset_clears_all_data() {
        let store = InMemoryConfigStore::new();
        store.save_raw("key1", b"value1").unwrap();
        store.save_raw("key2", b"value2").unwrap();

        store.reset();

        assert!(store.keys().is_empty());
        assert!(!store.contains_key("key1"));
        assert!(!store.contains_key("key2"));
    }

    #[test]
    fn reset_clears_load_and_save_counts() {
        let store = InMemoryConfigStore::new();
        store.save_raw("key", b"value").unwrap();
        let _ = store.load_raw("key");
        let _ = store.load_raw("key");

        assert_eq!(store.save_count(), 1);
        assert_eq!(store.load_count(), 2);

        store.reset();

        assert_eq!(store.save_count(), 0);
        assert_eq!(store.load_count(), 0);
    }

    #[test]
    fn reset_clears_fail_flags() {
        let store = InMemoryConfigStore::new();
        store.set_fail_on_load(true);
        store.set_fail_on_save(true);

        store.reset();

        // After reset, operations should succeed
        store.save_raw("key", b"value").unwrap();
        let result = store.load_raw("key");
        assert!(result.is_ok());
    }

    #[test]
    fn clone_shares_state_between_instances() {
        let store1 = InMemoryConfigStore::new();
        let store2 = store1.clone();

        // Save through store1
        store1.save_raw("shared_key", b"shared_value").unwrap();

        // Load through store2 - should see the same data
        let loaded = store2.load_raw("shared_key").unwrap();
        assert_eq!(loaded, b"shared_value");

        // Counts should be shared
        assert_eq!(store1.save_count(), 1);
        assert_eq!(store2.save_count(), 1);
        assert_eq!(store1.load_count(), 1);
        assert_eq!(store2.load_count(), 1);
    }

    #[test]
    fn clone_shares_fail_flags() {
        let store1 = InMemoryConfigStore::new();
        let store2 = store1.clone();

        // Set fail_on_save through store1
        store1.set_fail_on_save(true);

        // store2 should also fail on save
        let result = store2.save_raw("key", b"value");
        assert!(matches!(result, Err(ConfigError::Other(_))));

        // Set fail_on_load through store2
        store2.set_fail_on_load(true);

        // store1 should also fail on load
        let result = store1.load_raw("any");
        assert!(matches!(result, Err(ConfigError::Other(_))));
    }

    #[test]
    fn clone_shares_reset() {
        let store1 = InMemoryConfigStore::new();
        let store2 = store1.clone();

        store1.save_raw("key", b"value").unwrap();
        store1.set_fail_on_save(true);

        // Reset through store2
        store2.reset();

        // store1 should see the reset state
        assert!(store1.keys().is_empty());
        assert_eq!(store1.save_count(), 0);
        // fail_on_save should be cleared, so this should succeed
        store1.save_raw("new_key", b"new_value").unwrap();
    }

    #[test]
    fn fail_on_save_still_increments_save_count() {
        let store = InMemoryConfigStore::new();
        store.set_fail_on_save(true);

        let _ = store.save_raw("key", b"value");
        let _ = store.save_raw("key2", b"value2");

        // Even failed saves should increment the count
        assert_eq!(store.save_count(), 2);
    }

    #[test]
    fn fail_on_load_still_increments_load_count() {
        let store = InMemoryConfigStore::new();
        store.set_fail_on_load(true);

        let _ = store.load_raw("key");
        let _ = store.load_raw("key2");

        // Even failed loads should increment the count
        assert_eq!(store.load_count(), 2);
    }

    #[test]
    fn fail_on_save_does_not_store_data() {
        let store = InMemoryConfigStore::new();
        store.set_fail_on_save(true);

        let _ = store.save_raw("key", b"value");

        // Data should not be stored when save fails
        assert!(!store.contains_key("key"));
        assert!(store.keys().is_empty());
    }

    #[test]
    fn fail_on_save_can_be_toggled() {
        let store = InMemoryConfigStore::new();

        // Initially should work
        store.save_raw("key1", b"value1").unwrap();

        // Enable failure
        store.set_fail_on_save(true);
        let result = store.save_raw("key2", b"value2");
        assert!(result.is_err());

        // Disable failure
        store.set_fail_on_save(false);
        store.save_raw("key3", b"value3").unwrap();

        // Only key1 and key3 should exist
        assert!(store.contains_key("key1"));
        assert!(!store.contains_key("key2"));
        assert!(store.contains_key("key3"));
    }

    #[test]
    fn fail_on_load_can_be_toggled() {
        let store = InMemoryConfigStore::new();
        store.save_raw("key", b"value").unwrap();

        // Initially should work
        assert!(store.load_raw("key").is_ok());

        // Enable failure
        store.set_fail_on_load(true);
        assert!(store.load_raw("key").is_err());

        // Disable failure
        store.set_fail_on_load(false);
        assert!(store.load_raw("key").is_ok());
    }
}
