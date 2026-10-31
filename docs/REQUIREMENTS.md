# Tangibles Chess — Requirements

A chess application for a **65" horizontal tabletop smart desk**
(iiyama TF6539UHSC-class panel, ~1428×804 mm sensing surface). Two
players sit at opposite table edges facing each other; each views and
operates the game from their own side. The application combines
physical **Tangible Objects** (marker pieces detected by the touch
panel through a separate daemon) with normal touch input and optional
on-screen piece rendering.

Reference UX: GNOME Chess, extended by (a) a physical-piece mode, (b)
a true two-sided tabletop orientation, (c) a material, artistic
presentation.

This document is the normative requirements baseline. Requirement IDs
(`FR-*`, `NFR-*`, `IF-*`, `ARCH-*`, `X-*`) are stable — code, tests and
docs should reference them. Status markers: ✅ done, 🟡 partial,
⬜ open, 🐛 implemented but defective, 🔮 future (explicitly out of scope).

---

## 1. Hardware Context

- **H-1** Panel: large-format multitouch, lies flat on the desk.
  Up to **50 simultaneous contacts** (iiyama spec). This budget
  covers 32 piece markers + ~18 contacts for hands/touch UI. ✅ assumed
- **H-2** Players sit opposite each other; every interactive element
  must be reachable and readable from both table edges. ✅
- **H-3** Tangible objects are detected through the **standalone
  `tangible-objects` daemon** (`github.com/smearor/tangible-objects-rs`),
  which reads the panel via evdev and exposes occupancy fields over
  GraphQL. The app never links the daemon's evdev/engine crates. ✅
- **H-4** Marker design: **one contact per piece** plus occupancy
  fields (not multi-pin patterns). 32 markers on the board, extra
  markers optional for button zones. ✅

## 2. Terminology

- **Tangible Objects / tangibles**: physical pieces whose contacts the
  daemon tracks.
- **Field / field set**: daemon-side rectangles (mm) reporting
  `FIELD_PLACED`/`FIELD_LIFTED`/`FIELD_COUNT_CHANGED` events.
- **Field index `f0`–`f63`**: screen-fixed board cells, row-major from
  the board rect's top-left screen corner. `f*` is positional, *not* a
  chess square — `BoardGeometry` maps index→square under the current
  orientation.
- **Zones**: non-board fields `btn-south`, `btn-north` (confirm),
  `tray-south`, `tray-north` (captured-piece count), plus the parking
  strips `cap-west-0..7` and `cap-east-0..7` (per-cell occupancy for
  the rendered captured pieces beside the board).
- **Handshake**: the physical-placement phase before a tangible game
  starts — on by default, can be disabled via `require_placement`
  (debug aid for incomplete physical setups).
- **South/North**: the two table edges; each player "owns" one edge.
- **Wire mirror / projection**: this app's local domain types fed from
  upstream model types via `From` conversions (see ARCH-4).

## 3. Play Modes (game-wide setting)

- **FR-MODE-1** `Tangibles`: physical markers are the pieces; the UI
  renders board, highlights, guides and panels — **no piece sprites**.
  Promotion is signalled via badge/picker UI. ✅
- **FR-MODE-2** `TangiblesRendered`: same input, plus rendered digital
  pieces as a control/overlay; physical↔logical discrepancies are
  visualized (anomaly state). ✅ persistent discrepancy rings via
  `update_discrepancy`
- **FR-MODE-3** `NoTangibles`: pure touch chess with rendered pieces;
  no daemon needed; always available (must work daemon-less). ✅
- **FR-MODE-4** Mode selection happens on the setup screen and applies
  to the whole game (the board is shared). ✅
- **FR-MODE-5** In both tangible modes `suppress = true` is set on the
  field set so marker contacts never reach the OS; touch gestures on
  board fields are therefore intentionally dead — panel buttons outside
  fields stay usable. ✅ (by design)

## 4. Chess Rules & Game State (crate `tangibles-chess-core`)

- **FR-CORE-1** All chess legality, SAN, FEN, outcomes come from
  **`shakmaty` (0.30)** — no reimplemented rules. ✅
- **FR-CORE-2** `GameSession` owns: position, move history (UCI+SAN),
  phase (`Setup`/`AwaitingPlacement`/`InProgress`/`Finished`), outcome,
  draw state, undo stack. ✅
- **FR-CORE-3** `MoveRequest` covers normal moves, captures, castling
  and promotion (explicit role). ✅
