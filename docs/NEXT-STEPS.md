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

## 3. Draw offer / resign confirmation flows (FR-UI-6, FR-CORE-5)

Both actions currently take effect on a single click — one side can
unilaterally draw or instantly resign.

- Draw: offer → the opponent's edge shows an accept/decline prompt
  facing them
- Resign: confirmation dialog facing the requester
- Reuse the `PromotionPicker` pattern (`RotationWidget` wrapper for
  the north edge)

## 4. Guided undo on the board (FR-CORE-6)

`UndoPlan` is computed but only `debug!`-logged. Visualize its
physical rebuild steps (move-back list, captured-piece re-placement,
castling rook, promotion reversal) on the scene — the existing
missing/foreign marker and ceremony-highlight infrastructure from the
handshake can be reused for the guidance.

## 5. `DiagnosticsOverlay` (FR-UI-13)

Live per-field contact counts plus raw touch points, needed for
calibration and hardware bring-up before V-4 can be executed
meaningfully.

## 6. Hardware verification (V-4, V-3)

- Daemon integration against the `--replay` fixture, then the real
  panel: registration accepted (disjoint set), handshake, inference,
  trays, commit zones
- Manual tabletop checks: rotation 0/90/180/270°, touch accuracy,
  both-edge readability

This closes M2.

## 7. M3 kickoff (roughly in this order)

1. **Engine wiring (FR-ENG-1)** — `EngineClient` exists; spawn it off
   the GTK thread when an `OpponentKind::Engine` side is up, feed
   positions, apply `bestmove`. Un-greys the setup picker.
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
