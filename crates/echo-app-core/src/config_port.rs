// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Config port shared across Echo tools (viewer, host, etc.).

use crate::config::{ConfigService, ConfigStore};
use crate::prefs::ViewerPrefs;

/// Config-facing port for loading/saving viewer preferences (and similar blobs).
pub trait ConfigPort {
    /// Load viewer preferences (returns None if missing or unreadable).
    fn load_prefs(&self) -> Option<ViewerPrefs>;
    /// Persist viewer preferences (best-effort; impl may log errors internally).
    fn save_prefs(&self, prefs: &ViewerPrefs);
}

impl<S: ConfigStore> ConfigPort for ConfigService<S> {
    fn load_prefs(&self) -> Option<ViewerPrefs> {
        self.load::<ViewerPrefs>("viewer_prefs").ok().flatten()
    }

    fn save_prefs(&self, prefs: &ViewerPrefs) {
        let _ = self.save("viewer_prefs", prefs);
    }
}
