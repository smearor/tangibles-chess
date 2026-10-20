# tangibles-chess

Tabletop chess for the smart desk: a GTK4/libadwaita chess game for a large
lying touchscreen (65") where two players sit opposite each other. Inspired by
GNOME Chess, with a tangible-figures mode powered by
[tangible-objects](https://github.com/smearor/tangible-objects-rs).

## Modes

| Mode | Input | Pieces |
|------|-------|--------|
| Tangibles | physical single-contact markers on the screen | not rendered |
| Tangibles + rendered | physical markers | rendered for control |
| Touch | tap/drag on the board | rendered |

## Requirements

- Rust 1.92+ (Edition 2024)
- GTK >= 4.20, libadwaita >= 1.5 (`libgtk-4-dev libadwaita-1-dev`)
- For tangible modes: the `tangible-objects` daemon running with
  `--filter --graphql-listen 127.0.0.1:8080`

## Build & Run

```bash
cargo build --workspace
cargo run -p tangibles-chess
```

## Documentation

- `book/` - mdBook documentation (architecture, tangible setup, calibration)
- `AGENTS.md` - code conventions

## License

GPL-3.0-or-later. Bundled fonts/icons/sounds keep their own licenses (see
[NOTICE](./NOTICE)).
