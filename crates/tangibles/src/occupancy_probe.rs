use std::collections::BTreeSet;

use tangibles_chess_geometry::FieldIndex;

use crate::FieldEvent;
use crate::FieldEventKind;
use crate::FieldName;
use crate::TouchPoint;
use crate::Zone;

/// Live view of which board fields are physically occupied.
///
/// Fed incrementally from `fieldEvents` and refreshed wholesale via a
/// `touchPoints` query when the subscription reports lag or the app
/// reconnects.
#[derive(Debug, Default, Clone)]
pub struct OccupancyProbe {
    /// Occupied board fields (screen order).
    occupied: BTreeSet<FieldIndex>,
    /// Occupied zone fields (`btn-*`, `tray-*`, `cap-*`).
    occupied_zones: BTreeSet<Zone>,
}

impl OccupancyProbe {
    /// Updates the probe from one field event.
    ///
    /// `FieldPlaced` inserts, `FieldLifted` removes, `FieldCountChanged`
    /// to 0 removes (a contact group can collapse without a lift event
    /// when debouncing glitches), `Lagged` marks nothing - callers should
    /// resync via [`OccupancyProbe::refresh`].
    pub fn on_event(&mut self, event: &FieldEvent) {
        let Some(name) = &event.field else {
            return;
        };
        let occupied = match event.kind {
            FieldEventKind::FieldPlaced => true,
            FieldEventKind::FieldLifted => false,
            FieldEventKind::FieldCountChanged => event.count.unwrap_or(0) > 0,
            FieldEventKind::Lagged | FieldEventKind::Other => return,
        };
        match name {
            FieldName::Board(index) => {
                if occupied {
                    self.occupied.insert(*index);
                } else {
                    self.occupied.remove(index);
                }
            }
            FieldName::Zone(zone) => {
                if occupied {
                    self.occupied_zones.insert(*zone);
                } else {
                    self.occupied_zones.remove(zone);
                }
            }
            FieldName::Unknown(_) => {}
        }
    }

    /// Rebuilds board occupancy from a `touchPoints` query result:
    /// contacts inside each field's normalized rectangle count it
    /// occupied. `window_mm`/`board_mm` come from the same calibration
    /// that built the field set.
    pub fn refresh(&mut self, touches: &[TouchPoint], board_mm: impl Into<crate::BoardRectMm>) {
        self.occupied.clear();
        let board = board_mm.into();
        let cell = board.cell().as_f64();
        let (bx, by) = (board.x.as_f64(), board.y.as_f64());
        for touch in touches {
            let col = ((touch.pos.x - bx) / cell) as i64;
            let row = ((touch.pos.y - by) / cell) as i64;
            if (0..8).contains(&col)
                && (0..8).contains(&row)
                && let Some(index) = FieldIndex::new(row as usize * 8 + col as usize)
            {
                self.occupied.insert(index);
            }
        }
    }

    /// All currently occupied board fields (screen order).
    pub fn occupied(&self) -> &BTreeSet<FieldIndex> {
        &self.occupied
    }

    /// Whether a zone field currently holds a marker.
    pub fn zone_occupied(&self, zone: Zone) -> bool {
        self.occupied_zones.contains(&zone)
    }
}

#[cfg(test)]
mod tests {
    use tangibles_chess_geometry::FieldIndex;

    use super::OccupancyProbe;
    use crate::FieldEvent;
    use crate::FieldEventKind;
    use crate::FieldName;
    use crate::TouchPoint;
    use crate::Zone;

    fn event(kind: FieldEventKind, field: &str, count: Option<i32>) -> FieldEvent {
        FieldEvent {
            kind,
            field_set: Some("chess".to_string()),
            field: Some(FieldName::from(field)),
            count,
            skipped: None,
        }
    }

    #[test]
    fn tracks_place_and_lift() {
        let mut probe = OccupancyProbe::default();
        probe.on_event(&event(FieldEventKind::FieldPlaced, "f12", None));
        probe.on_event(&event(FieldEventKind::FieldPlaced, "btn-south", None));
        let f12 = FieldIndex::new(12).expect("in range");
        assert!(probe.occupied().contains(&f12));
        assert!(probe.zone_occupied(Zone::BtnSouth));
        probe.on_event(&event(FieldEventKind::FieldLifted, "f12", None));
        assert!(!probe.occupied().contains(&f12));
        probe.on_event(&event(FieldEventKind::FieldLifted, "btn-south", None));
        assert!(!probe.zone_occupied(Zone::BtnSouth));
    }

    #[test]
    fn count_zero_clears_without_lift() {
        let mut probe = OccupancyProbe::default();
        probe.on_event(&event(FieldEventKind::FieldPlaced, "f9", None));
        probe.on_event(&event(FieldEventKind::FieldCountChanged, "f9", Some(0)));
        assert!(probe.occupied().is_empty());
    }

    #[test]
    fn refresh_maps_touches_into_board_fields() {
        let mut probe = OccupancyProbe::default();
        let touches = [
            TouchPoint {
                slot: 0,
                tracking_id: 1,
                pos: crate::ContactPos { x: 90.0, y: 90.0 },
            },
            TouchPoint {
                slot: 1,
                tracking_id: 2,
                pos: crate::ContactPos { x: 715.0, y: 715.0 },
            },
            TouchPoint {
                slot: 2,
                tracking_id: 3,
                pos: crate::ContactPos { x: 400.0, y: 790.0 }, // below the board - not a field
            },
        ];
        probe.refresh(&touches, (80.0, 80.0, 640.0));
        assert_eq!(probe.occupied().len(), 2);
        assert!(probe.occupied().contains(&FieldIndex::new(0).expect("in range")));
        assert!(probe.occupied().contains(&FieldIndex::new(63).expect("in range")));
    }
}
