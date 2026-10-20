# Calibration

## Screen size in millimeters

The app queries `deviceInfo { xRes yRes xMin xMax yMin yMax }` from the daemon
and computes the screen size in millimeters (`x_max / x_res`). The board
rectangle in window pixels is scaled to millimeters with square pixels
assumed; the field set is registered normalized (`0.0..=1.0`) inside that
board rectangle.

## Player sides

Configure which physical table edge belongs to White in the setup screen.
`BoardGeometry` combines that side assignment with the current
`RotationWidget` orientation to permute field indices onto logical squares.

## Diagnostics overlay

Enable the diagnostics overlay in the settings to see live per-field contact
counts and raw touch points - useful to verify that the rendered board aligns
exactly with the sensor area.
