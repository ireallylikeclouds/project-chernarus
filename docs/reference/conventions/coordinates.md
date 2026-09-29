# Coordinate conventions

The simulation stores positions in the frame that ARMA 2 scripts observe through `getPosASL`,
so captured traces and simulation traces compare without conversion (`chernarus_core::coords`).

| Convention | Value | Status | How it will be verified |
|---|---|---|---|
| World `x` | metres east | ESTIMATED | Capture: face east, walk forward, `dx` grows |
| World `y` | metres north | ESTIMATED | Capture: face north, walk forward, `dy` grows |
| World `z` | metres up; ASL for `getPosASL` | ESTIMATED | Capture on a slope; compare with terrain height |
| Heading | degrees clockwise from north, `[0, 360)`, as `getDir` | ESTIMATED | Capture validation table in `tools/capture/README.md` |
| Scale | 1 unit = 1 metre | ESTIMATED | Walk between map-grid lines 100 m apart |
| Origin | south-west corner of the terrain at `[0, 0]` | ESTIMATED | Compare catalogued terrain size with extreme positions |
| Chernarus extent | about 15 360 m × 15 360 m | ESTIMATED | Terrain header, once the WRP reader exists |
| Model space (P3D) | differs from world space (Y up in models) | ESTIMATED | Relevant once models are converted |

Nothing on this page has been checked against the reference yet.
