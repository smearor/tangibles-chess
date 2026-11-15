use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::graphene;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{GestureClick, glib};
use shakmaty::Chess;
use shakmaty::Position;
use shakmaty::Square;
use smearor_commons_color::Color;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Millimeters;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PixelsPerMm;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Vec2;
use smearor_commons_geometry::WidgetSizeExt;
use tangibles_chess_config::BoardTheme;
use tangibles_chess_geometry::FieldIndex;
use tangibles_chess_material::Piece;

use crate::theme::TextureLayout;
use crate::theme::ThemePalette;
use crate::theme::ThemeSlot;
use crate::ui::flash_role::FlashRole;
use crate::ui::illegal_flash::IllegalFlash;
use crate::ui::parking_strip::ParkingStrip;

/// Fraction of the available extent the board fills - the rest stays
/// free for frame and parking strips.
pub(crate) const BOARD_FILL: Fraction = Fraction::new(0.88);

/// Board-frame thickness as a fraction of the board edge - the wood
/// border carrying the coordinate inlay.
const FRAME_MARGIN: Fraction = Fraction::new(0.045);

/// Coordinate-inlay font size as a fraction of the frame margin.
const INLAY_FONT: Fraction = Fraction::new(0.55);

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
    /// Diagnostics overlay: draw field indices, contact counts and raw
    /// touch points (calibration bring-up, FR-UI-13).
    pub(crate) diagnostics: bool,
    /// Live contact count per field (probe mirror, diagnostics).
    pub(crate) diag_counts: BTreeMap<FieldIndex, u32>,
    /// Raw touch positions in scene pixels (diagnostics).
    pub(crate) diag_touches: Vec<Vec2<Pixels>>,
    /// Destination of a buffered move intent awaiting the confirm-zone
    /// press (`OnConfirmZone`).
    pub(crate) pending: Option<Square>,
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
            pending: None,
            diagnostics: false,
            diag_counts: BTreeMap::new(),
            diag_touches: Vec::new(),
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

/// Diagnostics overlay color (field indices, contact counts, raw
/// touch points) - warm red, readable on any board material.
const DIAGNOSTIC: Color<f64> = Color::<f64>::new(0.95, 0.3, 0.2);

/// The drawable, tappable board.
///
/// Renders the board in *scene space* (always `a8` top-left .. `h1`
/// bottom-right); the `RotationWidget` handles the physical orientation.
/// All input arrives already un-rotated in scene coordinates.
mod imp {
    use super::*;
    use cairo::{Matrix, Operator, SurfacePattern};
    use gtk4::TextDirection;
    use gtk4::{IconLookupFlags, IconPaintable, Orientation};

