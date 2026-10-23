use tangible_objects_model::Field;
use tangible_objects_model::FieldName;
use tangible_objects_model::FieldSet;
use tangible_objects_model::FieldSetName;
use tangible_objects_model::FrameCount;
use tangible_objects_model::Millimeters;
use tangible_objects_model::Region;

/// Builds the chess [`FieldSet`] from board geometry in screen
/// millimeters.
///
/// Board fields are absolute screen-mm rectangles; confirm zones and
/// captured-piece trays sit just outside the board edges.
pub struct FieldSetBuilder {
    /// Board rectangle in screen millimeters.
    pub board_mm: (f64, f64, f64),
    /// Overall window (screen) size in millimeters.
    pub window_mm: (f64, f64),
    /// Depth of the confirm/tray zones alongside the board, in mm.
    pub zone_mm: f64,
}

impl FieldSetBuilder {
    /// Creates a builder; `board_mm` is (x, y, side) of the board square
    /// and `window_mm` the full window size in millimeters.
    pub fn new(board_mm: (f64, f64, f64), window_mm: (f64, f64)) -> Self {
        Self {
            board_mm,
            window_mm,
            zone_mm: 40.0,
        }
    }

    /// One occupancy cell named `name` with absolute bounds in mm.
    fn field(name: &str, x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Field {
        Field {
            name: FieldName::new(name),
            region: Region {
                x_min: Millimeters::new(x_min),
                y_min: Millimeters::new(y_min),
                x_max: Millimeters::new(x_max),
                y_max: Millimeters::new(y_max),
            },
        }
    }

    /// Builds the full field set: 64 board cells + 2 confirm zones +
    /// 2 captured trays, suppressing marker contacts from the OS.
    ///
    /// Disjointness is structural: the confirm zone and tray on each
    /// side split the free strip between the board edge and the surface
    /// edge without gaps or clamps that could invert a region. A board
    /// that would poke past the surface is shrunk to fit first, since
    /// any overflow row would overlap the degenerate edge zones.
    pub fn build(&self) -> FieldSet {
        let (bx, by, bs) = self.board_mm;
        let (ww, wh) = self.window_mm;
        let bs = bs.min(ww - bx).min(wh - by).max(0.0);
        let cell = bs / 8.0;
        let mut fields = Vec::with_capacity(64 + 4);
        for index in 0..64 {
            let (row, col) = (index / 8, index % 8);
            fields.push(Self::field(
                &format!("f{index}"),
                bx + col as f64 * cell,
                by + row as f64 * cell,
                bx + (col + 1) as f64 * cell,
                by + (row + 1) as f64 * cell,
            ));
        }
        let zone = self.zone_mm;
        // South side: confirm strip directly below the board, tray takes
        // the rest down to the surface edge. When the free strip is
        // shorter than `zone`, the tray collapses to the empty region at
        // `wh` - a zero-height region never overlaps.
        let south_top = by + bs + 4.0;
        let south_confirm_h = (wh - south_top).max(0.0).min(zone);
        fields.push(Self::field(crate::ZONE_BTN_SOUTH, bx + bs * 0.35, south_top, bx + bs * 0.65, south_top + south_confirm_h));
        fields.push(Self::field(crate::ZONE_TRAY_SOUTH, bx, (south_top + south_confirm_h).min(wh), bx + bs, wh));
        // North side, mirrored: confirm strip just above the board.
        let north_bottom = (by - 4.0).max(0.0);
        let north_confirm_h = north_bottom.min(zone);
        fields.push(Self::field(
            crate::ZONE_BTN_NORTH,
            bx + bs * 0.35,
            north_bottom - north_confirm_h,
            bx + bs * 0.65,
            north_bottom,
        ));
        fields.push(Self::field(crate::ZONE_TRAY_NORTH, bx, 0.0, bx + bs, north_bottom - north_confirm_h));
        FieldSet {
            name: FieldSetName::new(crate::FIELD_SET_NAME),
            window: None,
            stability: FrameCount::new(2),
            fields,
            suppress: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FieldSetBuilder;

    #[test]
    fn builds_68_fields_with_board_cells() {
        // 800x800mm window, 640mm board centered.
        let set = FieldSetBuilder::new((80.0, 80.0, 640.0), (800.0, 800.0)).build();
        assert_eq!(set.name.as_str(), "chess");
        assert!(set.suppress);
        assert!(set.is_disjoint());
        assert_eq!(set.fields.len(), 68);
        let f0 = set.fields.iter().find(|f| f.name.as_str() == "f0").expect("f0");
        assert_eq!(f0.region.x_min.as_f64(), 80.0);
        assert_eq!(f0.region.y_min.as_f64(), 80.0);
        let f63 = set.fields.iter().find(|f| f.name.as_str() == "f63").expect("f63");
        assert_eq!(f63.region.x_max.as_f64(), 720.0);
        assert_eq!(f63.region.y_max.as_f64(), 720.0);
        for name in ["btn-south", "btn-north", "tray-south", "tray-north"] {
            assert!(set.fields.iter().any(|f| f.name.as_str() == name), "missing {name}");
        }
    }

    #[test]
    fn tight_window_collapses_zones_without_overlap() {
        // The board fills the window edge to edge - the south zones
        // collapse but the set must stay disjoint (the daemon rejects
        // overlapping fields at registration).
        let set = FieldSetBuilder::new((0.0, 0.0, 800.0), (800.0, 800.0)).build();
        assert!(set.is_disjoint());
        let tray = set.fields.iter().find(|f| f.name.as_str() == "tray-south").expect("tray");
        assert_eq!(tray.region.y_min.as_f64(), 800.0);
        assert_eq!(tray.region.y_max.as_f64(), 800.0);
    }

    #[test]
    fn oversized_board_is_clamped_into_the_surface() {
        // A board taller than the window would overlap the collapsed
        // tray line - the builder shrinks it to fit.
        let set = FieldSetBuilder::new((10.0, 5.0, 900.0), (800.0, 400.0)).build();
        assert!(set.is_disjoint());
        let f63 = set.fields.iter().find(|f| f.name.as_str() == "f63").expect("f63");
        assert!(f63.region.y_max.as_f64() <= 400.0);
    }
}
