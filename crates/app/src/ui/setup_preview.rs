//! Miniature visual previews drawn inside the setup screen's option
//! buttons: board material, seat edge and commit mode each get a small
//! schematic `DrawingArea` instead of a text-only drop-down entry.

use gtk4::cairo::Context;
use gtk4::prelude::*;
use tangibles_chess_core::BoardTheme;
use tangibles_chess_core::CommitMode;
use tangibles_chess_core::OpponentKind;
use tangibles_chess_core::TangibleMode;

use crate::theme::ThemePalette;

/// Edge length of the square preview drawings (px).
const PREVIEW_SIZE: i32 = 88;

/// RGB triple used by the cairo helpers below.
type Rgb = (f64, f64, f64);

/// Muted neutral board colors for the schematic previews (orientation
/// and commit mode are drawn theme-agnostic).
const NEUTRAL_LIGHT: Rgb = (0.82, 0.82, 0.80);
const NEUTRAL_DARK: Rgb = (0.42, 0.44, 0.47);
const NEUTRAL_FRAME: Rgb = (0.30, 0.30, 0.30);

/// Warm highlight (targets, check marks).
const ACCENT: Rgb = (0.98, 0.80, 0.30);
/// Light piece fill.
const WHITE_PIECE: Rgb = (0.95, 0.95, 0.94);
/// Dark piece fill.
const BLACK_PIECE: Rgb = (0.16, 0.16, 0.17);
/// Confirm check mark.
const OK_GREEN: Rgb = (0.35, 0.75, 0.40);

/// Preview of a board theme: the real material palette rendered as a
/// miniature 8x8 board.
pub fn theme_preview(theme: BoardTheme) -> gtk4::DrawingArea {
    preview_area(move |cr, width, height| {
        let palette = ThemePalette::of(theme);
        let side = width.min(height) * 0.86;
        paint_board(cr, (width - side) / 2.0, (height - side) / 2.0, side, palette.light, palette.dark, palette.frame);
    })
}

/// Preview of a seat edge: neutral board with a white seat marker on the
/// viewer's near edge (`white_near`) or the far edge.
pub fn orientation_preview(white_near: bool) -> gtk4::DrawingArea {
    preview_area(move |cr, width, height| {
        let side = width.min(height) * 0.58;
        paint_board(cr, (width - side) / 2.0, (height - side) / 2.0, side, NEUTRAL_LIGHT, NEUTRAL_DARK, NEUTRAL_FRAME);
        let radius = width * 0.085;
        let center_x = width / 2.0;
        let near_y = height - radius * 1.5;
        let far_y = radius * 1.5;
        let (white_y, black_y) = if white_near { (near_y, far_y) } else { (far_y, near_y) };
        disc(cr, center_x, black_y, radius, BLACK_PIECE);
        disc(cr, center_x, white_y, radius, WHITE_PIECE);
        ring(cr, center_x, white_y, radius, BLACK_PIECE);
    })
}

/// Preview of a commit mode: `OnPlace` shows the move committed directly
/// on the target square, `OnConfirmZone` shows the pending piece plus the
/// separate clock zone the marker must be placed on.
pub fn commit_preview(mode: CommitMode) -> gtk4::DrawingArea {
    preview_area(move |cr, width, height| {
        let side = height * 0.64;
        let board_x = width * 0.06;
        let board_y = (height - side) / 2.0;
        paint_board(cr, board_x, board_y, side, NEUTRAL_LIGHT, NEUTRAL_DARK, NEUTRAL_FRAME);
        let cell = side / 8.0;
        let radius = cell * 0.42;
        // Origin square (e2) keeps a faint ghost, target square (e4)
        // carries the piece.
        let origin_x = board_x + 4.5 * cell;
        let origin_y = board_y + 6.5 * cell;
        let target_x = board_x + 4.5 * cell;
        let target_y = board_y + 4.5 * cell;
        ring(cr, origin_x, origin_y, radius, NEUTRAL_DARK);
        match mode {
            CommitMode::OnPlace => {
                disc(cr, target_x, target_y, radius, WHITE_PIECE);
                check_mark(cr, target_x + cell * 0.55, target_y - cell * 0.55, radius * 0.9);
            }
            CommitMode::OnConfirmZone => {
                ring(cr, target_x, target_y, radius, WHITE_PIECE);
                // Clock zone: outlined pill beside the board with the
                // piece inside.
                let zone_x = width * 0.76;
                let zone_w = width * 0.20;
                let zone_h = side * 0.8;
                let zone_y = (height - zone_h) / 2.0;
                rounded_rect(cr, zone_x, zone_y, zone_w, zone_h, zone_w / 2.0);
                cr.set_source_rgb(ACCENT.0, ACCENT.1, ACCENT.2);
                let _ = cr.stroke();
                disc(cr, zone_x + zone_w / 2.0, height / 2.0, radius, WHITE_PIECE);
                arrow(cr, target_x + cell, target_y, zone_x - radius * 0.4, height / 2.0);
            }
        }
    })
}

