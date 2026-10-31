# Bug: `CommitMode::OnConfirmZone` commits on place and punishes the confirm press

- **Requirement:** FR-TAN-11 (strict commit mode: pending until the mover's `btn-*` zone reports a placed contact)
- **Status:** open — documented 2026-10-29, not yet fixed
- **Severity:** feature broken; strict mode is selectable in the setup screen but behaves worse than `OnPlace`

## Observed behaviour

With `CommitMode::OnConfirmZone` configured:

1. The player lifts a piece and places it on a destination square → the
   move is applied to the session **immediately** (same as `OnPlace`).
2. The player then presses the confirm zone (`btn-south`/`btn-north`)
   as the protocol intends → the buffered intent is re-applied, fails
   as illegal (the move already happened), and produces an
   illegal-move flash plus a rejected-move entry.

So strict mode neither defers the commit nor accepts the confirm press
gracefully.

## Root cause

Two halves of the contract disagree about who applies the intent:

- `MoveInference::commit()` emits `InferenceOutput::MoveIntent`
  **unconditionally**, even in `OnConfirmZone` mode, and additionally
  stores the request in `pending_confirm`
  (`crates/tangibles/src/move_inference.rs`, `commit()`). The intent is
  meant "for display/commit handling" per the test comment in
  `confirm_zone_release_emits_buffered_intent` — i.e. the caller is
  expected to *show* it but not *apply* it until the zone press.
- `GameController::on_game_field_event` applies **every** `MoveIntent`
  via `apply_request` and explicitly ignores
  `ZoneAction::ConfirmMove` (empty match arm,
  `crates/app/src/controller/tangibles.rs`).

When the confirm zone is pressed, `on_confirm_zone()` re-emits the
buffered `MoveIntent`; `apply_request` runs a second time against the
already-moved position → `MoveOutcome::Illegal` → `rejected_move` +
`flash_illegal` + status-line error
(`crates/app/src/controller/playable.rs`).

## Secondary issues worth handling in the fix

- `pending_confirm` is only cleared by a zone press. If the player
  lifts the just-placed piece again instead of confirming (takeback of
  the physical placement), the buffered intent goes stale — the next
  unrelated zone press would replay it.
- `on_confirm_zone` accepts a press from *either* edge
  (`ZoneAction::ConfirmMove { edge }` is ignored in the controller). In
  strict mode the confirm must come from the *mover's* edge; a press on
  the opponent's `btn-*` zone should not commit.
- There is no "pending confirm" UI state (e.g. pulsing destination
  square or status line prompting the clock press), so even a correct
  dispatch would lack feedback.

## Suggested direction

Make the output contract explicit instead of relying on the caller to
know that the first `MoveIntent` is display-only:

1. In `OnConfirmZone` mode, emit a distinct
   `InferenceOutput::PendingIntent(MoveRequest)` from `commit()` and
   reserve `MoveIntent` for the confirmed commit. The controller shows
   a pending state on `PendingIntent` and only calls `apply_request`
   for `MoveIntent` (and for `PendingIntent` in `OnPlace` mode there is
   none — `OnPlace` keeps emitting `MoveIntent` directly).
2. Route the confirm through the edge check: only the mover's
   `btn-{edge}` completes the pending intent; the other edge's press is
   a stray-press anomaly or a no-op.
3. Clear `pending_confirm` whenever the physically placed piece is
   lifted again (`on_lift` of the pending destination square) — that is
   the natural "take it back before confirming" gesture — and surface
   `InferenceOutput::Cancelled` so the UI can drop the pending state.
4. UI: pending destination highlight + status line "press your button
   to confirm" toward the mover's edge.

Files: `crates/tangibles/src/move_inference.rs` (`commit`,
`on_confirm_zone`, `on_lift`), `crates/tangibles/src/inference_output.rs`,
`crates/app/src/controller/tangibles.rs` (`on_game_field_event`).

## Reproduction

- Unit level: `confirm_zone_release_emits_buffered_intent` already shows
  the double-emission; add a controller-level test or drive
  `MockSource` end-to-end.
- Manual: `cargo run -p tangibles-chess -- --mock`, pick a tangible mode
  + `OnConfirmZone`, simulate lift `f52` / place `f36` (e2→e4 at 0°)
  then place on `btn-south`. Observe: move applied at place time,
  illegal flash on the zone press.
