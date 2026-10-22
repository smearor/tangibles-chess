use serde::Deserialize;
use tangible_objects_model::EventKind;
use tangible_objects_model::TangibleEvent;

/// Discriminator of an occupancy event from `fieldEvents`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldEventKind {
    /// The field became occupied (debounced placed).
    FieldPlaced,
    /// The field became empty (debounced lifted).
    FieldLifted,
    /// The field's debounced contact count changed.
    FieldCountChanged,
    /// Subscription lag: events were dropped; resync via `touchPoints`.
    Lagged,
    /// A non-field event the projection does not consume.
    #[serde(other)]
    Other,
}

/// Flattened payload delivered by the `fieldEvents` subscription.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldEvent {
    /// What happened.
    pub kind: FieldEventKind,
    /// Field-set name (`chess` for ours).
    pub field_set: Option<String>,
    /// Field name (`f0`..`f63`, `btn-*`, `tray-*`).
    pub field: Option<String>,
    /// Debounced contact count for `FieldCountChanged`.
    pub count: Option<i32>,
    /// Dropped-event count for `Lagged`.
    pub skipped: Option<i32>,
}

impl FieldEvent {
    /// A synthetic lag event (e.g. on a subscription stream error).
    pub fn lagged(skipped: Option<i32>) -> Self {
        Self {
            kind: FieldEventKind::Lagged,
            field_set: None,
            field: None,
            count: None,
            skipped,
        }
    }
}

impl From<&TangibleEvent> for FieldEvent {
    /// Projects the full daemon event down to the field-occupancy
    /// subset this crate consumes.
    fn from(event: &TangibleEvent) -> Self {
        let kind = match event.kind {
            EventKind::FieldPlaced => FieldEventKind::FieldPlaced,
            EventKind::FieldLifted => FieldEventKind::FieldLifted,
            EventKind::FieldCountChanged => FieldEventKind::FieldCountChanged,
            EventKind::Lagged => FieldEventKind::Lagged,
            _ => FieldEventKind::Other,
        };
        Self {
            kind,
            field_set: event.field_set.as_ref().map(|name| name.as_str().to_string()),
            field: event.field.as_ref().map(|name| name.as_str().to_string()),
            count: event.count.map(|count| count.as_u32() as i32),
            skipped: event.skipped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FieldEvent;
    use super::FieldEventKind;

    #[test]
    fn decodes_field_placed() {
        let event: FieldEvent =
            serde_json::from_str(r#"{"kind":"FIELD_PLACED","fieldSet":"chess","field":"f12","count":null,"skipped":null}"#).expect("decode");
        assert_eq!(event.kind, FieldEventKind::FieldPlaced);
        assert_eq!(event.field.as_deref(), Some("f12"));
    }

    #[test]
    fn decodes_count_changed_and_lagged() {
        let count: FieldEvent = serde_json::from_str(r#"{"kind":"FIELD_COUNT_CHANGED","field":"f3","count":2}"#).expect("decode");
        assert_eq!(count.kind, FieldEventKind::FieldCountChanged);
        assert_eq!(count.count, Some(2));
        let lagged: FieldEvent = serde_json::from_str(r#"{"kind":"LAGGED","skipped":7}"#).expect("decode");
        assert_eq!(lagged.kind, FieldEventKind::Lagged);
        assert_eq!(lagged.skipped, Some(7));
    }
}
