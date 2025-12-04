// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Config port shared across Echo tools (viewer, host, etc.).

use crate::prefs::ViewerPrefs;

/// Config-facing port for loading/saving viewer preferences (and similar blobs).
pub trait ConfigPort {
    /// Load viewer preferences (returns None if missing or unreadable).
    fn load_prefs(&self) -> Option<ViewerPrefs>;
    /// Persist viewer preferences (best-effort; impl may log errors internally).
    fn save_prefs(&self, prefs: &ViewerPrefs);
}
