# Reference capture harness

`chernarus_capture.sqf` is a small script, written for this project, that runs **inside the
reference game** (ARMA 2: Operation Arrowhead with DayZ Mod) and logs what the player's unit
actually does: position, heading, speed and animation state, once per frame. The `parity` tool
turns those logs into reference traces and compares them with the Rust simulation.

**Status: UNTESTED.** Nobody has run it against the reference game yet. The first capture session
must start with the validation steps below.

## What you need

- A legitimate ARMA 2 OA installation with the DayZ Mod version chosen as the reference
  (not yet pinned; see [docs/reference/reference-installation.md](../../docs/reference/reference-installation.md)).
- The 2D editor (singleplayer), so scripts can run. Multiplayer DayZ servers do not allow this.

## Capturing a scenario

1. Pick a scenario in [`tests/parity/scenarios/`](../../tests/parity/scenarios). Its
   `capture_procedure` says exactly what to do with the keyboard.
2. In the editor, on Chernarus, place a playable unit. Record which class you used; the DayZ
   survivor class is preferred because DayZ may change movement configs (UNKNOWN). Pick
   flat, open, level ground.
3. Put `chernarus_capture.sqf` in the mission folder (`Documents\ArmA 2\missions\<name>.Chernarus\`).
4. Preview. Start the capture from a radio trigger or the unit's init line, for example
   `["movement.stand_run_forward", 12] execVM "chernarus_capture.sqf";`
5. Perform the procedure. Repeat for **at least three runs** (three consistent runs are what
   the harness needs to call a reference behaviour VERIFIED).
6. Quit, find the newest `ArmA2OA.RPT`, and import it, recording the platform and frame rate:

   | Platform | Where the RPT usually is (ESTIMATED) |
   |---|---|
   | Windows | `%LOCALAPPDATA%\ArmA 2 OA\` |
   | Linux, Steam Play (Proton) | `<steam library>/steamapps/compatdata/<app id>/pfx/drive_c/users/steamuser/AppData/Local/ArmA 2 OA/` |
   | Linux, Wine | `<wine prefix>/drive_c/users/<user>/AppData/Local/ArmA 2 OA/` |

   On Linux, `find ~ -iname '*.rpt' -newermt '-1 hour' 2>/dev/null` finds it wherever the prefix is.

   ```sh
   cargo run -p chernarus-parity-cli -- import-rpt path/to/ArmA2OA.RPT --meta platform=windows --meta fps=60
   #   … or --meta platform=proton-<version> / wine-<version>
   cargo run -p chernarus-parity-cli -- run
   ```

   Traces are written to `tests/parity/reference/<scenario>/<run>.csv`. They contain only our own
   measurements and may be committed. Do not commit the RPT file itself: it contains machine
   paths and unrelated engine output, and `.gitignore` excludes it.

## Validating the harness (do this first)

| Check | How | Expected |
|---|---|---|
| Script runs at all | Start a 5 s capture standing still | RPT has BEGIN, ~5 s of SAMPLE lines, END; no script errors |
| Zero drift | Same, not touching anything | `dx dy dz` stay at 0 (to logging precision) |
| Axes | Face north (`getDir` ≈ 0), walk forward | `dy` grows, `dx` stays near 0 |
| Heading | Face east | `getDir` ≈ 90 and forward motion grows `dx` |
| Scale | Walk between two map-grid lines 100 m apart | ~100 m of displacement |
| Sampling | Look at `t` spacing | Roughly frame time; note the FPS during capture |

Record the outcome in [docs/reference/conventions/coordinates.md](../../docs/reference/conventions/coordinates.md)
and raise the statuses there accordingly.

## Known limitations

- Commands used (`diag_log`, `diag_tickTime`, `productVersion`, `getPosASL`, `getDir`, `speed`,
  `animationState`) are believed to exist in ARMA 2 OA 1.6x (ESTIMATED). If `productVersion` is
  missing in the chosen build, remove it from the BEGIN line and record the version by hand.
- Human key timing is imprecise. The parity metrics align on motion onset and measure steady-state
  speed, acceleration and stopping, which do not depend on when exactly a key was pressed.
- Frame-rate effects on movement are UNKNOWN; record the FPS of each session with `--meta fps=…`.
- Platform effects (native Windows vs Proton/Wine) are UNKNOWN; record them with `--meta platform=…`
  and keep platforms separate until runs are shown to agree.
