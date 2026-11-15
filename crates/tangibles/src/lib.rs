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

mod anomaly;
mod board_geometry;
mod board_rect_mm;
mod cap_side;
mod daemon_error;
mod field_event;
mod field_name;
mod field_set_builder;
mod graphql_source;
mod inference_output;
mod layout;
mod mock_source;
mod move_inference;
mod occupancy_probe;
mod placement_gate;
mod placement_status;
mod surface_mm;
mod table_edge;
mod tangible_source;
mod zone;
mod zone_action;

pub use anomaly::Anomaly;
pub use board_geometry::BoardGeometry;
pub use board_rect_mm::BoardRectMm;
pub use cap_side::CapSide;
pub use daemon_error::DaemonError;
pub use field_event::FieldEvent;
pub use field_event::FieldEventKind;
pub use field_name::FieldName;
pub use field_set_builder::FieldSetBuilder;
pub use graphql_source::GraphQlSource;
pub use inference_output::InferenceOutput;
pub use layout::Layout;
pub use mock_source::MockSource;
pub use move_inference::MoveInference;
pub use occupancy_probe::OccupancyProbe;
pub use placement_gate::PlacementGate;
pub use placement_status::PlacementStatus;
pub use surface_mm::SurfaceMm;
pub use table_edge::TableEdge;
pub use tangible_source::TangibleSource;
pub use zone::Zone;
pub use zone_action::ZoneAction;

/// Field set name registered with the daemon for chess.
pub const FIELD_SET_NAME: &str = "chess";
/// Owner name used for daemon registrations of this app.
pub const REGISTRATION_OWNER: &str = "tangibles-chess";
