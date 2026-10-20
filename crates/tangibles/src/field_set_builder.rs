use serde::Serialize;

use crate::FieldInput;

/// Full input for the daemon's `registerFieldSet` mutation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldSetSpec {
    /// Set name (namespace of the fields).
    pub name: String,
    /// Debounce stability in frames.
    pub stability: u32,
    /// Window anchor the normalized field coordinates resolve against.
    pub window: WindowInput,
    /// Whether contacts inside these fields are suppressed from the OS.
    pub suppress: bool,
    /// The cells: 64 board fields + confirm zones + trays.
    pub fields: Vec<FieldInput>,
}

/// Window anchor of a field set (screen millimeters).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInput {
    /// Left edge of the window in screen mm.
    pub x_offset: f64,
    /// Top edge of the window in screen mm.
    pub y_offset: f64,
    /// Window width in mm (enables normalized `0..1` field coordinates).
    pub width: f64,
    /// Window height in mm.
    pub height: f64,
}

/// Builds the chess field set from board geometry in screen millimeters.
///
/// Board fields are normalized inside the `[window]` board rectangle;
/// confirm zones and trays sit just outside the board edges and are
/// given in absolute screen mm offset from it.
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

    /// Builds the full field set specification.
    pub fn build(&self) -> FieldSetSpec {
        let (bx, by, bs) = self.board_mm;
        let (ww, wh) = self.window_mm;
        let cell = bs / 8.0;
        let mut fields = Vec::with_capacity(64 + 4);
        for index in 0..64 {
            let (row, col) = (index / 8, index % 8);
            fields.push(FieldInput {
                name: format!("f{index}"),
                x_min: bx + col as f64 * cell,
                y_min: by + row as f64 * cell,
                x_max: bx + (col + 1) as f64 * cell,
                y_max: by + (row + 1) as f64 * cell,
            });
        }
        let zone = self.zone_mm;
        // Confirm zones: squares just beyond the south/north board edges.
        fields.push(FieldInput {
            name: crate::ZONE_BTN_SOUTH.to_string(),
            x_min: bx + bs * 0.35,
            y_min: by + bs + 4.0,
            x_max: bx + bs * 0.65,
            y_max: by + bs + 4.0 + zone,
        });
        fields.push(FieldInput {
            name: crate::ZONE_BTN_NORTH.to_string(),
            x_min: bx + bs * 0.35,
            y_min: by - 4.0 - zone,
            x_max: bx + bs * 0.65,
            y_max: by - 4.0,
        });
        // Captured-piece trays along the south/north edges.
        fields.push(FieldInput {
            name: crate::ZONE_TRAY_SOUTH.to_string(),
            x_min: bx,
            y_min: (by + bs + 4.0 + zone).min(wh - zone),
            x_max: bx + bs,
            y_max: wh,
        });
        fields.push(FieldInput {
            name: crate::ZONE_TRAY_NORTH.to_string(),
            x_min: bx,
            y_min: 0.0,
            x_max: bx + bs,
            y_max: (by - 4.0 - zone).max(zone),
        });
        FieldSetSpec {
            name: crate::FIELD_SET_NAME.to_string(),
            stability: 2,
            window: WindowInput {
                x_offset: 0.0,
                y_offset: 0.0,
                width: ww,
                height: wh,
            },
            suppress: true,
            fields,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FieldSetBuilder;

    #[test]
    fn builds_68_fields_with_board_cells_normalized() {
        // 800x800mm window, 640mm board centered.
        let spec = FieldSetBuilder::new((80.0, 80.0, 640.0), (800.0, 800.0)).build();
        assert_eq!(spec.name, "chess");
        assert!(spec.suppress);
        assert_eq!(spec.fields.len(), 68);
        let f0 = spec.fields.iter().find(|f| f.name == "f0").expect("f0");
        assert_eq!(f0.x_min, 80.0);
        assert_eq!(f0.y_min, 80.0);
        let f63 = spec.fields.iter().find(|f| f.name == "f63").expect("f63");
        assert_eq!(f63.x_max, 720.0);
        assert_eq!(f63.y_max, 720.0);
        for name in ["btn-south", "btn-north", "tray-south", "tray-north"] {
            assert!(spec.fields.iter().any(|f| f.name == name), "missing {name}");
        }
    }
}
