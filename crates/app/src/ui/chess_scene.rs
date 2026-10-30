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
use tangibles_chess_core::Piece;

use crate::theme::TextureLayout;
use crate::theme::ThemePalette;
use crate::ui::parking_strip::ParkingStrip;

/// Everything the scene needs to draw one frame.
#[derive(Debug)]
pub(crate) struct SceneState {
    /// Current board position; `None` before a game starts.
    pub(crate) position: Option<Chess>,
    /// Whether rendered piece glyphs are drawn at all (off in `Tangibles`).
    pub(crate) render_pieces: bool,
    /// Whether captured-piece parking columns are drawn beside the
    /// board (physical modes only).
    pub(crate) parking_zones: bool,
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
    /// Briefly flashed squares explaining a rejected move.
    pub(crate) illegal: Option<IllegalFlash>,
    /// Physical/logical discrepancy markers (`TangiblesRendered`).
    pub(crate) discrepant: Vec<Square>,
    /// West parking strip cells (occupancy + rendered captured piece).
    pub(crate) cap_west: ParkingStrip,
    /// East parking strip cells, mirroring `cap_west`.
    pub(crate) cap_east: ParkingStrip,
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            position: None,
            render_pieces: true,
            parking_zones: false,
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
            illegal: None,
            discrepant: Vec::new(),
            cap_west: ParkingStrip::default(),
            cap_east: ParkingStrip::default(),
        }
    }
}

/// Tap callback receiving a scene-space square.
type SquareHandler = Rc<dyn Fn(Square)>;

/// A rejected move's flash: the rejected target plus, when the naive
/// move would have left the own king in check, the king and its
/// attackers.
#[derive(Debug)]
pub(crate) struct IllegalFlash {
    /// Flashed squares with their meaning.
    pub squares: Vec<(Square, FlashRole)>,
    /// Generation counter so a stale expiry timer cannot clear a newer
    /// flash.
    pub generation: u64,
}

/// What a flashed square explains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlashRole {
    /// The rejected destination.
    Target,
    /// The own king left in check.
    King,
    /// An attacker giving that check.
    Attacker,
}

/// The drawable, tappable board.
///
/// Renders the board in *scene space* (always `a8` top-left .. `h1`
/// bottom-right); the `RotationWidget` handles the physical orientation.
/// All input arrives already un-rotated in scene coordinates.
mod imp {
    use super::*;
    use gtk4::{IconLookupFlags, TextDirection};

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

            // Captured-piece parking columns (physical modes): one cell
            // wide, board height, flush beside the board - matching the
            // `cap-*` daemon fields registered for the same geometry.
            if state.parking_zones {
                let parking = snapshot.append_cairo(&graphene::Rect::new((bx - cell) as f32, by as f32, (bs + 2.0 * cell) as f32, bs as f32));
                if bx >= cell {
                    parking_strip(&parking, bx - cell, by, cell, &state.cap_west, &palette);
                }
                if bx + bs + cell <= width {
                    parking_strip(&parking, bx + bs, by, cell, &state.cap_east, &palette);
                }
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

                // Per-square material (wood inlay): every square samples
                // its own crop of the texture, so the grain ends at the
                // square's edge like a real piece of wood.
                if palette.texture_layout == TextureLayout::PerSquare
                    && let Some(surface) = palette.texture.and_then(texture_surface)
                {
                    let offset_x = pseudo_random(scene as u32, 3) * f64::from(surface.width());
                    let offset_y = pseudo_random(scene as u32, 11) * f64::from(surface.height());
                    let pattern = gtk4::cairo::SurfacePattern::create(&surface);
                    pattern.set_extend(gtk4::cairo::Extend::Repeat);
                    let mut matrix = gtk4::cairo::Matrix::identity();
                    matrix.translate(offset_x - x, offset_y - y);
                    pattern.set_matrix(matrix);
                    squares.set_operator(gtk4::cairo::Operator::Multiply);
                    let _ = squares.set_source(&pattern);
                    squares.rectangle(x, y, cell, cell);
                    let _ = squares.fill();
                    squares.set_operator(gtk4::cairo::Operator::Over);
                }

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
                if let Some(flash) = &state.illegal {
                    for (flashed, role) in &flash.squares {
                        if *flashed == square {
                            match role {
                                FlashRole::Target => fill_highlight(&squares, x, y, cell, palette.check, 0.55),
                                FlashRole::King => {
                                    fill_highlight(&squares, x, y, cell, palette.check, 0.45);
                                    ring_highlight(&squares, x, y, cell, palette.check);
                                }
                                FlashRole::Attacker => ring_highlight(&squares, x, y, cell, palette.check),
                            }
                        }
                    }
                }
                if state.lifted == Some(square) {
                    fill_highlight(&squares, x, y, cell, palette.selection, 0.18);
                }
                if state.ghost.is_some_and(|(_, to)| to == square) {
                    ring_highlight(&squares, x, y, cell, palette.ceremony);
                }
            }

