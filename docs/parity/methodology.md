# Parity methodology

> Observe → Specify → Implement → Measure → Compare → Correct

## Scenario

A TOML file in `tests/parity/scenarios/` (format: `crates/parity/src/scenario.rs`) holds:

- **what to do**: initial state and an input timeline (`[[input]]` segments), for the simulation;
  plus a human `capture_procedure` for the reference game;
- **how to measure**: `[analysis]` settings and `[[metric]]` entries with tolerances.

## Measurement

Simulation traces and reference traces go through **the same metric code**
(`crates/parity/src/metrics.rs`):

1. Horizontal displacement from the first sample, linearly interpolated (handles the irregular
   frame timing of captures).
2. **Onset**: the first time displacement exceeds `onset_threshold_m`. All metrics are relative to
   onset, so the unknown moment a human pressed a key does not matter. The detection bias
   (onset is seen slightly after motion starts) is the same on both sides.
3. Metrics (M0):
   - `steady_speed`: least-squares slope over `steady_window_s` after onset, m/s;
   - `time_to_fraction`: time from onset until speed first reaches the fraction of steady speed, s;
   - `stop_distance`: distance covered after speed first falls below the fraction of steady speed
     (after the steady window) until the end, m.

   Speed is a central difference over `speed_half_window_s`. These metrics assume straight-line motion.

## Verdicts and statuses

For each metric: the simulation value, the reference mean ± sd over runs, the difference and the tolerance.

| Verdict | Meaning |
|---|---|
| `PASS` | \|sim − reference mean\| ≤ tolerance for every metric |
| `FAIL` | at least one metric outside tolerance |
| `NO_REFERENCE` | no reference runs; nothing is compared and nothing is invented |
| `SIM_UNMEASURABLE` | the simulation trace lacks what a metric needs (a scenario or implementation bug) |

The report also states:

- **Reference behaviour status**: UNKNOWN with 0 runs, PARTIALLY VERIFIED with 1–2 runs or
  inconsistent runs (sd > tolerance), VERIFIED with ≥ 3 runs and sd ≤ tolerance for every metric.
- **Simulation inputs status**: the weakest status among the data parameters the run actually used.
  A PASS computed from UNKNOWN placeholders is a lead to promote those values, not evidence of parity.

## Tolerances

Tolerances are project decisions, recorded in each scenario, and should be comparable to
reference run-to-run spread. They are revisited once real captures show that spread.

## Running

```sh
cargo run -p chernarus-parity-cli -- run                       # every scenario, markdown report
cargo run -p chernarus-parity-cli -- run --out local/parity    # + JSON reports and sim traces
cargo run -p chernarus-parity-cli -- import-rpt ArmA2OA.RPT    # add reference captures
```

The harness itself is tested with synthetic references (simulation output rendered as RPT logs) in
`crates/parity/tests/parity_loop.rs`. Those tests prove the harness, not parity.
