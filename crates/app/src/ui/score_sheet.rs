use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use tangibles_chess_core::Ply;
use tangibles_chess_core::San;
use tangibles_chess_core::Side;
use tracing::debug;

/// Paper-scoresheet styled move list along a board edge.
///
/// Displays `1. e4 e5` rows in a quiet, serif-ish presentation; one
/// instance per player edge, each readable from its own side.
pub struct ScoreSheet {
    /// The scrollable container.
    pub widget: gtk4::ScrolledWindow,
    /// Rows of move pairs.
    rows: gtk4::ListBox,
    /// The rendered SAN history (kept for re-rendering on highlight
    /// changes).
    sans: RefCell<Vec<San>>,
    /// Highlighted ply index (0-based into the SAN list, replay
    /// cursor).
    active_ply: Cell<Option<Ply>>,
    /// Click callback for a ply (0-based index into the SAN list).
    on_ply: RefCell<Option<PlyHandler>>,
    /// Invalidates scroll callbacks scheduled by older renders.
    scroll_gen: Rc<Cell<u64>>,
}

/// Ply click callback receiving a 0-based index into the SAN list.
type PlyHandler = Rc<dyn Fn(Ply)>;

impl ScoreSheet {
    /// A new empty sheet, styled in the player's color like the control
    /// panel.
    ///
    /// The height is fixed at `min_content_height`: without natural-height
    /// propagation the sheet scrolls internally instead of growing the edge
    /// row (and shrinking the board) as the move list gets longer.
    pub fn new(side: Side) -> Self {
        let rows = gtk4::ListBox::new();
        rows.add_css_class("score-sheet");
        rows.set_selection_mode(gtk4::SelectionMode::None);
        let widget = gtk4::ScrolledWindow::new();
        widget.add_css_class("score-sheet-frame");
        widget.add_css_class(side.css_class());
        widget.set_child(Some(&rows));
        widget.set_propagate_natural_height(false);
        widget.set_min_content_height(120);
        widget.set_hscrollbar_policy(gtk4::PolicyType::Never);
        Self {
            widget,
            rows,
            sans: RefCell::new(Vec::new()),
            active_ply: Cell::new(None),
            on_ply: RefCell::new(None),
            scroll_gen: Rc::new(Cell::new(0)),
        }
    }

    /// Sets the click callback for individual plies (replay navigation).
    pub fn set_on_ply(&self, callback: PlyHandler) {
        *self.on_ply.borrow_mut() = Some(callback);
        self.render();
    }

    /// Rebuilds the rows from the SAN history.
    pub fn set_moves(&self, sans: &[San]) {
        self.sans.replace(sans.to_vec());
        self.render();
    }

    /// Highlights a ply in the list (0-based index into the SAN list);
    /// `None` clears the highlight. Used by replay mode to mark the
    /// move that produced the displayed position.
    pub fn set_active_ply(&self, ply: Option<Ply>) {
        self.active_ply.set(ply);
        self.render();
    }

    /// Re-renders the move rows, bolding the active ply if set. Each
    /// half-move is a flat button so a ply can be selected directly.
    fn render(&self) {
        while let Some(child) = self.rows.first_child() {
            self.rows.remove(&child);
        }
        let active = self.active_ply.get();
        let on_ply = self.on_ply.borrow().clone();
        for (row_index, pair) in self.sans.borrow().chunks(2).enumerate() {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
            let number = gtk4::Label::new(Some(&format!("{}.", row_index + 1)));
            number.set_xalign(0.0);
            number.add_css_class("score-row");
            row.append(&number);
            for (offset, san) in pair.iter().enumerate() {
                let ply = Ply::from(row_index * 2 + offset);
                let button = gtk4::Button::new();
                button.add_css_class("flat");
                button.add_css_class("score-ply");
                // A focused widget makes the scrolled window chase it
                // on every rebuild, yanking the clicked sheet back to
                // the top - ply buttons are for touch, not keyboard.
                button.set_focusable(false);
                if active == Some(ply) {
                    button.add_css_class("score-ply-active");
                }
                let label = gtk4::Label::new(None);
                label.set_markup(&Self::ply_markup(san, active == Some(ply)));
                button.set_child(Some(&label));
                if let Some(on_ply) = &on_ply {
                    let on_ply = on_ply.clone();
                    button.connect_clicked(move |_| on_ply(ply));
                }
                row.append(&button);
            }
            self.rows.append(&row);
        }
        // Scroll to the highlighted ply's row in replay mode, otherwise
        // keep the tail visible. The row heights are uniform, so the
        // target position can be computed from the adjustment's upper
        // bound instead of unreliable per-row widget bounds.
        let row_count = self.sans.borrow().len().div_ceil(2);
        let target = active.map(|ply| ply.index() / 2).or((row_count > 0).then(|| row_count - 1));
        // A new render supersedes all scroll callbacks still queued from
        // older renders - without this a stale "scroll to tail" could
        // clobber a later "scroll to the selected ply".
        let scroll_gen = self.scroll_gen.clone();
        scroll_gen.set(scroll_gen.get() + 1);
        let generation = scroll_gen.get();
        let rows = self.rows.clone();
        let adjustment = self.widget.vadjustment();
        let tries = std::cell::Cell::new(0u32);
        rows.add_tick_callback(move |_rows, _clock| {
            if scroll_gen.get() != generation {
                return gtk4::glib::ControlFlow::Break;
            }
            tries.set(tries.get() + 1);
            let Some(index) = target else {
                return gtk4::glib::ControlFlow::Break;
            };
            // `upper` is the full content height; while the rebuilt rows
            // are not yet measured into it, the adjustment still reports
            // a shrunken or stale value - retry a few frames.
            let (upper, page) = (adjustment.upper(), adjustment.page_size());
            let scrollable = upper - page;
            if scrollable <= 0.0 || row_count == 0 {
                return gtk4::glib::ControlFlow::Break;
            }
            let row_height = upper / row_count as f64;
            let row_center = index as f64 * row_height + row_height / 2.0;
            let value = adjustment.value();
            if row_center < value || row_center > value + page {
                let scroll_to = (row_center - page / 2.0).clamp(0.0, scrollable);
                debug!("score sheet scrolls to row {index} (value {value} -> {scroll_to})");
                adjustment.set_value(scroll_to);
            }
            glib::ControlFlow::Break
        });
    }

    /// The ply's SAN, bold when it is the highlighted one.
    fn ply_markup(san: &San, active: bool) -> String {
        if active { format!("<b>{san}</b>") } else { san.to_string() }
    }
}