- **FR-CORE-4** Terminal conditions via shakmaty: checkmate, stalemate,
  insufficient material, 50-move rule, threefold repetition (via
  Zobrist history), plus agreed draw and resignation. ✅
- **FR-CORE-5** Draw agreement between two human players; claimable
  draws surfaced to the UI. 🟡 — implemented as single-click
  `agree_draw` (either side can draw instantly); a real offer/accept
  handshake between the two panels is still missing (FR-UI-6)
- **FR-CORE-6** `UndoPlan` computes the *physical* rebuild steps for a
  takeback (move-back list, captured-piece re-placement, castling rook,
  promotion reversal) so tangible undo can be guided on the board. ✅ core
  ⬜ guided UI — the plan is currently only `debug!`-logged; undo/redo
  buttons work logically but give the player no physical guidance
- **FR-CORE-7** `SavedGame` (TOML) persists FEN-replayable move list +
  config + end state after every move (autosave). ✅
- **FR-CORE-8** Resume: a saved game can be rebuilt
  (`GameSession::from_saved`); in tangible modes the placement gate
  validates the *resumed* position. ✅

## 5. Tangible Integration (crate `tangibles-chess-tangibles`)

### 5.1 Daemon communication

- **FR-TAN-1** All daemon I/O goes through the upstream
  **`tangible-objects-client`** typed GraphQL client (cynic,
  schema-checked) — HTTP queries/mutations + `graphql-transport-ws`
  subscription. No hand-written wire code. ✅
- **FR-TAN-2** `TangibleSource` trait abstracts the daemon so tests and
  `--mock` dev work without hardware: `events()`, `device_info()`,
  `touch_points()`, `register_field_set()`, `unregister()`.
  Implementations: `GraphQlSource` (adapter over `GraphQLClient`),
  `MockSource` (scriptable). ✅
- **FR-TAN-3** Field set `chess` registered with owner `tangibles-chess`
  and `Lifetime::Session`; unregistered on cleanup. ✅
- **FR-TAN-4** Geometry: `FieldSetBuilder` emits 64 board cells
  (absolute screen-mm rects from device calibration + rendered board
  rect) + zones (`btn-*`, `tray-*`, and `cap-*` parking cells where the
  surface has room); the set must satisfy `FieldSet::is_disjoint()` (the
  daemon rejects overlapping fields — enforced in tests). ✅
- **FR-TAN-5** `deviceInfo` supplies mm-per-pixel calibration; missing
  data degrades gracefully to estimated geometry. ✅
- **FR-TAN-6** `touchPoints` drives the `OccupancyProbe` resync after
  `LAGGED` events and at handshake. ✅

### 5.2 Move inference (state machine over field events)

- **FR-TAN-7** A move is a lift (origin) followed by place
  (destination); the piece "in the air" is tracked per turn color. ✅
- **FR-TAN-8** Capture sequence: lift own piece → opponent lift on
  destination (marked captured) → place own piece → move committed. ✅
- **FR-TAN-9** Castling: king moves two files ⇒ castle intent; the
  subsequent physical rook hop is *swallowed* (no double move). The
  `Move::Castle` variant stores origin squares — the user-visible king
  destination is derived. ✅
- **FR-TAN-10** Promotion: a reached last rank with a pending promotion
  request; UI shows the picker toward the promoting player. ✅
- **FR-TAN-11** `CommitMode::OnPlace` (casual: move commits on place)
  vs `CommitMode::OnConfirmZone` (strict: pending until the mover's
  `btn-*` zone reports a placed contact — clock-puck style). ✅ inference 🐛 UX —
  the controller applies the display intent immediately; see
  `docs/bugs/on-confirm-zone-commit-order.md`
- **FR-TAN-12** Tray zones (`tray-*`) count captured pieces per side;
  `cap-*` parking cells track occupancy and render the captured pieces
  beside the board. ✅
- **FR-TAN-13** Anomalies (unexpected lift/place, extra contact, foreign
  field) produce `InferenceOutput::Anomaly` events, surface in the UI
  and trigger occupancy resync; the game must never silently corrupt
  board state. ✅ events + status-line surfacing ⬜ anomaly-triggered
  resync + recovery UX (resync currently only on `LAGGED`)
- **FR-TAN-14** `LAGGED` subscription events and stream errors force a
  `touchPoints` resync — the physical board is source of truth. ✅
  (stream errors are translated into `Lagged` events; parking strips
  resync from raw touches too)
