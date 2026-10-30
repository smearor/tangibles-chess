use std::cell::Cell;

use gtk4::MediaFile;
use gtk4::gio::ResourceLookupFlags;
use gtk4::gio::resources_lookup_data;
use gtk4::prelude::*;

use crate::sound_effect::SoundEffect;

/// Game sound effects via `gtk::MediaFile` on gresource-embedded audio.
///
/// Assets land in M3 (`assets/sounds/*.ogg` compiled into a gresource);
/// until then `play` degrades to a no-op when the resource is missing.
pub struct SoundPlayer {
    /// Whether sound is enabled.
    enabled: Cell<bool>,
}

impl SoundPlayer {
    /// A player honoring the settings flag.
    pub fn new(enabled: bool) -> Self {
        Self { enabled: Cell::new(enabled) }
    }

    /// Toggles playback (sound is a global app setting, live-applied).
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.set(enabled);
    }

    /// Plays a sound effect's resource if present.
    pub fn play(&self, effect: SoundEffect) {
        if !self.enabled.get() {
            return;
        }
        let path = format!("/io/github/smearor/tangibles_chess/sounds/{effect}.ogg");
        if resources_lookup_data(&path, ResourceLookupFlags::NONE).is_err() {
            return;
        }
        let file = MediaFile::for_resource(&path);
        file.play();
    }
}
