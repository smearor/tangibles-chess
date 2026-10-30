//! Which tangible backend feeds the controller.

use std::rc::Rc;

use tangibles_chess_tangibles::GraphQlSource;
use tangibles_chess_tangibles::MockSource;

/// Which tangible backend is active.
pub(super) enum Source {
    /// Real daemon connection.
    Daemon(Rc<GraphQlSource>),
    /// Development mock.
    Mock(MockSource),
}
