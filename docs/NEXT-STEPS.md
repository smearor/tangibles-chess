# Tangibles Chess — Next Steps

Prioritized proposal for completing M2 and starting M3, based on the
implementation audit against `docs/REQUIREMENTS.md` (2026-10-29).
Requirement IDs refer to that document.

## 1. Fix `CommitMode::OnConfirmZone` (FR-TAN-11) ✅

Fixed per `docs/bugs/on-confirm-zone-commit-order.md`: `commit()` now
emits `PendingIntent` (display-only) instead of an immediately applied
`MoveIntent`; only the mover's `btn-{edge}` press completes the intent
(`ConfirmWrongEdge`/`ConfirmWithoutPending` anomalies otherwise);
lifting the pending destination cancels via `Cancelled` + `takeback`
restore; the scene shows a destination highlight plus a per-side
"press your button" status line.

- Remaining: drive it end-to-end through `MockSource` (covered so far
  at `MoveInference` unit level).

## 2. Anomaly-triggered occupancy resync (FR-TAN-13) ✅

`Anomaly::needs_resync()` distinguishes gesture violations (lift/place
protocol breaks) from stray zone presses; the controller now calls
`inference.reset()` + clears the in-air/pending scene glow +
`resync_occupancy()` on the former, so the physical board re-establishes
truth instead of cascading anomalies off stale in-air state.

## 3. Draw offer / resign confirmation flows (FR-UI-6, FR-CORE-5) ✅

Draw is now a two-step handshake: the draw button records
`GameSession::offer_draw(side)` and shows a `ConfirmCard` facing the
**opponent's** edge - `accept_draw` finishes the game, `decline_draw`
flashes "draw declined" back to the offerer. Resign shows the same
card facing the requester before `session.resign` runs. `ConfirmCard`
reuses the `PromotionPicker` overlay pattern (`RotationWidget` for the
north edge); played moves lapse an open offer (FIDE convention); the
MCP `resign`/`agree_draw` tools keep their direct, immediate semantics.

## 4. Guided undo on the board (FR-CORE-6) ✅

`undo()`/`redo()` now arm `undo_guided`: the plan steps flash on both
status lines (localized via `undo-step-move`/`undo-step-restore`) and
the scene shows the live physical↔logical diff with the handshake's
`missing`/`foreign` markers — refreshed on every field event via the
`OccupancyProbe`, cleared when the board matches the rewound position
("position restored"). `NoTangibles` skips (rendered board already
matches); entering replay clears a pending guide.

## 5. `DiagnosticsOverlay` (FR-UI-13) ✅

`--diagnostics` draws the overlay inside `ChessScene`: per-field index
(`f{n}` top-left), live contact count (bottom-right, fed by a new
`OccupancyProbe::counts` map from `FieldCountChanged`/`refresh`), and
raw `touchPoints` dots polled at 200ms (`set_diagnostics` timer).
Rendering inside the scene keeps it rotation-aligned; per-cell labels
position exactly on the registered `f*` fields.

## 6. Hardware verification (V-4, V-3)

- Daemon integration against the `--replay` fixture, then the real
  panel: registration accepted (disjoint set), handshake, inference,
  trays, commit zones
- Manual tabletop checks: rotation 0/90/180/270°, touch accuracy,
  both-edge readability

This closes M2.

## 7. M3 kickoff (roughly in this order)

1. **Engine wiring (FR-ENG-1)** ✅ — `ChessEngine` trait +
   `EngineClient` impl; worker thread owns the blocking client,
   `(Chess, Fen)` requests / `EngineReply` poll at 50ms; `bestmove`
   flows through `MoveRequest::from_move` + `apply_request` with a
   stale-FEN guard; `OpponentKind::Agent` added (controller-side no-op
   like `Human`, moves arrive via MCP `make_move`); the setup picker
   is un-greyed. `engine_path` in `settings.toml` (default
   `stockfish`).
2. **Chess clocks (FR-ENG-3)** — increment/fixed, per-edge display;
   pairs naturally with the engine.
3. **Sound assets + missing cues (FR-ART-4, FR-SND-1)** —
   `assets/sounds/*.ogg` into the gresource; add castle, promotion and
   error variants to `SoundEffect`.
4. **PGN load/save (FR-ENG-4)** — `pgn-reader` is already in the
   workspace deps.
5. **Auto-rotation (FR-ENG-2)** — optional, `NoTangibles` only.
6. More themes, animation polish.

## Housekeeping

- `CHANGELOG.md` still only contains the scaffolding entry — catch up
  at the next release commit (replay mode, parking strips, MCP
  endpoint, discrepancy rings, mock/manual modes, new crates).