/// Preview of an input mode: `NoTangibles` shows a rendered piece plus a
/// touch target, `Tangibles` a physical marker with light guidance only,
/// `TangiblesRendered` physical marker plus rendered control layer.
pub fn input_preview(mode: TangibleMode) -> gtk4::DrawingArea {
    preview_area(move |cr, width, height| {
        let side = width.min(height) * 0.72;
        let board_x = (width - side) / 2.0;
        let board_y = (height - side) / 2.0;
        paint_board(cr, board_x, board_y, side, NEUTRAL_LIGHT, NEUTRAL_DARK, NEUTRAL_FRAME);
        let cell = side / 8.0;
        let radius = cell * 0.42;
        let square_center = |col: usize, row: usize| (board_x + (col as f64 + 0.5) * cell, board_y + (row as f64 + 0.5) * cell);
        match mode {
            TangibleMode::NoTangibles => {
                let (x, y) = square_center(4, 4);
                piece(cr, x, y, radius);
                let (x, y) = square_center(4, 2);
                ring(cr, x, y, radius, ACCENT);
                ring(cr, x, y, radius * 0.55, ACCENT);
            }
            TangibleMode::Tangibles => {
                let (x, y) = square_center(4, 4);
                puck(cr, x, y, radius);
                let (x, y) = square_center(4, 2);
                ring(cr, x, y, radius * 0.7, ACCENT);
            }
            TangibleMode::TangiblesRendered => {
                let (x, y) = square_center(4, 4);
                piece(cr, x - radius * 0.35, y - radius * 0.35, radius);
                puck(cr, x + radius * 0.4, y + radius * 0.4, radius);
            }
        }
    })
}

/// Preview of an opponent kind: a person silhouette for `Human`, a chip
/// for `Engine`.
pub fn opponent_preview(kind: OpponentKind) -> gtk4::DrawingArea {
    preview_area(move |cr, width, height| {
        let center_x = width / 2.0;
        match kind {
            OpponentKind::Human => {
                disc(cr, center_x, height * 0.30, width * 0.14, NEUTRAL_DARK);
                cr.arc(center_x, height * 0.95, width * 0.27, std::f64::consts::PI, std::f64::consts::TAU);
                cr.set_source_rgb(NEUTRAL_DARK.0, NEUTRAL_DARK.1, NEUTRAL_DARK.2);
                let _ = cr.fill();
            }
            OpponentKind::Engine => {
                let size = width * 0.42;
                let x = (width - size) / 2.0;
                let y = (height - size) / 2.0;
                let pin_length = width * 0.06;
                cr.set_source_rgb(NEUTRAL_DARK.0, NEUTRAL_DARK.1, NEUTRAL_DARK.2);
                cr.set_line_width(width * 0.026);
                for pin in 0..3 {
                    let offset = size * (0.25 + pin as f64 * 0.25);
                    cr.move_to(x + offset, y);
                    cr.line_to(x + offset, y - pin_length);
                    cr.move_to(x + offset, y + size);
                    cr.line_to(x + offset, y + size + pin_length);
                    cr.move_to(x, y + offset);
                    cr.line_to(x - pin_length, y + offset);
                    cr.move_to(x + size, y + offset);
                    cr.line_to(x + size + pin_length, y + offset);
                }
                let _ = cr.stroke();
                rounded_rect(cr, x, y, size, size, size * 0.14);
                cr.set_source_rgb(NEUTRAL_DARK.0, NEUTRAL_DARK.1, NEUTRAL_DARK.2);
                let _ = cr.fill();
                let inner = size * 0.46;
                rounded_rect(cr, center_x - inner / 2.0, (height - inner) / 2.0, inner, inner, inner * 0.14);
                cr.set_source_rgb(NEUTRAL_LIGHT.0, NEUTRAL_LIGHT.1, NEUTRAL_LIGHT.2);
                let _ = cr.fill();
            }
        }
    })
}

/// A flat rendered piece (light disc with dark outline).
fn piece(cr: &Context, x: f64, y: f64, radius: f64) {
    disc(cr, x, y, radius, WHITE_PIECE);
    ring(cr, x, y, radius, BLACK_PIECE);
}

