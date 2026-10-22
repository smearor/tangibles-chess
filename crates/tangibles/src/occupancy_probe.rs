use std::collections::BTreeSet;

use crate::BoardGeometry;
use crate::FieldEvent;
use crate::FieldEventKind;
use crate::TouchPoint;

/// Live view of which board fields are physically occupied.
///
/// Fed incrementally from `fieldEvents` and refreshed wholesale via a
/// `touchPoints` query when the subscription reports lag or the app
/// reconnects.
#[derive(Debug, Default, Clone)]
pub struct OccupancyProbe {
    /// Occupied board field indices (screen order).
    occupied: BTreeSet<usize>,
    /// Occupied zone fields (`btn-*`, `tray-*` names).
    occupied_zones: BTreeSet<String>,
}

impl OccupancyProbe {
    /// Updates the probe from one field event.
    ///
    /// `FieldPlaced` inserts, `FieldLifted` removes, `FieldCountChanged`
    /// to 0 removes (a contact group can collapse without a lift event
    /// when debouncing glitches), `Lagged` marks nothing - callers should
    /// resync via [`OccupancyProbe::refresh`].
    pub fn on_event(&mut self, event: &FieldEvent) {
        let Some(field) = event.field.as_deref() else {
            return;
        };
        let occupied = match event.kind {
            FieldEventKind::FieldPlaced => true,
            FieldEventKind::FieldLifted => false,
            FieldEventKind::FieldCountChanged => event.count.unwrap_or(0) > 0,
            FieldEventKind::Lagged | FieldEventKind::Other => return,
        };
        if let Some(index) = BoardGeometry::field_index(field) {
            if occupied {
                self.occupied.insert(index);
            } else {
                self.occupied.remove(&index);
            }
        } else if occupied {
            self.occupied_zones.insert(field.to_string());
        } else {
            self.occupied_zones.remove(field);
        }
    }

    /// Rebuilds board occupancy from a `touchPoints` query result:
    /// contacts inside each field's normalized rectangle count it
    /// occupied. `window_mm`/`board_mm` come from the same calibration
    /// that built the field set.
    pub fn refresh(&mut self, touches: &[TouchPoint], board_mm: (f64, f64, f64)) {
        self.occupied.clear();
        let (bx, by, bs) = board_mm;
        let cell = bs / 8.0;
        for touch in touches {
            let col = ((touch.pos.x - bx) / cell) as i64;
            let row = ((touch.pos.y - by) / cell) as i64;
            if (0..8).contains(&col) && (0..8).contains(&row) {
                self.occupied.insert(row as usize * 8 + col as usize);
            }
        }
    }

    /// All currently occupied board field indices (screen order).
    pub fn occupied(&self) -> &BTreeSet<usize> {
        &self.occupied
    }

    /// Whether a zone field currently holds a marker.
    pub fn zone_occupied(&self, zone: &str) -> bool {
        self.occupied_zones.contains(zone)
    }
}

#[cfg(test)]
mod tests {
    use super::OccupancyProbe;
    use crate::FieldEvent;
    use crate::FieldEventKind;
    use crate::TouchPoint;

    fn event(kind: FieldEventKind, field: &str, count: Option<i32>) -> FieldEvent {
        FieldEvent {
            kind,
            field_set: Some("chess".to_string()),
            field: Some(field.to_string()),
            count,
            skipped: None,
        }
    }

    #[test]
    fn tracks_place_and_lift() {
        let mut probe = OccupancyProbe::default();
        probe.on_event(&event(FieldEventKind::FieldPlaced, "f12", None));
        probe.on_event(&event(FieldEventKind::FieldPlaced, "btn-south", None));
        assert!(probe.occupied().contains(&12));
        assert!(probe.zone_occupied("btn-south"));
        probe.on_event(&event(FieldEventKind::FieldLifted, "f12", None));
        assert!(!probe.occupied().contains(&12));
        probe.on_event(&event(FieldEventKind::FieldLifted, "btn-south", None));
        assert!(!probe.zone_occupied("btn-south"));
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
        assert!(probe.occupied().contains(&0));
        assert!(probe.occupied().contains(&63));
    }
}
