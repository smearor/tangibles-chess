use gtk4::prelude::*;

/// Paper-scoresheet styled move list along a board edge.
///
/// Displays `1. e4 e5` rows in a quiet, serif-ish presentation; one
/// instance per player edge, each readable from its own side.
pub struct ScoreSheet {
    /// The scrollable container.
    pub widget: gtk4::ScrolledWindow,
    /// Rows of move pairs.
    rows: gtk4::ListBox,
}

impl ScoreSheet {
    /// A new empty sheet.
    pub fn new() -> Self {
        let rows = gtk4::ListBox::new();
        rows.add_css_class("score-sheet");
        rows.set_selection_mode(gtk4::SelectionMode::None);
        let widget = gtk4::ScrolledWindow::new();
        widget.set_child(Some(&rows));
        widget.set_propagate_natural_height(true);
        widget.set_min_content_height(120);
        widget.set_hscrollbar_policy(gtk4::PolicyType::Never);
        Self { widget, rows }
    }

    /// Rebuilds the rows from the SAN history.
    pub fn set_moves(&self, sans: &[String]) {
        while let Some(child) = self.rows.first_child() {
            self.rows.remove(&child);
        }
        for pair in sans.chunks(2) {
            let turn = gtk4::Label::new(Some(&format!("{}. {}", pair[0], pair.get(1).map(String::as_str).unwrap_or(""))));
            turn.set_xalign(0.0);
            turn.add_css_class("score-row");
            self.rows.append(&turn);
        }
        // Keep the tail visible.
        let adjustment = self.widget.vadjustment();
        adjustment.set_value(adjustment.upper());
    }
}
