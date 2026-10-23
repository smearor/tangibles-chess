use std::cell::RefCell;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::glib;
use gtk4::graphene;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use shakmaty::Chess;
use shakmaty::Position;
use shakmaty::Square;
use tangibles_chess_core::BoardTheme;

use crate::piece_icons::piece_icon_name;
use crate::theme::ThemePalette;

/// Everything the scene needs to draw one frame.
#[derive(Debug)]
pub(crate) struct SceneState {
    /// Current board position; `None` before a game starts.
    pub(crate) position: Option<Chess>,
    /// Whether rendered piece glyphs are drawn at all (off in `Tangibles`).
    pub(crate) render_pieces: bool,
    /// The active board material.
    pub(crate) theme: BoardTheme,
    /// Selected square (touch input), if any.
    pub(crate) selected: Option<Square>,
    /// Legal destinations of the selection.
    pub(crate) legal_targets: Vec<Square>,
    /// Last played move (from, to).
    pub(crate) last_move: Option<(Square, Square)>,
    /// King square currently in check.
    pub(crate) check: Option<Square>,
    /// Square whose marker is in the air (tangible lift in progress).
    pub(crate) lifted: Option<Square>,
    /// Fields lit by the setup ceremony.
    pub(crate) ceremony: BTreeSet<Square>,
    /// Expected-but-missing squares (placement gate / board check).
    pub(crate) missing: Vec<Square>,
    /// Unexpectedly occupied squares.
    pub(crate) foreign: Vec<Square>,
    /// Ghost target of the guided undo, if any.
    pub(crate) ghost: Option<(Square, Square)>,
    /// Physical/logical discrepancy markers (`TangiblesRendered`).
    pub(crate) discrepant: Vec<Square>,
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            position: None,
            render_pieces: true,
            theme: BoardTheme::default(),
            selected: None,
            legal_targets: Vec::new(),
            last_move: None,
            check: None,
            lifted: None,
            ceremony: BTreeSet::new(),
            missing: Vec::new(),
            foreign: Vec::new(),
            ghost: None,
            discrepant: Vec::new(),
        }
    }
}

/// Tap callback receiving a scene-space square.
type SquareHandler = Rc<dyn Fn(Square)>;

/// The drawable, tappable board.
///
/// Renders the board in *scene space* (always `a8` top-left .. `h1`
/// bottom-right); the `RotationWidget` handles the physical orientation.
/// All input arrives already un-rotated in scene coordinates.
mod imp {
    use super::*;

    /// Implementation state of [`ChessScene`].
    #[derive(Default)]
    pub struct ChessSceneImp {
        /// Render state.
        pub(crate) state: RefCell<SceneState>,
        /// Tap handler receiving scene-space squares.
        pub on_square: RefCell<Option<SquareHandler>>,
        /// Icon paintable cache (icon name → paintable).
        pub paintables: RefCell<HashMap<String, gtk4::IconPaintable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ChessSceneImp {
        const NAME: &'static str = "TangiblesChessScene";
        type Type = super::ChessScene;
        type ParentType = gtk4::Widget;
    }

    impl ObjectImpl for ChessSceneImp {}