    /// Implementation state of [`ChessScene`].
    #[derive(Default)]
    pub struct ChessSceneImp {
        /// Render state.
        pub(crate) state: RefCell<SceneState>,
        /// Tap handler receiving scene-space squares.
        pub on_square: RefCell<Option<SquareHandler>>,
        /// Icon paintable cache (icon name → paintable).
        pub paintables: RefCell<HashMap<String, IconPaintable>>,
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
        fn measure(&self, _orientation: Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (200, 400, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let extent = self.obj().allocated_size().to_f64().with_unit::<Pixels>();
            let Some(board) = board_rect(extent) else {
                return;
            };
            let cell = board.size.x_unit() / 8.0;
            let state = self.state.borrow();
            let palette = ThemePalette::of(state.theme);
            let light = palette.color(ThemeSlot::Light);
            let dark = palette.color(ThemeSlot::Dark);
            let frame_color = palette.color(ThemeSlot::Frame);

            // Board frame (the material border with coordinate inlay).
            // append_cairo clips to the given bounds but the context
            // itself draws in widget coordinates.
            let margin = board.size.x_unit() * FRAME_MARGIN;
            let frame_rect = board.outset(margin);
            let frame = snapshot.append_cairo(&frame_rect.to_f32().into());
            frame_color.apply(&frame);
            frame.rectangle(frame_rect.min.x, frame_rect.min.y, frame_rect.size.x, frame_rect.size.y);
            let _ = frame.fill();

            // Coordinate inlay engraved into the frame: ranks run on
            // both vertical edges (left upright for the south player,
            // right rotated 180° for the north player); files on the
            // bottom edge upright and on the top edge rotated, so each
            // player reads the labels on their own side.
            let muted = light.lerp(&frame_color, 0.5).brighten(0.08);
            muted.apply(&frame);
            let half_margin = margin / 2.0;
            let font = margin * INLAY_FONT;
            frame.set_font_size(font.as_f64());
            for row in 0..8usize {
                let rank = (8 - row).to_string();
                let cy = board.min.y + (row as f64 + 0.5) * cell.as_f64();
                text_centered(&frame, &rank, Vec2::<Pixels>::new(board.min.x - half_margin.as_f64(), cy), Radians::ZERO);
                text_centered(&frame, &rank, Vec2::new(board.right() + half_margin.as_f64(), cy), Radians::HALF_TURN);
            }
            for col in 0..8usize {
                let letter = ((b'a' + col as u8) as char).to_string();
                let cx = board.min.x + (col as f64 + 0.5) * cell.as_f64();
                text_centered(&frame, &letter, Vec2::new(cx, board.bottom() + half_margin.as_f64()), Radians::ZERO);
                text_centered(&frame, &letter, Vec2::new(cx, board.min.y - half_margin.as_f64()), Radians::HALF_TURN);
            }

            // Captured-piece parking columns (physical modes): one cell
            // wide, board height, flush beside the board - matching the
            // `cap-*` daemon fields registered for the same geometry.
            let column_size = Vec2::<Pixels>::from_px(cell, board.size.y_unit());
            let west = Rect::new(board.min - Vec2::<Pixels>::from_px(cell, Pixels::new(0.0)), column_size);
            let east = Rect::new(Vec2::new(board.right(), board.min.y), column_size);
            if state.parking_zones {
                let strip_area = Rect::new(west.min, Vec2::new(board.size.x + 2.0 * cell.as_f64(), board.size.y));
                let parking = snapshot.append_cairo(&strip_area.to_f32().into());
                if board.min.x >= cell.as_f64() {
                    parking_strip(&parking, west, &state.cap_west, &palette);
                }
                if board.right() + cell.as_f64() <= extent.x {
                    parking_strip(&parking, east, &state.cap_east, &palette);
                }
            }

            let squares = snapshot.append_cairo(&board.to_f32().into());
            for scene in 0..64usize {
                let (row, col) = (scene / 8, scene % 8);
                let square = scene_square(scene);
                let square_px = square_rect(board, scene);
                let base = if (row + col) % 2 == 0 { light } else { dark };
                base.apply(&squares);
                squares.rectangle(square_px.min.x, square_px.min.y, square_px.size.x, square_px.size.y);
                let _ = squares.fill();

                // Per-square material (wood inlay): every square samples
                // its own crop of the texture, so the grain ends at the
                // square's edge like a real piece of wood.
                if palette.texture_layout == TextureLayout::PerSquare
                    && let Some(surface) = palette.texture.and_then(texture_surface)
                {
                    let offset_x = pseudo_random(scene as u32, 3) * f64::from(surface.width());
                    let offset_y = pseudo_random(scene as u32, 11) * f64::from(surface.height());
                    let pattern = SurfacePattern::create(&surface);
                    pattern.set_extend(gtk4::cairo::Extend::Repeat);
                    let mut matrix = Matrix::identity();
                    matrix.translate(offset_x - square_px.min.x, offset_y - square_px.min.y);
                    pattern.set_matrix(matrix);
                    squares.set_operator(Operator::Multiply);
                    let _ = squares.set_source(&pattern);
                    squares.rectangle(square_px.min.x, square_px.min.y, square_px.size.x, square_px.size.y);
                    let _ = squares.fill();
                    squares.set_operator(Operator::Over);
                }

                // Layered highlights.
                if state.last_move.is_some_and(|(from, to)| from == square || to == square) {
                    fill_highlight(&squares, square_px, palette.color(ThemeSlot::LastMove), 0.28);
                }
                if state.ceremony.contains(&square) {
                    fill_highlight(&squares, square_px, palette.color(ThemeSlot::Ceremony), 0.35);
                }
                if state.missing.contains(&square) {
                    ring_highlight(&squares, square_px, palette.color(ThemeSlot::Check));
                }
                if state.foreign.contains(&square) || state.discrepant.contains(&square) {
                    ring_highlight(&squares, square_px, palette.color(ThemeSlot::Target));
                }
                if state.selected == Some(square) {
                    fill_highlight(&squares, square_px, palette.color(ThemeSlot::Selection), 0.30);
                }
                if state.legal_targets.contains(&square) {
                    target_dot(&squares, square_px, palette.color(ThemeSlot::Target));
                }
                if state.check == Some(square) {
                    fill_highlight(&squares, square_px, palette.color(ThemeSlot::Check), 0.45);
                }
                if let Some(flash) = &state.illegal {
                    for (flashed, role) in &flash.squares {
                        if *flashed == square {
                            match role {
                                FlashRole::Target => fill_highlight(&squares, square_px, palette.color(ThemeSlot::Check), 0.55),
                                FlashRole::King => {
                                    fill_highlight(&squares, square_px, palette.color(ThemeSlot::Check), 0.45);
                                    ring_highlight(&squares, square_px, palette.color(ThemeSlot::Check));
                                }
                                FlashRole::Attacker => ring_highlight(&squares, square_px, palette.color(ThemeSlot::Check)),
                            }
                        }
                    }
                }
                if state.lifted == Some(square) {
                    fill_highlight(&squares, square_px, palette.color(ThemeSlot::Selection), 0.18);
                }
                if state.pending == Some(square) {
                    fill_highlight(&squares, square_px, palette.color(ThemeSlot::Selection), 0.28);
                    ring_highlight(&squares, square_px, palette.color(ThemeSlot::Selection));
                }
                if state.ghost.is_some_and(|(_, to)| to == square) {
                    ring_highlight(&squares, square_px, palette.color(ThemeSlot::Ceremony));
                }

                // Diagnostics: field index (top-left) and live contact
                // count (bottom-right), one per cell.
                if state.diagnostics {
                    let font = cell.as_f64() * 0.16;
                    squares.set_font_size(font);
                    DIAGNOSTIC.apply_alpha(&squares, 0.8);
                    squares.move_to(square_px.min.x + cell.as_f64() * 0.06, square_px.min.y + font * 1.2);
                    let _ = squares.show_text(&format!("f{scene}"));
                    if let Some(count) = FieldIndex::new(scene).and_then(|index| state.diag_counts.get(&index)) {
                        let label = count.to_string();
                        if let Ok(extents) = squares.text_extents(&label) {
                            squares.move_to(square_px.right() - extents.width() - cell.as_f64() * 0.06, square_px.bottom() - font * 0.4);
                            let _ = squares.show_text(&label);
                        }
                    }
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
                let repeat = cell.as_f64() * 2.4;
                let scale = f64::from(surface.width()) / repeat;
                let mut matrix = gtk4::cairo::Matrix::identity();
                matrix.translate(-board.min.x, -board.min.y);
                matrix.scale(scale, scale);
                let pattern = gtk4::cairo::SurfacePattern::create(&surface);
                pattern.set_matrix(matrix);
                // Surface patterns default to Extend::None - without
                // Repeat the texture would only cover one tile and
                // multiply the rest of the board by transparent black.
                pattern.set_extend(gtk4::cairo::Extend::Repeat);
                squares.set_operator(gtk4::cairo::Operator::Multiply);
                let _ = squares.set_source(&pattern);
                squares.rectangle(board.min.x, board.min.y, board.size.x, board.size.y);
                let _ = squares.fill();
                squares.set_operator(gtk4::cairo::Operator::Over);
            }

            // Diagnostics: raw touch points as dots on the board area.
            if state.diagnostics {
                DIAGNOSTIC.apply_alpha(&squares, 0.5);
                for touch in &state.diag_touches {
                    squares.arc(touch.x, touch.y, cell.as_f64() * 0.2, 0.0, std::f64::consts::TAU);
                    let _ = squares.fill();
                }
            }

            // Piece layer.
            if state.render_pieces {
                let scale = self.obj().scale_factor();
                let paint_piece = |piece: Piece, at: Vec2<Pixels>| {
                    let name = piece.icon_name();
                    let paintable = {
                        let mut cache = self.paintables.borrow_mut();
                        cache
                            .entry(name.to_string())
                            .or_insert_with(|| {
                                gtk4::IconTheme::for_display(&self.obj().display()).lookup_icon(
                                    name,
                                    &[],
                                    cell.as_f64() as i32,
                                    scale,
                                    TextDirection::None,
                                    IconLookupFlags::empty(),
                                )
                            })
                            .clone()
                    };
                    let half_cell = cell.as_f64() as f32 / 2.0;
                    snapshot.save();
                    snapshot.translate(&at.to_f32().into());
                    if piece.side == tangibles_chess_material::Side::Black {
                        // The north player faces their own pieces: flip
                        // black glyphs around the cell centre so they are
                        // readable upside down.
                        snapshot.translate(&graphene::Point::new(half_cell, half_cell));
                        snapshot.rotate(180.0);
                        snapshot.translate(&graphene::Point::new(-half_cell, -half_cell));
                    }
                    paintable.snapshot(snapshot, cell.as_f64(), cell.as_f64());
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
                        paint_piece(Piece::from(piece), square_rect(board, square_scene(square)).min);
                    }
                }
                // Parked captured pieces on the cap strips
                // (`TangiblesRendered`): the cells sit beside the
                // board, each the size of a board square.
                if state.parking_zones {
                    for (strip, column) in [(state.cap_west, west), (state.cap_east, east)] {
                        for (row, cell_state) in strip.cells().iter().enumerate() {
                            if let Some(piece) = cell_state.piece {
                                paint_piece(piece, column.min + Vec2::<Pixels>::from_px(Pixels::new(0.0), cell * row as f64));
                            }
                        }
                    }
                }
            }

            // Capture affordance on occupied legal targets: a ring on
            // top of the defending piece - the plain `target_dot` in
            // the square layer paints *under* pieces and is invisible
            // on capture squares.
            if let Some(position) = &state.position {
                let captures: Vec<Square> = state
                    .legal_targets
                    .iter()
                    .copied()
                    .filter(|square| position.board().piece_at(*square).is_some())
                    .collect();
                if !captures.is_empty() {
                    let ring = snapshot.append_cairo(&board.to_f32().into());
                    for square in captures {
                        ring_highlight(&ring, square_rect(board, square_scene(square)), palette.color(ThemeSlot::Target));
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
        let click = GestureClick::new();
        click.connect_released(glib::clone!(
            #[weak]
            scene,
            move |_, _n, x, y| {
                if let Some(square) = scene.point_to_square(Vec2::<Pixels>::new(x, y))
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
    pub fn board_rect(&self) -> Option<Rect<Pixels>> {
        board_rect(Vec2::<Pixels>::new(self.width() as f64, self.height() as f64))
    }

    /// The board rectangle converted to screen millimeters.
    ///
    /// Maps all four board corners through the widget transform into
    /// window coordinates and takes the axis-aligned bounding box - this
    /// stays correct while the surface sits rotated inside the
    /// `RotationWidget`, where a naive `(0,0)` lookup would report the
    /// corner's transformed position instead of the board's coverage.
    pub fn board_rect_mm(&self, scale: PixelsPerMm) -> Option<Rect<Millimeters>> {
        let board = self.board_rect()?;
        let root = self.root()?;
        let mut min = Vec2::<Pixels>::splat(Pixels::new(f64::MAX));
        let mut max = Vec2::<Pixels>::splat(Pixels::new(f64::MIN));
        for corner in board.corners() {
            let point = self.compute_point(&root, &graphene::Point::new(corner.x as f32, corner.y as f32))?;
            min = Vec2::new(min.x.min(f64::from(point.x())), min.y.min(f64::from(point.y())));
            max = Vec2::new(max.x.max(f64::from(point.x())), max.y.max(f64::from(point.y())));
        }
        Some(Rect::new(scale.unscale_vec(min), scale.unscale_vec(max - min)))
    }

    /// Maps a scene-space point to a square.
    pub fn point_to_square(&self, point: Vec2<Pixels>) -> Option<Square> {
        let board = self.board_rect()?;
        // Half-open bounds: `Rect::contains` is edge-inclusive, but a
        // tap exactly on the far edge belongs to no square.
        if !board.contains(&point) || point.x >= board.right() || point.y >= board.bottom() {
            return None;
        }
        let cell = board.size.x_unit() / 8.0;
        let col = ((point.x - board.min.x) / cell.as_f64()) as usize;
        let row = ((point.y - board.min.y) / cell.as_f64()) as usize;
        Some(scene_square(row * 8 + col))
    }
}

/// Centered board rect inside `size` (scene-local pixels).
fn board_rect(size: Vec2<Pixels>) -> Option<Rect<Pixels>> {
    let side = Pixels::new(size.x.min(size.y)) * BOARD_FILL;
    (side.as_f64() > 0.0).then(|| Rect::centered(size.half(), Vec2::splat(side)))
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
fn parking_strip(cr: &gtk4::cairo::Context, column: Rect<Pixels>, strip: &ParkingStrip, palette: &ThemePalette) {
    let cell = column.size.x_unit();
    for (index, cell_state) in strip.cells().iter().enumerate() {
        let cell_rect = Rect::new(column.min + Vec2::<Pixels>::from_px(Pixels::new(0.0), cell * index as f64), Vec2::splat(cell));
        if cell_state.occupied {
            palette.color(ThemeSlot::Target).apply_alpha(cr, 0.45);
        } else {
            palette.color(ThemeSlot::Dark).apply_alpha(cr, 0.30);
        }
        cr.rectangle(cell_rect.min.x, cell_rect.min.y, cell_rect.size.x, cell_rect.size.y);
        let _ = cr.fill();
        palette.color(ThemeSlot::Frame).apply_alpha(cr, 0.7);
        cr.set_line_width(1.0);
        cr.rectangle(cell_rect.min.x + 0.5, cell_rect.min.y + 0.5, cell.as_f64() - 1.0, cell.as_f64() - 1.0);
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

/// The scene-local pixel rect of board square `scene` (0 = a8,
/// row-major).
fn square_rect(board: Rect<Pixels>, scene: usize) -> Rect<Pixels> {
    let cell = board.size.x_unit() / 8.0;
    Rect::new(board.min + Vec2::<Pixels>::from_px(cell * (scene % 8) as f64, cell * (scene / 8) as f64), Vec2::splat(cell))
}

/// Soft filled highlight over one cell.
fn fill_highlight(cr: &gtk4::cairo::Context, square: Rect<Pixels>, color: Color<f64>, alpha: f64) {
    color.apply_alpha(cr, alpha);
    cr.rectangle(square.min.x, square.min.y, square.size.x, square.size.y);
    let _ = cr.fill();
}

/// Ring outline inside a cell (missing/foreign/ghost).
fn ring_highlight(cr: &gtk4::cairo::Context, square: Rect<Pixels>, color: Color<f64>) {
    color.apply_alpha(cr, 0.85);
    let cell = square.size.x_unit();
    cr.set_line_width((cell * 0.06).as_f64().max(2.0));
    let ring = square.inset(cell * 0.08);
    cr.rectangle(ring.min.x, ring.min.y, ring.size.x, ring.size.y);
    let _ = cr.stroke();
}

/// Legal-target dot.
fn target_dot(cr: &gtk4::cairo::Context, square: Rect<Pixels>, color: Color<f64>) {
    color.apply_alpha(cr, 0.55);
    let center = square.center();
    cr.arc(center.x, center.y, (square.size.x_unit() * 0.16).as_f64(), 0.0, std::f64::consts::TAU);
    let _ = cr.fill();
}

/// Draws `text` centered on `at`, rotated by `angle`.
///
/// Used for the frame inlay so the north-edge labels are readable by
/// the player sitting opposite.
fn text_centered(cr: &gtk4::cairo::Context, text: &str, at: Vec2<Pixels>, angle: Radians) {
    if let Ok(extents) = cr.text_extents(text) {
        let _ = cr.save();
        cr.translate(at.x, at.y);
        cr.rotate(angle.as_f64());
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
