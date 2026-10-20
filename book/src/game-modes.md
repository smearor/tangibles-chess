# Game Modes

## Tangible objects mode

| Mode | Input | Pieces |
|------|-------|--------|
| `Tangibles` | physical markers | not rendered - screen shows board, light guidance, panels |
| `TangiblesRendered` | physical markers | rendered pieces as control layer + discrepancy display |
| `NoTangibles` | touch (tap-tap / drag) | rendered pieces |

## Start handshake

A tangible game only starts once the physical occupancy matches the expected
position (initial position, or the resumed one). The board check screen shows
progress ("n/32 pieces") and marks missing or wrongly occupied squares.

## Commit mode

- `OnPlace` - a move applies when the marker is set down.
- `OnConfirmZone` - the move only applies after the moving player places the
  marker on their `btn-*` confirm zone (tournament-style clock press).

## Guided undo

Undo in tangible modes replays the physical steps: ghost targets show where
markers need to go back, and `OccupancyProbe` confirms each step before the
undo applies logically.
