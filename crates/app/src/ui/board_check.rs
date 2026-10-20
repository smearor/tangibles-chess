use fluent::FluentArgs;
use gtk4::prelude::*;
use tangibles_chess_tangibles::PlacementStatus;

/// Overlay card of the start handshake: shows placement progress
/// (`n/32 pieces`) and hints which squares still need correction.
pub struct BoardCheckOverlay {
    /// Root widget (hidden when consistent).
    pub widget: gtk4::Box,
    /// Progress line.
    progress: gtk4::Label,
    /// First missing hint.
    missing: gtk4::Label,
    /// First unexpected hint.
    unexpected: gtk4::Label,
}

impl BoardCheckOverlay {
    /// A new overlay bound to `l10n` texts.
    pub fn new(l10n: &crate::l10n::L10n) -> Self {
        let widget = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        widget.add_css_class("board-check");
        widget.set_halign(gtk4::Align::Center);
        widget.set_valign(gtk4::Align::Center);
        let heading = gtk4::Label::new(Some(&l10n.msg("check-heading")));
        heading.add_css_class("title-2");
        let progress = gtk4::Label::new(None);
        let missing = gtk4::Label::new(None);
        let unexpected = gtk4::Label::new(None);
        widget.append(&heading);
        widget.append(&progress);
        widget.append(&missing);
        widget.append(&unexpected);
        widget.set_visible(false);
        Self {
            widget,
            progress,
            missing,
            unexpected,
        }
    }

    /// Updates the overlay from a placement status; hides itself when the
    /// board is consistent.
    pub fn set_status(&self, status: &PlacementStatus, l10n: &crate::l10n::L10n) {
        self.widget.set_visible(!status.is_consistent());
        let mut args = FluentArgs::new();
        args.set("placed", status.placed() as i64);
        args.set("total", status.total as i64);
        self.progress.set_text(&l10n.msg_args("check-progress", &args));
        self.missing.set_text(
            &status
                .missing
                .first()
                .map(|square| {
                    let mut args = FluentArgs::new();
                    args.set("square", square.to_string());
                    l10n.msg_args("check-missing", &args)
                })
                .unwrap_or_default(),
        );
        self.unexpected.set_text(
            &status
                .unexpected
                .first()
                .map(|square| {
                    let mut args = FluentArgs::new();
                    args.set("square", square.to_string());
                    l10n.msg_args("check-unexpected", &args)
                })
                .unwrap_or_default(),
        );
    }
}
