# Tangible Setup

The tangible modes need the `tangible-objects` daemon running with the filter
proxy and the GraphQL interface:

```bash
tangible-objects --filter --device /dev/input/eventX \
    --graphql-listen 127.0.0.1:8080
```

The daemon grabs the physical touchscreen (`EVIOCGRAB`), detects contact
patterns and re-emits non-tangible touches via a uinput clone. Contacts inside
registered field cells are suppressed (`suppress = true`), so markers on the
board never tap the UI.

## Field set

On game start the app registers a field set via `registerFieldSet`:

- `f0`..`f63` - the 64 board cells, indexed in *screen* geometry (row-major),
  normalized 0..1 inside the `[window]` board rectangle. Screen-relative naming
  keeps the set stable across board rotations; `BoardGeometry` permutes field
  indices to logical squares through the current orientation.
- `btn-south` / `btn-north` - action zones next to the board used as
  physical confirm/clock buttons.
- `tray-south` / `tray-north` - captured-piece trays; occupancy counts as
  material taken off the board.

## Markers

Each physical figure carries one conductive marker on its base. The panel
handles up to 50 simultaneous contacts - 32 markers plus regular touches.
