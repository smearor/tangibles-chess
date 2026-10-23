use tangibles_chess_tangibles::DaemonError;
use tangibles_chess_tangibles::GraphQlSource;
use tangibles_chess_tangibles::TangibleSource;

/// Owns the Tokio runtime the daemon client runs on.
///
/// GTK runs the glib main loop on the main thread; the websocket task of
/// `GraphQlSource` needs a Tokio reactor, hosted here on its own runtime
/// (multi-thread scheduler drives spawned tasks without `block_on`
/// staying parked).
pub struct TangibleHost {
    /// The Tokio runtime driving daemon I/O.
    runtime: tokio::runtime::Runtime,
}

impl TangibleHost {
    /// Creates the runtime.
    pub fn new() -> Result<Self, std::io::Error> {
        Ok(Self {
            runtime: tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build()?,
        })
    }

    /// Connects to the daemon and returns the live source.
    ///
    /// Runs `connect` on the runtime so the websocket subscription task
    /// stays scheduled there.
    pub fn connect(&self, endpoint: &str) -> Result<GraphQlSource, DaemonError> {
        self.runtime.block_on(GraphQlSource::connect(endpoint))
    }

    /// Registers the chess field set on the daemon.
    pub fn register(&self, source: &GraphQlSource, spec: &tangibles_chess_tangibles::FieldSet) -> Result<(), DaemonError> {
        self.runtime.block_on(source.register_field_set(spec))
    }

    /// Queries device calibration.
    pub fn device_info(&self, source: &GraphQlSource) -> Result<tangibles_chess_tangibles::DeviceInfo, DaemonError> {
        self.runtime.block_on(source.device_info())
    }

    /// Queries live touch points (occupancy resync).
    pub fn touch_points(&self, source: &GraphQlSource) -> Result<Vec<tangibles_chess_tangibles::TouchPoint>, DaemonError> {
        self.runtime.block_on(source.touch_points())
    }

    /// Removes this app's registrations on the daemon - the field set
    /// and every resource it owns.
    pub fn unregister(&self, source: &GraphQlSource) -> Result<(), DaemonError> {
        self.runtime.block_on(source.unregister())
    }
}
