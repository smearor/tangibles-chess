use crate::ui::flash_role::FlashRole;
use crate::ui::generation_counter::GenerationCounter;
use shakmaty::Square;

/// A rejected move's flash: the rejected target plus, when the naive
/// move would have left the own king in check, the king and its
/// attackers.
#[derive(Debug)]
pub(crate) struct IllegalFlash {
    /// Flashed squares with their meaning.
    pub squares: Vec<(Square, FlashRole)>,
    /// Generation counter so a stale expiry timer cannot clear a newer
    /// flash.
    pub generation: GenerationCounter,
}
