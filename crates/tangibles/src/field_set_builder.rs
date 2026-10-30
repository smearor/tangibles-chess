use crate::BoardRectMm;
use crate::SurfaceMm;
use crate::TableEdge;
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
/// Board fields are absolute screen-mm rectangles; confirm zones,
/// captured-piece trays and parking columns sit just outside the board
/// edges.
pub struct FieldSetBuilder {
    /// Board rectangle in screen millimeters.
    pub board_mm: BoardRectMm,
    /// Overall window (screen) size in millimeters.
    pub window_mm: SurfaceMm,
}

impl FieldSetBuilder {
    /// Creates a builder; `board_mm` is (x, y, side) of the board square
    /// and `window_mm` the full window size in millimeters.
    pub fn new(board_mm: impl Into<BoardRectMm>, window_mm: impl Into<SurfaceMm>) -> Self {
        Self {
            board_mm: board_mm.into(),
            window_mm: window_mm.into(),
        }
    }

    /// One occupancy cell named `name` with absolute bounds.
    fn field(name: impl std::fmt::Display, x_min: Millimeters, y_min: Millimeters, x_max: Millimeters, y_max: Millimeters) -> Field {
        Field {
            name: FieldName::new(name.to_string()),
            region: Region { x_min, y_min, x_max, y_max },
        }
    }

    /// Builds the full field set: 64 board cells + 2 confirm zones +
    /// 2 captured trays + up to 2 parking columns, suppressing marker
    /// contacts from the OS.
    ///
    /// Disjointness is structural: the confirm zone and tray on each
    /// side split the free strip between the board edge and the surface
    /// edge without gaps or clamps that could invert a region. A board
    /// that would poke past the surface is shrunk to fit first, since
    /// any overflow row would overlap the degenerate edge zones.
    pub fn build(&self) -> FieldSet {
        let layout = self.layout();
        let mut fields = self.board_fields(&layout);
        fields.extend(self.edge_fields(TableEdge::South, &layout));
        fields.extend(self.edge_fields(TableEdge::North, &layout));
        fields.extend(self.parking_fields(&layout));
        FieldSet {
            name: FieldSetName::new(crate::FIELD_SET_NAME),
            window: None,
            stability: FrameCount::new(2),
            fields,
            suppress: true,
        }
    }

    /// Derived geometry for one build pass: the board clamped into the
    /// surface, the cell size and the confirm-zone depth scaled to it
    /// (half a square fits a resting figure - ~40mm on an 80mm cell -
    /// without eating the whole tray strip).
    fn layout(&self) -> Layout {
        let (bx, by) = (self.board_mm.x, self.board_mm.y);
        let (ww, wh) = (self.window_mm.width, self.window_mm.height);
        let bs = self.board_mm.side.min(ww - bx).min(wh - by).max(Millimeters::default());
        let cell = bs * 0.125;
        Layout {
            bx,
            by,
            bs,
            cell,
            gap: Millimeters::new(4.0),
            zone: cell * 0.5,
            ww,
            wh,
        }
    }

    /// The 64 board cells `f0`..`f63`, row-major from the board's
    /// top-left corner.
    fn board_fields(&self, layout: &Layout) -> Vec<Field> {
        (0..64)
            .map(|index| {
                let (row, col) = (index / 8, index % 8);
                Self::field(
                    format!("f{index}"),
                    layout.bx + layout.cell * col as f64,
                    layout.by + layout.cell * row as f64,
                    layout.bx + layout.cell * (col + 1) as f64,
                    layout.by + layout.cell * (row + 1) as f64,
                )
            })
            .collect()
    }

    /// Confirm strip + captured tray on one board edge. The confirm
    /// zone sits directly at the board edge and takes `zone` depth of
    /// the free strip; the tray claims the rest to the surface edge -
    /// collapsing to a zero-height region when no room is left (a
    /// zero-height region never overlaps).
    fn edge_fields(&self, edge: TableEdge, layout: &Layout) -> Vec<Field> {
        let (btn, tray, y0, y1, ty0, ty1) = match edge {
            TableEdge::South => {
                let top = layout.by + layout.bs + layout.gap;
                let confirm_h = (layout.wh - top).max(Millimeters::default()).min(layout.zone);
                (
                    crate::Zone::BtnSouth,
                    crate::Zone::TraySouth,
                    top,
                    top + confirm_h,
                    (top + confirm_h).min(layout.wh),
                    layout.wh,
                )
            }
            TableEdge::North => {
                let bottom = (layout.by - layout.gap).max(Millimeters::default());
                let confirm_h = bottom.min(layout.zone);
                (
                    crate::Zone::BtnNorth,
                    crate::Zone::TrayNorth,
                    bottom - confirm_h,
                    bottom,
                    Millimeters::default(),
                    bottom - confirm_h,
                )
            }
            // East/West host the parking strips, not confirm/tray zones.
            TableEdge::East | TableEdge::West => return Vec::new(),
        };
        vec![
            Self::field(btn, layout.bx + layout.bs * 0.35, y0, layout.bx + layout.bs * 0.65, y1),
            Self::field(tray, layout.bx, ty0, layout.bx + layout.bs, ty1),
        ]
    }