- **FR-TAN-15** `BoardGeometry` permutes field index→square under
  0/90/180/270° orientations; white/black edges tracked independently. ✅

### 5.3 Start handshake (mandatory by default)

- **FR-TAN-16** Tangible games cannot start until physical placement
  matches the expected position (initial or resumed):
  `GamePhase::AwaitingPlacement` + `PlacementGate` comparing
  `OccupancyProbe` against `shakmaty` board. ✅ — the gate can be
  disabled per game via `GameConfig::require_placement` (debug aid for
  incomplete physical setups)
- **FR-TAN-17** Handshake UI: dual-facing overlay, progress "n/32",
  missing + foreign-occupied fields highlighted on the board; fields
  light up when filled correctly (setup ceremony). ✅ — implemented as
  per-side status lines (own `placed/total` + first missing/foreign
  hint), missing/foreign markers and ceremony lighting on the scene
- **FR-TAN-18** Placement check remains armed after the handshake as
  the anomaly trigger. 🟡 — during play the `MoveInference` anomaly
  stream covers this; a persistent physical↔logical comparison exists
  only in `TangiblesRendered` (discrepancy rings, FR-MODE-2)

## 6. Tabletop UI (crate `tangibles-chess`, GTK4 + libadwaita)

### 6.1 Rotation & layout

- **FR-UI-1** The whole tabletop surface (board + both player panels)
  lives inside **`smearor-wrot-rotation`'s `RotationWidget`**:
  rotated bounding box, inverse input transform, two-finger rotate
  gesture with 90° snap, animated transitions. ✅
- **FR-UI-2** Board rendering is always logical `a8`-top-left…`h1`
  (south-white orientation); the RotationWidget supplies physical
  rotation — rendering and physical field mapping are separate
  concerns kept in sync via `BoardOrientation`. ✅
- **FR-UI-3** Rotation is **locked during active tangible games**
  (resting markers would remap wrongly); free in `NoTangibles` and
  during setup; rotate buttons + gesture on both sides. ✅
- **FR-UI-4** Dual-facing design: every interactive control exists on
  both edges — `DualFacingRow` renders the same control group twice,
  one copy rotated 180°, both bound to the same state/actions. ✅
- **FR-UI-5** Touch targets ≥ ~12 mm; large type; high contrast; no
  desktop chrome (fullscreen). ✅ baseline 🟡 visual polish
- **FR-UI-6** Dialog orientation: promotion picker faces the moving
  player; draw offers face the opponent; resign confirmations face
  the requester. 🟡 — promotion picker ✅ (rotated toward the mover's
  edge); draw offer/accept flow and resign confirmation ⬜ (both
  actions currently take effect on a single click)

### 6.2 Widgets

- **FR-UI-7** `ChessScene`: custom Cairo-rendered board widget —
  material themes, coordinates, selection + legal-target dots,
  check highlight, last-move trail, lifted/ghost pieces, ceremony
  states, missing/foreign markers, optional piece glyphs. ✅
- **FR-UI-8** Player panels on both edges: name/color, turn indicator,
  clock placeholder (M3), captured-material tray display, action
  buttons (resign, draw, undo, rotate) with icon-set icons. ✅ — tray
  count label + `cap-*` parking strips render captured pieces;
  ⬜ clock placeholder
- **FR-UI-9** **Score sheets** on both long edges, each upright to its
  player (notation in SAN); clicking a ply seeks the replay cursor in
  finished games. ✅
- **FR-UI-10** Setup screen: mode/theme/orientation/commit/opponent/
  sound controls + big start button, dual-facing; resume card when an
  autosave exists (with move count). ✅ — also `require_placement`
  and `allow_undo` toggles; engine opponent shown greyed-out (M3)
- **FR-UI-11** Daemon status: setup probes the endpoint (GraphQL
  `__typename` ping); tangible mode pickers grey out when the daemon
  is unreachable. ✅
- **FR-UI-12** Handshake overlay (FR-TAN-17). ✅
- **FR-UI-13** `DiagnosticsOverlay`: live per-field contact counts +
  raw touch points for calibration/hardware bring-up. ⬜ M2
- **FR-UI-14** Replay mode for finished games: the result overlay
  offers a replay entry that steps through the recorded plies —
  score-sheet clicks seek, undo/redo buttons walk the cursor, physical
  board events feed only the discrepancy display while replaying. ✅
  (added beyond the original scope)

### 6.3 Artistic direction — "edles, intelligentes, rustikales Schach"

