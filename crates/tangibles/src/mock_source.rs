use std::sync::Arc;
use std::sync::Mutex;

use crate::DaemonError;
use crate::DeviceInfo;
use crate::FieldEvent;
use crate::FieldSetSpec;
use crate::TangibleSource;
use crate::TouchPoint;

/// Scriptable [`TangibleSource`] for tests and development without
/// hardware: injects queued field events and answers queries with
/// configured values.
#[derive(Clone)]
pub struct MockSource {
    /// Event stream handed to `events()`.
    events: async_channel::Sender<FieldEvent>,
    /// The matching receiver.
    receiver: async_channel::Receiver<FieldEvent>,
    /// Device calibration returned by `device_info`.
    device: DeviceInfo,
    /// Field set registrations observed, for test assertions.
    pub registered: Arc<Mutex<Vec<FieldSetSpec>>>,
}

impl Default for MockSource {
    fn default() -> Self {
        let (events, receiver) = async_channel::unbounded();
        Self {
            events,
            receiver,
            device: DeviceInfo::default(),
            registered: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl MockSource {
    /// A mock reporting an 800×800mm panel.
    pub fn with_device(device: DeviceInfo) -> Self {
        Self { device, ..Self::default() }
    }

    /// Injects a field event into the stream.
    pub fn inject(&self, event: FieldEvent) {
        // try_send cannot fail on an unbounded channel.
        let _ = self.events.try_send(event);
    }

    /// Convenience: inject a `FieldPlaced` for a field name.
    pub fn place(&self, field: &str) {
        self.inject(FieldEvent {
            kind: crate::FieldEventKind::FieldPlaced,
            field_set: Some(crate::FIELD_SET_NAME.to_string()),
            field: Some(field.to_string()),
            count: None,
            skipped: None,
        });
    }

    /// Convenience: inject a `FieldLifted` for a field name.
    pub fn lift(&self, field: &str) {
        self.inject(FieldEvent {
            kind: crate::FieldEventKind::FieldLifted,
            field_set: Some(crate::FIELD_SET_NAME.to_string()),
            field: Some(field.to_string()),
            count: None,
            skipped: None,
        });
    }
}

impl TangibleSource for MockSource {
    fn events(&self) -> async_channel::Receiver<FieldEvent> {
        self.receiver.clone()
    }

    async fn device_info(&self) -> Result<DeviceInfo, DaemonError> {
        Ok(self.device.clone())
    }

    async fn touch_points(&self) -> Result<Vec<TouchPoint>, DaemonError> {
        Ok(Vec::new())
    }

    async fn register_field_set(&self, spec: &FieldSetSpec) -> Result<(), DaemonError> {
        self.registered
            .lock()
            .map_err(|_| DaemonError::Unavailable("mock lock poisoned".to_string()))?
            .push(spec.clone());
        Ok(())
    }

    async fn unregister(&self) -> Result<(), DaemonError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MockSource;
    use crate::FieldEventKind;
    use crate::TangibleSource;

    #[test]
    fn injected_events_arrive() {
        let source = MockSource::default();
        source.place("f12");
        let receiver = source.events();
        let event = receiver.try_recv().expect("event");
        assert_eq!(event.kind, FieldEventKind::FieldPlaced);
        assert_eq!(event.field.as_deref(), Some("f12"));
    }
}
