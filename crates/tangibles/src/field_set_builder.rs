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
    pub fn build(&self) -> FieldSet {
        let (bx, by, bs) = self.board_mm;
        let (_, wh) = self.window_mm;
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
        // Confirm zones: strips just beyond the south/north board edges.
        fields.push(Self::field(crate::ZONE_BTN_SOUTH, bx + bs * 0.35, by + bs + 4.0, bx + bs * 0.65, by + bs + 4.0 + zone));
        fields.push(Self::field(crate::ZONE_BTN_NORTH, bx + bs * 0.35, (by - 4.0 - zone).max(0.0), bx + bs * 0.65, by - 4.0));
        // Captured-piece trays along the south/north edges, past the
        // confirm zones. Screen-bound clamps only - the daemon rejects
        // overlapping fields (`FieldSet::is_disjoint`).
        fields.push(Self::field(crate::ZONE_TRAY_SOUTH, bx, (by + bs + 4.0 + zone).min(wh), bx + bs, wh));
        fields.push(Self::field(crate::ZONE_TRAY_NORTH, bx, 0.0, bx + bs, (by - 4.0 - zone).max(0.0)));
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
}