- **FR-ART-1** *Licht unter den Figuren* (signature feature): lifting a
  physical piece dims its origin and makes legal targets glow beneath
  the piece in hand; check = warm red under the king; last-move light
  trail; calm fades, no bouncy animation. ✅ baseline — lifted-square +
  legal-target glow on `FIELD_LIFTED`, check highlight, last-move
  trail and illegal-move flash with check projection implemented
- **FR-ART-2** Material themes `BoardTheme { WalnutMaple, Slate,
  TournamentGreen }` — wood-inlay look, engraved-edge coordinates,
  serif clocks, leather/wood panels; selectable + persisted. ✅ 3 themes
- **FR-ART-3** Score sheet styled like a physical scoresheet; captured
  trays double as physical zones (FR-TAN-12). ✅ — parking strips hold
  the physical captured pieces next to their rendered counterparts
- **FR-ART-4** Sound character: soft wooden knock on place, deeper
  strike on capture; distinct check/promotion/game-end/error cues. ⬜ M3 —
  `SoundPlayer` honors the persisted flag and degrades to no-op, but
  `assets/sounds/` is empty and `SoundEffect` covers only
  move/capture/check/end (castle, promotion, error cues missing)

## 7. Icons & Pieces

- **FR-ICON-1** Custom icon set crate `tangibles-chess-icons` built via
  `fonts-rs` `IconSetBuild` (icons.toml → `tch-*-symbolic` names,
  symbolic-themeable, Lucide/Tabler sources + local `dir` escape
  hatch). ✅
- **FR-ICON-2** Digital pieces: **Cuernavaca chess glyphs**
  (`fonts-rs-cuernavaca`, `Cuernavaca::install()` + icon-theme
  registration; `cuernavaca-k..p`/`l..o` names). ✅
- **FR-ICON-3** Icon install is idempotent and runs at app start. ✅

## 8. Internationalization & Sound

- **FR-I18N-1** Fluent (`fluent`, `fluent-bundle`, `fluent-langneg` +
  `icu_locid`) — `.ftl` resources under `i18n/{en,de}/`, embedded via
  `include_str!`, negotiated fallback to English. ✅ en+de
- **FR-I18N-2** `--locale` flag overrides system locale. ✅
- **FR-SND-1** `SoundPlayer` abstraction honoring a persisted
  sound-enabled flag; event hooks for move/capture/castle/check/
  promotion/end/error. 🟡 — player + persisted toggle ✅; hooks wired
  for move/capture/check/end; ⬜ castle/promotion/error cues +
  `assets/sounds/*.ogg` (FR-ART-4)

## 9. Persistence & CLI

- **FR-PER-1** Autosave TOML in `glib::user_data_dir()/tangibles-chess/`
  after every move; corrupted/absent files never block startup. ✅
- **FR-PER-2** `AppSettings` (TOML, config dir): last config,
  daemon URL, sound flag, theme. ✅
- **FR-CLI-1** Flags: `--daemon-url`, `--locale`, `--mock`, plus
  `--mcp-listen` (game-domain MCP endpoint, see X-1). ✅

## 10. Engine, Clocks, PGN (M3)

- **FR-ENG-1** AI opponent via external **UCI engine subprocess**
  (`tangibles-chess-engine`: spawn/uci/position/go/bestmove, run off
  the GTK thread). 🟡 — `EngineClient` + `OpponentKind::Engine` exist;
  the setup picker shows the engine greyed-out; ⬜ gameplay wiring
- **FR-ENG-2** Optional auto-rotation toward the active player in
  `NoTangibles`. ⬜
- **FR-ENG-3** Chess clocks (increment/fixed), per-edge display. ⬜
- **FR-ENG-4** PGN load/save. ⬜ — `pgn-reader` is declared in the
  workspace deps but not yet used by any crate

## 11. Architecture (ARCH-*)

- **ARCH-1** Multi-crate Rust workspace, edition 2024, `rust-version
  1.92`: `config` (setup enums + `GameConfig`) · `geometry` (orientation,
  field/scene indices) · `notation` (`Fen`/`San`/`Uci`/`SquareName`
  newtypes) · `core` (domain, shakmaty — re-exports the three above) ·
  `tangibles` (daemon adapter + physical interpretation) · `engine`
  (UCI) · `icons` (generated icon set) · `mcp` (game-domain MCP
  endpoint, X-1) · `app` (GTK4/libadwaita). ✅