/// A raised physical marker (puck) with a small shadow and inner ring.
fn puck(cr: &Context, x: f64, y: f64, radius: f64) {
    cr.arc(x + radius * 0.15, y + radius * 0.22, radius, 0.0, std::f64::consts::TAU);
    cr.set_source_rgba(0.0, 0.0, 0.0, 0.25);
    let _ = cr.fill();
    disc(cr, x, y, radius, BLACK_PIECE);
    ring(cr, x, y, radius * 0.6, NEUTRAL_LIGHT);
}

/// A fixed-size `DrawingArea` painting via `draw`.
fn preview_area(draw: impl Fn(&Context, f64, f64) + 'static) -> gtk4::DrawingArea {
    let area = gtk4::DrawingArea::new();
    area.set_content_width(PREVIEW_SIZE);
    area.set_content_height(PREVIEW_SIZE);
    area.add_css_class("setup-preview");
    area.set_draw_func(move |_, cr, width, height| draw(cr, width as f64, height as f64));
    area
}

/// An 8x8 board with a thin frame at `(x, y)` with edge length `side`.
fn paint_board(cr: &Context, x: f64, y: f64, side: f64, light: Rgb, dark: Rgb, frame: Rgb) {
    let margin = side * 0.05;
    cr.set_source_rgb(frame.0, frame.1, frame.2);
    cr.rectangle(x - margin, y - margin, side + 2.0 * margin, side + 2.0 * margin);
    let _ = cr.fill();
    let cell = side / 8.0;
    for row in 0..8usize {
        for col in 0..8usize {
            let color = if (row + col) % 2 == 0 { light } else { dark };
            cr.set_source_rgb(color.0, color.1, color.2);
            cr.rectangle(x + col as f64 * cell, y + row as f64 * cell, cell, cell);
            let _ = cr.fill();
        }
    }
}

/// A filled circle.
fn disc(cr: &Context, x: f64, y: f64, radius: f64, color: Rgb) {
    cr.arc(x, y, radius, 0.0, std::f64::consts::TAU);
    cr.set_source_rgb(color.0, color.1, color.2);
    let _ = cr.fill();
}

/// A circle outline.
fn ring(cr: &Context, x: f64, y: f64, radius: f64, color: Rgb) {
    cr.arc(x, y, radius, 0.0, std::f64::consts::TAU);
    cr.set_source_rgb(color.0, color.1, color.2);
    cr.set_line_width(radius * 0.18);
    let _ = cr.stroke();
}

/// A small green check mark centered on `(x, y)`.
fn check_mark(cr: &Context, x: f64, y: f64, size: f64) {
    cr.set_source_rgb(OK_GREEN.0, OK_GREEN.1, OK_GREEN.2);
    cr.set_line_width(size * 0.32);
    cr.set_line_cap(gtk4::cairo::LineCap::Round);
    cr.move_to(x - size * 0.55, y + size * 0.05);
    cr.line_to(x - size * 0.15, y + size * 0.45);
    cr.line_to(x + size * 0.55, y - size * 0.45);
    let _ = cr.stroke();
}

/// A straight arrow from `(x1, y1)` to `(x2, y2)` with a filled head.
fn arrow(cr: &Context, x1: f64, y1: f64, x2: f64, y2: f64) {
    let angle = (y2 - y1).atan2(x2 - x1);
    let head = (x2 - x1).hypot(y2 - y1).clamp(6.0, 12.0);
    cr.set_source_rgb(ACCENT.0, ACCENT.1, ACCENT.2);
    cr.set_line_width(2.0);
    cr.move_to(x1, y1);
    cr.line_to(x2 - head * angle.cos(), y2 - head * angle.sin());
    let _ = cr.stroke();
    cr.move_to(x2, y2);
    cr.line_to(x2 - head * (angle - 0.5).cos(), y2 - head * (angle - 0.5).sin());
    cr.line_to(x2 - head * (angle + 0.5).cos(), y2 - head * (angle + 0.5).sin());
    let _ = cr.fill();
}

/// Rounded-rectangle path (still needs a fill/stroke call afterwards).
fn rounded_rect(cr: &Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let radius = radius.min(width / 2.0).min(height / 2.0);
    let arc = std::f64::consts::PI / 2.0;
    cr.new_sub_path();
    cr.arc(x + width - radius, y + radius, radius, -arc, 0.0);
    cr.arc(x + width - radius, y + height - radius, radius, 0.0, arc);
    cr.arc(x + radius, y + height - radius, radius, arc, 2.0 * arc);
    cr.arc(x + radius, y + radius, radius, 2.0 * arc, 3.0 * arc);
    cr.close_path();
}