    /// Captured-piece parking cells beside the board - one board cell
    /// wide, eight cells tall. Each side registers only where a full
    /// column actually fits between board and surface edge; the cells
    /// are individual fields so the app knows exactly which cell a
    /// parked marker rests on.
    fn parking_fields(&self, layout: &Layout) -> Vec<Field> {
        let mut fields = Vec::new();
        for row in 0..crate::CapSide::ROWS {
            let y0 = layout.by + layout.cell * f64::from(row);
            let y1 = y0 + layout.cell;
            if layout.bx >= layout.cell {
                fields.push(Self::field(
                    crate::Zone::Cap {
                        side: crate::CapSide::West,
                        row,
                    },
                    layout.bx - layout.cell,
                    y0,
                    layout.bx,
                    y1,
                ));
            }
            if layout.bx + layout.bs + layout.cell <= layout.ww {
                fields.push(Self::field(
                    crate::Zone::Cap {
                        side: crate::CapSide::East,
                        row,
                    },
                    layout.bx + layout.bs,
                    y0,
                    layout.bx + layout.bs + layout.cell,
                    y1,
                ));
            }
        }
        fields
    }
}

/// Derived build geometry (all values screen mm).
struct Layout {
    /// Board left edge.
    bx: Millimeters,
    /// Board top edge.
    by: Millimeters,
    /// Board edge length, clamped to fit the surface.
    bs: Millimeters,
    /// One board square's edge length (`bs / 8`).
    cell: Millimeters,
    /// Gap between board edge and confirm zone.
    gap: Millimeters,
    /// Confirm-zone depth (`cell / 2`).
    zone: Millimeters,
    /// Surface width.
    ww: Millimeters,
    /// Surface height.
    wh: Millimeters,
}

#[cfg(test)]
mod tests {
    use super::FieldSetBuilder;

    #[test]
    fn builds_70_fields_with_board_cells_and_parking() {
        // 800x800mm window, 640mm board centered.
        let set = FieldSetBuilder::new((80.0, 80.0, 640.0), (800.0, 800.0)).build();
        assert_eq!(set.name.as_str(), "chess");
        assert!(set.suppress);
        assert!(set.is_disjoint());
        assert_eq!(set.fields.len(), 84);
        let f0 = set.fields.iter().find(|f| f.name.as_str() == "f0").expect("f0");
        assert_eq!(f0.region.x_min.as_f64(), 80.0);
        assert_eq!(f0.region.y_min.as_f64(), 80.0);
        let f63 = set.fields.iter().find(|f| f.name.as_str() == "f63").expect("f63");
        assert_eq!(f63.region.x_max.as_f64(), 720.0);
        assert_eq!(f63.region.y_max.as_f64(), 720.0);
        for name in ["btn-south", "btn-north", "tray-south", "tray-north"] {
            assert!(set.fields.iter().any(|f| f.name.as_str() == name), "missing {name}");
        }
        for row in 0..8 {
            for side in ["cap-west", "cap-east"] {
                let name = format!("{side}-{row}");
                assert!(set.fields.iter().any(|f| f.name.as_str() == name), "missing {name}");
            }
        }
        let west3 = set.fields.iter().find(|f| f.name.as_str() == "cap-west-3").expect("cap-west-3");
        assert_eq!(west3.region.x_min.as_f64(), 0.0);
        assert_eq!(west3.region.x_max.as_f64(), 80.0);
        assert_eq!(west3.region.y_min.as_f64(), 320.0);
        assert_eq!(west3.region.y_max.as_f64(), 400.0);
    }

    #[test]
    fn parking_columns_only_when_the_margin_fits() {
        // A board flush with the surface's left edge has no west strip
        // but still gets an east one.
        let set = FieldSetBuilder::new((0.0, 80.0, 640.0), (800.0, 800.0)).build();
        assert!(set.is_disjoint());
        assert!(set.fields.iter().all(|f| !f.name.as_str().starts_with("cap-west")));
        assert_eq!(set.fields.iter().filter(|f| f.name.as_str().starts_with("cap-east")).count(), 8);
        // Less than one board cell of margin fits neither side.
        let set = FieldSetBuilder::new((10.0, 80.0, 640.0), (700.0, 800.0)).build();
        assert!(set.fields.iter().all(|f| !f.name.as_str().starts_with("cap-")));
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