- **ARCH-2** Domain layers have **no GTK deps**; the app owns the only
  main loop; daemon runtime = dedicated tokio runtime thread
  (`TangibleHost`), events hop back via `async_channel`. ✅
- **ARCH-3** `GameController` bridges session ↔ tangibles ↔ widgets;
  thread-local active controller for callbacks that cannot retain it
  (promotion actions). ✅
- **ARCH-4** Upstream boundary: daemon access exclusively through
  `tangible-objects-{client,model}` git deps; this crate keeps slim
  **domain projections** (`DeviceInfo`, `TouchPoint`, `FieldEvent`)
  with `From` conversions — deliberate isolation, not wire
  duplication. ✅
- **ARCH-5** Internal `GameEvent`/`ControlCommand`-style channels stay
  cleanly bounded so a game-domain service can attach. ✅ — realized:
  `tangibles-chess-mcp` exposes the session via MCP over Streamable
  HTTP (`--mcp-listen`); commands hop onto the GTK main loop through a
  bounded channel (X-1)
- **ARCH-6** Coding rules per `AGENTS.md`: panic-free, rustdoc on all
  public items + fields/variants, one type per file, individual sorted
  imports, `debug!` macro imports, thiserror/miette errors, inline
  tests for success+failure paths, `cargo fmt`. ✅
- **ARCH-7** System requirement: **GTK ≥ 4.20** (rotation dep uses
  `v4_20`). ✅

## 12. Dependencies (normative)

| Crate | Source | Role |
|---|---|---|
| shakmaty 0.30 | crates.io | rules, SAN, FEN, outcomes |
| smearor-wrot-rotation 0.2 | crates.io | board/UI rotation |
| tangible-objects-client / -model | git `smearor/tangible-objects-rs` | daemon GraphQL |
| fonts-rs-cuernavaca etc. 0.6 | git `smearor/fonts-rs` | icon set + chess glyphs |
| gtk4 0.11 / libadwaita 0.9 | crates.io | UI |
| fluent* / icu_locid | crates.io | i18n |
| tokio / async-channel / serde / thiserror / miette / tracing | crates.io | plumbing |

License: GPL-3.0-or-later (shakmaty/pleco-space is GPL; fonts-rs icon
data MIT/Apache, Cuernavaca OFL — recorded in NOTICE).

## 13. Verification & Acceptance

- **V-1** `cargo build/test/clippy --workspace` green, `cargo fmt
  --check` clean. ✅ (74 tests)
- **V-2** `NoTangibles` playable end-to-end on desktop without daemon. ✅ baseline
- **V-3** Manual tabletop checks: rotation 0/90/180/270°, touch
  accuracy, both-edge readability. ⬜
- **V-4** Daemon integration against `--replay` fixture, then real
  hardware: registration accepted (disjoint set), handshake, inference,
  trays, commit zones. ⬜ M2
- **V-5** Daemon offline → app still starts; tangible modes disabled;
  `NoTangibles` fully functional. ✅

## 14. Explicitly Out of Scope (X-*)

- **X-1** Game-domain service (remote players, agent opponent,
  spectator stream, E2E automation) — **partially realized**: the
  `tangibles-chess-mcp` crate exposes an opt-in MCP endpoint over
  Streamable HTTP (`--mcp-listen`) with tools for game state, legal
  moves, make-move, undo, resign, draw, placement status and window
  screenshots plus a game-state subscription. Still out of scope:
  remote/network play and agent-vs-agent games (X-4).
- **X-2** Direct evdev/hardware access in the app — daemon only.
- **X-3** Multi-pin marker patterns — one-contact markers only.
- **X-4** Online/network play.

## 15. Milestones

- **M1 ✅** workspace/infrastructure, core, `NoTangibles` app, rotation,
  dual-facing UI, setup (+resume/daemon probe), promotion, undo,
  score sheets, themes, i18n, icons.
- **M2 🟡** tangible modes live (`--replay` → hardware), handshake
  ceremony polish, light-guide refinement, guided undo, diagnostics
  overlay, anomaly recovery UX, ~~captured-tray display~~,
  ~~owner-based deregistration~~, `OnConfirmZone` commit fix
  (`docs/bugs/on-confirm-zone-commit-order.md`), draw/resign
  confirmation flows (FR-UI-6). Done since: replay mode, discrepancy
  rings, parking strips, MCP endpoint (X-1).
- **M3 ⬜** UCI engine wiring, clocks, PGN, sound assets + remaining
  cues, more themes, auto-rotation, animation polish.
