// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

//! METHOD workspace discovery and validation.

use std::path::{Path, PathBuf};

/// Known backlog lane names.
pub(crate) const LANES: &[&str] = &["inbox", "asap", "up-next", "cool-ideas", "bad-code"];

/// A validated METHOD workspace rooted at a filesystem path.
pub struct MethodWorkspace {
    root: PathBuf,
}

impl MethodWorkspace {
    /// Discover a METHOD workspace at the given root.
    ///
    /// Returns an error if the required directory structure is missing.
    pub fn discover(root: &Path) -> Result<Self, String> {
        let backlog = root.join("docs/method/backlog");
        if !backlog.is_dir() {
            return Err(format!(
                "not a METHOD workspace: {} missing",
                backlog.display()
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Return the path to the backlog root.
    pub fn backlog_root(&self) -> PathBuf {
        self.root.join("docs/method/backlog")
    }

    /// Return the path to the METHOD docs root.
    pub fn method_root(&self) -> PathBuf {
        self.root.join("docs/method")
    }

    /// Return the path to the design docs root.
    pub fn design_root(&self) -> PathBuf {
        self.root.join("docs/design")
    }

    /// Return the path to the retro root.
    pub fn retro_root(&self) -> PathBuf {
        self.root.join("docs/method/retro")
    }
}
