//! Tangible input for tangibles-chess.
//!
//! Talks to the `tangible-objects` daemon over its GraphQL interface
//! (field set registration, `fieldEvents` subscription, `touchPoints`
//! probe) and derives chess moves from marker placements.
//!
//! Field names are screen-position identifiers (`f0`..`f63`, row-major
//! from the board rectangle's top-left screen corner, plus action and
//! tray zones). `BoardGeometry` permutes them onto logical chess squares
//! through the current board orientation, because the field set is
//! screen-fixed while the rendered board rotates.

mod board_geometry;
mod daemon_error;
mod device_info;
mod field_event;
mod field_input;
mod field_set_builder;
mod graphql_source;
mod inference_output;
mod mock_source;
mod move_inference;
mod occupancy_probe;
mod placement_gate;
mod placement_status;
mod table_edge;
mod tangible_source;
mod touch_point;
mod zone_action;

pub use board_geometry::BoardGeometry;
pub use daemon_error::DaemonError;
pub use device_info::DeviceInfo;
pub use field_event::FieldEvent;
pub use field_event::FieldEventKind;
pub use field_input::FieldInput;
pub use field_set_builder::FieldSetBuilder;
pub use field_set_builder::FieldSetSpec;
pub use graphql_source::GraphQlSource;
pub use inference_output::InferenceOutput;
pub use mock_source::MockSource;
pub use move_inference::MoveInference;
pub use occupancy_probe::OccupancyProbe;
pub use placement_gate::PlacementGate;
pub use placement_status::PlacementStatus;
pub use table_edge::TableEdge;
pub use tangible_source::TangibleSource;
pub use touch_point::ContactPos;
pub use touch_point::TouchPoint;
pub use zone_action::ZoneAction;

/// Field set name registered with the daemon for chess.
pub const FIELD_SET_NAME: &str = "chess";
/// Owner name used for daemon registrations of this app.
pub const REGISTRATION_OWNER: &str = "tangibles-chess";
/// Field name of the south-side confirm zone.
pub const ZONE_BTN_SOUTH: &str = "btn-south";
/// Field name of the north-side confirm zone.
pub const ZONE_BTN_NORTH: &str = "btn-north";
/// Field name of the south-side captured tray.
pub const ZONE_TRAY_SOUTH: &str = "tray-south";
/// Field name of the north-side captured tray.
pub const ZONE_TRAY_NORTH: &str = "tray-north";
