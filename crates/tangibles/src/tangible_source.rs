use std::future::Future;

use tangible_objects_model::FieldSet;

use crate::DaemonError;
use crate::DeviceInfo;
use crate::FieldEvent;
use crate::TouchPoint;

/// A source of tangible-object state, backed either by the daemon's
/// GraphQL interface or a mock for tests/development.
pub trait TangibleSource: Send + Sync {
    /// The live `fieldEvents` stream for the registered field set.
    ///
    /// Called once at activation; the receiver stays valid until the
    /// source is dropped. Implementations must translate broadcast lag
    /// into `FieldEventKind::Lagged` events so callers resync.
    fn events(&self) -> async_channel::Receiver<FieldEvent>;

    /// Device calibration from the `deviceInfo` query.
    fn device_info(&self) -> impl Future<Output = Result<DeviceInfo, DaemonError>> + Send;

    /// Live touch points from the `touchPoints` query (occupancy resync).
    fn touch_points(&self) -> impl Future<Output = Result<Vec<TouchPoint>, DaemonError>> + Send;

    /// Registers the chess field set (`registerFieldSet` mutation).
    fn register_field_set(&self, spec: &FieldSet) -> impl Future<Output = Result<(), DaemonError>> + Send;

    /// Removes this app's registrations (`unregisterOwner` mutation).
    fn unregister(&self) -> impl Future<Output = Result<(), DaemonError>> + Send;
}
