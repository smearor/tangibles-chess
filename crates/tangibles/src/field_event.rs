use serde::Deserialize;

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