    impl WidgetImpl for ChessSceneImp {
        /// (minimum, natural, min_baseline, nat_baseline): a small
        /// natural request - the tabletop must fit the window, never
        /// drive its size; vexpand distributes the surplus.
        fn measure(&self, _orientation: gtk4::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (200, 400, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let width = self.obj().width() as f64;
            let height = self.obj().height() as f64;
            if width <= 0.0 || height <= 0.0 {
                return;
            }
            let Some((bx, by, bs)) = board_rect(width, height) else {
                return;
            };
            let cell = bs / 8.0;
            let state = self.state.borrow();
            let palette = ThemePalette::of(state.theme);

            // Board frame (the material border with coordinate inlay).
            // append_cairo clips to the given bounds but the context
            // itself draws in widget coordinates.
            let frame = snapshot.append_cairo(&graphene::Rect::new(
                (bx - bs * 0.045) as f32,
                (by - bs * 0.045) as f32,
                (bs * 1.09) as f32,
                (bs * 1.09) as f32,
            ));
            frame.set_source_rgb(palette.frame.0, palette.frame.1, palette.frame.2);
            frame.rectangle(bx - bs * 0.045, by - bs * 0.045, bs * 1.09, bs * 1.09);
            let _ = frame.fill();

            // Coordinate inlay engraved into the frame: ranks run on
            // both vertical edges (left upright for the south player,
            // right rotated 180° for the north player); files on the
            // bottom edge upright and on the top edge rotated, so each
            // player reads the labels on their own side.
            let muted = (
                (palette.light.0 + palette.frame.0) / 2.0 + 0.08,
                (palette.light.1 + palette.frame.1) / 2.0 + 0.08,
                (palette.light.2 + palette.frame.2) / 2.0 + 0.08,
            );
            frame.set_source_rgb(muted.0, muted.1, muted.2);
            let margin = bs * 0.045;
            let font = margin * 0.55;
            frame.set_font_size(font);
            for row in 0..8usize {
                let rank = (8 - row).to_string();
                let cy = by + row as f64 * cell + cell / 2.0;
                text_centered(&frame, &rank, bx - margin / 2.0, cy, 0.0);
                text_centered(&frame, &rank, bx + bs + margin / 2.0, cy, std::f64::consts::PI);
            }
            for col in 0..8usize {
                let letter = ((b'a' + col as u8) as char).to_string();
                let cx = bx + col as f64 * cell + cell / 2.0;
                text_centered(&frame, &letter, cx, by + bs + margin / 2.0, 0.0);
                text_centered(&frame, &letter, cx, by - margin / 2.0, std::f64::consts::PI);
            }

            let squares = snapshot.append_cairo(&graphene::Rect::new(bx as f32, by as f32, bs as f32, bs as f32));
            for scene in 0..64usize {
                let (row, col) = (scene / 8, scene % 8);
                let square = scene_square(scene);
                let base = if (row + col) % 2 == 0 { palette.light } else { palette.dark };
                let x = bx + col as f64 * cell;
                let y = by + row as f64 * cell;
                squares.set_source_rgb(base.0, base.1, base.2);
                squares.rectangle(x, y, cell, cell);
                let _ = squares.fill();

                // Layered highlights.
                if state.last_move.is_some_and(|(from, to)| from == square || to == square) {
                    fill_highlight(&squares, x, y, cell, palette.last_move, 0.28);
                }
                if state.ceremony.contains(&square) {
                    fill_highlight(&squares, x, y, cell, palette.ceremony, 0.35);
                }
                if state.missing.contains(&square) {
                    ring_highlight(&squares, x, y, cell, palette.check);
                }
                if state.foreign.contains(&square) || state.discrepant.contains(&square) {
                    ring_highlight(&squares, x, y, cell, palette.target);
                }
                if state.selected == Some(square) {
                    fill_highlight(&squares, x, y, cell, palette.selection, 0.30);
                }
                if state.legal_targets.contains(&square) {
                    target_dot(&squares, x, y, cell, palette.target);
                }
                if state.check == Some(square) {
                    fill_highlight(&squares, x, y, cell, palette.check, 0.45);
                }
                if state.lifted == Some(square) {
                    fill_highlight(&squares, x, y, cell, palette.selection, 0.18);
                }
                if state.ghost.is_some_and(|(_, to)| to == square) {
                    ring_highlight(&squares, x, y, cell, palette.ceremony);
                }
            }

            // Piece layer.
            if state.render_pieces
                && let Some(position) = &state.position
            {
                let scale = self.obj().scale_factor();
                for square in Square::ALL {
                    let Some(piece) = position.board().piece_at(square) else {
                        continue;
                    };
                    if state.lifted == Some(square) {
                        continue; // in the player's hand
                    }
                    let scene = square_scene(square);
                    let x = bx + (scene % 8) as f64 * cell;
                    let y = by + (scene / 8) as f64 * cell;
                    let name = piece_icon_name(piece);
                    let paintable = {
                        let mut cache = self.paintables.borrow_mut();
                        cache
                            .entry(name.to_string())
                            .or_insert_with(|| {
                                gtk4::IconTheme::for_display(&self.obj().display()).lookup_icon(
                                    name,
                                    &[],
                                    cell as i32,
                                    scale,
                                    gtk4::TextDirection::None,
                                    gtk4::IconLookupFlags::empty(),
                                )
                            })
                            .clone()
                    };
                    snapshot.save();
                    snapshot.translate(&graphene::Point::new(x as f32, y as f32));
                    if piece.color == shakmaty::Color::Black {
                        // The north player faces their own pieces: flip
                        // black glyphs around the cell centre so they are
                        // readable upside down.
                        snapshot.translate(&graphene::Point::new((cell / 2.0) as f32, (cell / 2.0) as f32));
                        snapshot.rotate(180.0);
                        snapshot.translate(&graphene::Point::new((-cell / 2.0) as f32, (-cell / 2.0) as f32));
                    }
                    paintable.snapshot(snapshot, cell, cell);
                    snapshot.restore();
                }
            }
        }
    }
}

glib::wrapper! {
    /// The board widget: draws frame, squares, highlights and pieces, and
    /// reports taps in scene coordinates.
    pub struct ChessScene(ObjectSubclass<imp::ChessSceneImp>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl ChessScene {
    /// A new scene with a tap-to-square click gesture.
    pub fn new() -> Self {
        let scene: Self = glib::Object::builder().build();
        scene.set_hexpand(true);
        scene.set_vexpand(true);
        let click = gtk4::GestureClick::new();
        click.connect_released(glib::clone!(
            #[weak]
            scene,
            move |_, _n, x, y| {
                if let Some(square) = scene.point_to_square(x, y)
                    && let Some(handler) = scene.imp().on_square.borrow().as_ref()
                {
                    handler(square);
                }
            }
        ));
        scene.add_controller(click);
        scene
    }

    /// Registers the tap callback.
    pub fn connect_square(&self, handler: impl Fn(Square) + 'static) {
        *self.imp().on_square.borrow_mut() = Some(Rc::new(handler) as SquareHandler);
    }

    /// Mutates the render state and queues a redraw.
    pub(crate) fn update(&self, apply: impl FnOnce(&mut SceneState)) {
        apply(&mut self.imp().state.borrow_mut());
        self.queue_draw();
    }

    /// The board rectangle in scene-local pixels.
    pub fn board_rect(&self) -> Option<(f64, f64, f64)> {
        board_rect(self.width() as f64, self.height() as f64)
    }

    /// The board rectangle converted to screen millimeters, using the
    /// widget's position inside the window.
    pub fn board_rect_mm(&self, mm_per_px: f64) -> Option<(f64, f64, f64)> {
        let (bx, by, bs) = self.board_rect()?;
        let root = self.root()?;
        let (ox, oy) = self
            .compute_point(&root, &graphene::Point::new(0.0, 0.0))
            .map(|point| (point.x() as f64, point.y() as f64))
            .unwrap_or((0.0, 0.0));
        Some(((bx + ox) * mm_per_px, (by + oy) * mm_per_px, bs * mm_per_px))
    }

    /// Maps a scene-space point to a square.
    pub fn point_to_square(&self, x: f64, y: f64) -> Option<Square> {
        let (bx, by, bs) = board_rect(self.width() as f64, self.height() as f64)?;
        if x < bx || y < by || x >= bx + bs || y >= by + bs {
            return None;
        }
        let col = ((x - bx) / (bs / 8.0)) as usize;
        let row = ((y - by) / (bs / 8.0)) as usize;
        Some(scene_square(row * 8 + col))
    }
}

/// Centered board rect inside the widget.
fn board_rect(width: f64, height: f64) -> Option<(f64, f64, f64)> {
    let side = width.min(height) * 0.88;
    if side <= 0.0 {
        return None;
    }
    Some(((width - side) / 2.0, (height - side) / 2.0, side))
}

/// Scene-space index (0 = top-left = a8) → square.
fn scene_square(scene: usize) -> Square {
    Square::ALL[(7 - scene / 8) * 8 + scene % 8]
}

/// Square → scene-space index.
fn square_scene(square: Square) -> usize {
    let index = square as usize; // a1=0..h8=63
    let (file, rank) = (index % 8, index / 8);
    (7 - rank) * 8 + file
}

/// Soft filled highlight over one cell.
fn fill_highlight(cr: &gtk4::cairo::Context, x: f64, y: f64, cell: f64, color: (f64, f64, f64), alpha: f64) {
    cr.set_source_rgba(color.0, color.1, color.2, alpha);
    cr.rectangle(x, y, cell, cell);
    let _ = cr.fill();
}

/// Ring outline inside a cell (missing/foreign/ghost).
fn ring_highlight(cr: &gtk4::cairo::Context, x: f64, y: f64, cell: f64, color: (f64, f64, f64)) {
    cr.set_source_rgba(color.0, color.1, color.2, 0.85);
    cr.set_line_width((cell * 0.06).max(2.0));
    let inset = cell * 0.08;
    cr.rectangle(x + inset, y + inset, cell - inset * 2.0, cell - inset * 2.0);
    let _ = cr.stroke();
}

/// Legal-target dot.
fn target_dot(cr: &gtk4::cairo::Context, x: f64, y: f64, cell: f64, color: (f64, f64, f64)) {
    cr.set_source_rgba(color.0, color.1, color.2, 0.55);
    let center = cell / 2.0;
    cr.arc(x + center, y + center, cell * 0.16, 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
}

/// Draws `text` centered on `(x, y)`, rotated by `angle` radians.
///
/// Used for the frame inlay so the north-edge labels are readable by
/// the player sitting opposite.
fn text_centered(cr: &gtk4::cairo::Context, text: &str, x: f64, y: f64, angle: f64) {
    if let Ok(extents) = cr.text_extents(text) {
        let _ = cr.save();
        cr.translate(x, y);
        cr.rotate(angle);
        cr.move_to(-(extents.x_bearing() + extents.width() / 2.0), -(extents.y_bearing() + extents.height() / 2.0));
        let _ = cr.show_text(text);
        let _ = cr.restore();
    }
}
