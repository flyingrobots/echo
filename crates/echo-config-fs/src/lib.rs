// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Filesystem-backed `ConfigStore` for Echo tools (uses platform config dir).

use directories::ProjectDirs;
use echo_app_core::config::{ConfigError, ConfigStore};
use std::fs;
use std::path::PathBuf;

/// Store configs as JSON files under the platform config directory.
pub struct FsConfigStore {
    base: PathBuf,
}

impl FsConfigStore {
    /// Create a store rooted at the user config directory (e.g., `~/.config/Echo`).
    pub fn new() -> Result<Self, ConfigError> {
        let proj = ProjectDirs::from("dev", "flyingrobots", "Echo")
            .ok_or_else(|| ConfigError::Other("could not resolve config dir".into()))?;
        let base = proj.config_dir().to_path_buf();
        fs::create_dir_all(&base)?;
        Ok(Self { base })
    }

    fn path_for(&self, key: &str) -> PathBuf {
        let filename = format!("{}.json", key);
        self.base.join(filename)
    }
}

impl ConfigStore for FsConfigStore {
    fn load_raw(&self, key: &str) -> Result<Vec<u8>, ConfigError> {
        let path = self.path_for(key);
        match fs::read(path) {
            Ok(bytes) => Ok(bytes),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(ConfigError::NotFound),
            Err(err) => Err(ConfigError::Io(err)),
        }
    }

    fn save_raw(&self, key: &str, data: &[u8]) -> Result<(), ConfigError> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, data)?;
        Ok(())
    }
}
