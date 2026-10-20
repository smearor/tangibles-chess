# tangibles-chess

Tabletop chess for the smart desk: two players sit opposite each other at a
large lying touchscreen (65"), each viewing and controlling the game from
their own side.

Inspired by GNOME Chess, with three key differences:

1. **Tangible modes** - play with physical figures: single-contact markers on
   the pieces are detected by the
   [tangible-objects](https://github.com/smearor/tangible-objects-rs) daemon
   via occupancy field sets (`a1`..`h8`-style cells plus action and tray
   zones), delivered over its GraphQL interface.
2. **Tabletop** - the screen lies flat; board and controls are rotated and
   duplicated so both players can read and operate everything.
3. **Rust** - a multi-crate workspace.
