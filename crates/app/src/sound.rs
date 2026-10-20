use std::cell::Cell;

use gtk4::prelude::*;

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

    /// Plays a named sound resource if present.
    pub fn play(&self, name: &str) {
        if !self.enabled.get() {
            return;
        }
        let path = format!("/io/github/smearor/tangibles_chess/sounds/{name}.ogg");
        if gtk4::gio::resources_lookup_data(&path, gtk4::gio::ResourceLookupFlags::NONE).is_err() {
            return;
        }
        let file = gtk4::MediaFile::for_resource(&path);
        file.play();
    }
}