            // Material texture: one continuous luminance image multiplied
            // over the whole board, so the grain runs seamlessly across
            // square boundaries while each square keeps its base tint.
            if palette.texture_layout == TextureLayout::BoardWide
                && let Some(surface) = palette.texture.and_then(texture_surface)
            {
                // Pattern space is surface pixels: scale it so one image
                // spans `repeat` px on the board, anchored at the board
                // corner - the repeat then tiles seamlessly.
                let repeat = cell * 2.4;
                let scale = f64::from(surface.width()) / repeat;
                let mut matrix = gtk4::cairo::Matrix::identity();
                matrix.translate(-bx, -by);
                matrix.scale(scale, scale);
                let pattern = gtk4::cairo::SurfacePattern::create(&surface);
                pattern.set_matrix(matrix);
                // Surface patterns default to Extend::None - without
                // Repeat the texture would only cover one tile and
                // multiply the rest of the board by transparent black.
                pattern.set_extend(gtk4::cairo::Extend::Repeat);
                squares.set_operator(gtk4::cairo::Operator::Multiply);
                let _ = squares.set_source(&pattern);
                squares.rectangle(bx, by, bs, bs);
                let _ = squares.fill();
                squares.set_operator(gtk4::cairo::Operator::Over);
            }

            // Piece layer.
            if state.render_pieces {
                let scale = self.obj().scale_factor();
                let paint_piece = |piece: Piece, x: f64, y: f64| {
                    let name = piece.icon_name();
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
                                    TextDirection::None,
                                    IconLookupFlags::empty(),
                                )
                            })
                            .clone()
                    };
                    snapshot.save();
                    snapshot.translate(&graphene::Point::new(x as f32, y as f32));
                    if piece.side == tangibles_chess_core::Side::Black {
                        // The north player faces their own pieces: flip
                        // black glyphs around the cell centre so they are
                        // readable upside down.
                        snapshot.translate(&graphene::Point::new((cell / 2.0) as f32, (cell / 2.0) as f32));
                        snapshot.rotate(180.0);
                        snapshot.translate(&graphene::Point::new((-cell / 2.0) as f32, (-cell / 2.0) as f32));
                    }
                    paintable.snapshot(snapshot, cell, cell);
                    snapshot.restore();
                };
                if let Some(position) = &state.position {
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
                        paint_piece(Piece::from(piece), x, y);
                    }
                }
                // Parked captured pieces on the cap strips
                // (`TangiblesRendered`): the cells sit beside the
                // board, each the size of a board square.
                if state.parking_zones {
                    for (strip, strip_x) in [(state.cap_west, bx - cell), (state.cap_east, bx + bs)] {
                        for (row, cell_state) in strip.cells().iter().enumerate() {
                            if let Some(piece) = cell_state.piece {
                                paint_piece(piece, strip_x, by + row as f64 * cell);
                            }
                        }
                    }
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

    /// The board rectangle converted to screen millimeters.
    ///
    /// Maps all four board corners through the widget transform into
    /// window coordinates and takes the axis-aligned bounding box - this
    /// stays correct while the surface sits rotated inside the
    /// `RotationWidget`, where a naive `(0,0)` lookup would report the
    /// corner's transformed position instead of the board's coverage.
    pub fn board_rect_mm(&self, mm_per_px: f64) -> Option<(f64, f64, f64)> {
        let (bx, by, bs) = self.board_rect()?;
        let root = self.root()?;
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for (px, py) in [(bx, by), (bx + bs, by), (bx, by + bs), (bx + bs, by + bs)] {
            let point = self.compute_point(&root, &graphene::Point::new(px as f32, py as f32))?;
            min_x = min_x.min(point.x() as f64);
            min_y = min_y.min(point.y() as f64);
            max_x = max_x.max(point.x() as f64);
            max_y = max_y.max(point.y() as f64);
        }
        let side = ((max_x - min_x).min(max_y - min_y)) * mm_per_px;
        Some((min_x * mm_per_px, min_y * mm_per_px, side))
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

/// One captured-piece parking column: eight muted cells so the strip
/// reads as a place to park pieces rather than as board squares.
/// Occupied cells are highlighted so a parked marker is visible at a
/// glance.
fn parking_strip(cr: &gtk4::cairo::Context, x: f64, y: f64, cell: f64, strip: &ParkingStrip, palette: &ThemePalette) {
    for (index, cell_state) in strip.cells().iter().enumerate() {
        let yy = y + index as f64 * cell;
        if cell_state.occupied {
            cr.set_source_rgba(palette.target.0, palette.target.1, palette.target.2, 0.45);
        } else {
            cr.set_source_rgba(palette.dark.0, palette.dark.1, palette.dark.2, 0.30);
        }
        cr.rectangle(x, yy, cell, cell);
        let _ = cr.fill();
        cr.set_source_rgba(palette.frame.0, palette.frame.1, palette.frame.2, 0.7);
        cr.set_line_width(1.0);
        cr.rectangle(x + 0.5, yy + 0.5, cell - 1.0, cell - 1.0);
        let _ = cr.stroke();
    }
}

/// Deterministic pseudo-random value in `0.0..1.0` for a square+key
/// pair - keeps per-square texture crops stable across repaints.
fn pseudo_random(seed: u32, key: u32) -> f64 {
    let hash = seed.wrapping_mul(2_654_435_761).wrapping_add(key.wrapping_mul(40_503));
    f64::from(hash % 1000) / 1000.0
}

/// The embedded luminance texture for `name`, decoded once (cairo
/// surfaces are single-threaded, so the cache is thread-local like the
/// paintable cache).
fn texture_surface(name: &str) -> Option<gtk4::cairo::ImageSurface> {
    thread_local! {
        static SURFACES: std::cell::RefCell<std::collections::HashMap<&'static str, Option<gtk4::cairo::ImageSurface>>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    SURFACES.with(|surfaces| {
        let mut surfaces = surfaces.borrow_mut();
        surfaces
            .entry(match name {
                "wood" => "wood",
                "slate" => "slate",
                "fabric" => "fabric",
                "marble" => "marble",
                _ => return None,
            })
            .or_insert_with(|| {
                let bytes: &[u8] = match name {
                    "wood" => include_bytes!("../../resources/textures/wood.png"),
                    "slate" => include_bytes!("../../resources/textures/slate.png"),
                    "fabric" => include_bytes!("../../resources/textures/fabric.png"),
                    "marble" => include_bytes!("../../resources/textures/marble.png"),
                    _ => return None,
                };
                // gdk decodes the PNG; the pixels are then copied into a
                // cairo image surface (channel order is irrelevant - the
                // texture is grayscale, so R == G == B either way).
                decode_texture(bytes)
            })
            .clone()
    })
}

/// Decodes embedded PNG bytes into a cairo image surface via
/// [`gtk4::gdk::Texture`].
fn decode_texture(bytes: &'static [u8]) -> Option<gtk4::cairo::ImageSurface> {
    let texture = gtk4::gdk::Texture::from_bytes(&gtk4::glib::Bytes::from_static(bytes)).ok()?;
    let (width, height) = (texture.width() as usize, texture.height() as usize);
    // PNG textures are 4-channel RGBA data; channel order does not
    // matter because the grayscale texture has R == G == B either way.
    let stride = width * 4;
    let mut pixels = vec![0u8; height * stride];
    texture.download(&mut pixels, stride);
    let mut surface = gtk4::cairo::ImageSurface::create(gtk4::cairo::Format::ARgb32, texture.width(), texture.height()).ok()?;
    let surface_stride = surface.stride() as usize;
    {
        let mut surface_data = surface.data().ok()?;
        for row in 0..height {
            let (src, dst) = (row * stride, row * surface_stride);
            surface_data[dst..dst + stride].copy_from_slice(&pixels[src..src + stride]);
        }
    }
    surface.mark_dirty();
    Some(surface)
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

#[cfg(test)]
mod tests {
    use super::decode_texture;

    /// The embedded textures must decode to a valid cairo surface;
    /// otherwise the material layer is silently skipped.
    #[test]
    fn decodes_embedded_textures() {
        let texture = decode_texture(include_bytes!("../../resources/textures/wood.png"));
        assert!(texture.is_some(), "gdk failed to decode the texture");
        if let Some(surface) = texture {
            assert!(surface.width() > 0);
            assert!(surface.height() > 0);
        }
    }

    /// The copied pixels must carry actual texture content - a uniform
    /// surface would make the multiply pass invisible.
    #[test]
    fn texture_pixels_carry_luminance_variation() {
        let Some(mut surface) = decode_texture(include_bytes!("../../resources/textures/wood.png")) else {
            panic!("texture did not decode");
        };
        let (stride, width) = (surface.stride() as usize, surface.width() as usize);
        let height = surface.height() as usize;
        let data = surface.data().expect("surface data");
        let (mut min, mut max) = (255u8, 0u8);
        for row in 0..height {
            for col in 0..width {
                let pixel = data[row * stride + col * 4];
                min = min.min(pixel);
                max = max.max(pixel);
            }
        }
        assert!(max as i32 - min as i32 > 20, "texture is uniform: min {min}, max {max}");
    }
}
