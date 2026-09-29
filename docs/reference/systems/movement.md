# Movement

Parity level: **P1** (conceptually implemented). Confidence: **low**.

## What the reference is believed to do (ESTIMATED)

- Infantry motion is **animation-driven**. Each movement state (stance × pace × direction ×
  weapon state) is a class in the `CfgMoves*` configs. It names an RTM animation, a playback
  `speed`, and interpolation and transition settings. The animation's root displacement moves the unit.
- State changes blend between animations over an interpolation time, which is where observable
  acceleration and deceleration come from. There is probably no separate acceleration constant.
- Stance changes are transition animations of fixed length, during which normal movement input is not applied.
- Terrain slope, the weapon held, fatigue and injury (for example DayZ fractures forcing prone)
  change the active states. All UNKNOWN in detail.

## Two independent routes to the numbers

1. **Data extraction:** read the movement state classes from the effective config (needs config
   merging) and the RTM step vectors (needs the RTM reader). Ground speed of a state ≈ step length ×
   playback speed. Status after extraction: PARTIALLY VERIFIED.
2. **Measurement:** the scenarios in `tests/parity/scenarios/movement_*.toml`, captured with
   `tools/capture/chernarus_capture.sqf`. They measure steady speed, time to 90% of steady speed,
   and stopping distance. VERIFIED needs three consistent runs.

The two routes should agree. If they do not, the explanation (blending, frame-rate dependence,
slope) is itself a finding.

## Current implementation (not the reference model)

`crates/sim/src/movement.rs` is a kinematic stand-in with target speed per
(stance, pace, direction), bounded acceleration and deceleration, and fixed stance-transition
durations. It exists so the measure-and-compare chain runs end to end. All 53 values in
`data/movement/infantry.toml` are **UNKNOWN placeholders**.

## Known differences

- Structure: kinematic ramps instead of animation blending.
- Values: none measured.
- Left/right symmetry, diagonal speeds, slope effects, weapon-state effects, fatigue: not modelled, all UNKNOWN.

## Open questions

- Is the frame rate at capture time a factor in measured speeds?
- Does DayZ Mod override vanilla movement configs for its survivor classes?
- What input model does ARMA 2 OA use for walk/run/sprint (toggle vs hold; default bindings)?
