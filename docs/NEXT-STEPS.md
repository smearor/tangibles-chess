# Tangibles Chess — Next Steps

Prioritized proposal for completing M2 and starting M3, based on the
implementation audit against `docs/REQUIREMENTS.md` (2026-10-29).
Requirement IDs refer to that document.

## 1. Fix `CommitMode::OnConfirmZone` (FR-TAN-11)

The strict commit mode is currently defective: moves commit on place
anyway and the confirm-zone press produces a spurious illegal-move
flash. Root cause and fix direction are documented in
`docs/bugs/on-confirm-zone-commit-order.md`.

- Make the inference output contract explicit (`PendingIntent` vs
  `MoveIntent`) or gate the apply in the controller
- Only the mover's `btn-{edge}` completes a pending intent
- Clear the pending state when the placed piece is lifted again
- Add a pending-confirm UI state (destination highlight + status line)
- Cover it end-to-end with `MockSource`

## 2. Anomaly-triggered occupancy resync (FR-TAN-13)

`InferenceOutput::Anomaly` currently only flashes a transient status
line. Call `resync_occupancy()` on anomalies as well (not only on
`Lagged`) so the physical board re-establishes truth after protocol
violations. Small change in `GameController::on_game_field_event`,
high robustness payoff before hardware testing.

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
