# Architecture overview

Status: M0 (foundation). This page describes what exists and the rules new code must follow.
Decisions and their reasons are in [adr/](adr/).

## Shape of the project

```text
             ┌──────────────────────────── reference side (local only) ───────────────────────────┐
 reference   │  refscan ──► chernarus-assets::discovery ──► catalog.json (git-ignored)            │
 install ────┤                 └─ chernarus-formats (PBO, LZSS, raP, signatures)                  │
             │  ARMA 2 OA + chernarus_capture.sqf ──► RPT log ──► parity import-rpt ──┐            │
             └────────────────────────────────────────────────────────────────────────┼────────────┘
                                                                                       ▼
 data/*.toml ──► chernarus-sim (headless, fixed tick) ──► trace ──► chernarus-parity ◄── tests/parity/reference/*.csv
   (Param: value + status + source)                                   └─► report (verdict, statuses)
```

Nothing on the reference side writes into the repository except PROJECT-CREATED measurement
traces (our own observations, see [docs/assets/provenance.md](../assets/provenance.md)).

## Crates

| Crate | Path | Role | Depends on |
|---|---|---|---|
| `chernarus-core` | `crates/core` | Knowledge status (`Verification`), sourced parameters (`Param<T>`), coordinate conventions, fixed tick | — |
| `chernarus-formats` | `crates/formats` | Readers for reference formats (PBO, LZSS, raP), signature identification, path-string scanning, synthetic fixture writers | — |
| `chernarus-assets` | `crates/assets` | Provenance model and enforcement, repository guard, discovery pipeline, catalog and queries | formats |
| `chernarus-sim` | `crates/sim` | Deterministic gameplay simulation (M0: infantry movement) | core |
| `chernarus-parity` | `crates/parity` | Scenarios, traces, RPT import, metrics, sim-vs-reference comparison | core, sim |
| `chernarus-refscan` | `tools/refscan` | CLI `refscan` | assets, formats |
| `chernarus-parity-cli` | `tools/parity-cli` | CLI `parity` | parity, sim, core |

### Mapping to the GDD's suggested structure

The GDD suggests `engine/ simulation/ world/ assets/ rendering/ physics/ audio/ animation/
networking/ input/ tools/ server/ client/ tests/ docs/`. Library code lives in `crates/`
and a crate is created when its first real code exists (no empty placeholder crates):

| GDD area | Where it lives / will live |
|---|---|
| simulation | `crates/sim` (split into per-system crates once systems grow: player, weapons, inventory, medical, zombies, …) |
| assets | `crates/assets` (tooling), `assets/` (distributable runtime assets + provenance manifest) |
| tools | `tools/` |
| tests | per-crate `tests/`, plus `tests/parity/` for scenarios and reference traces |
| docs | `docs/` |
| engine, rendering, input, audio, client | future `crates/client` (+ `crates/render` if needed); see ADR 0003 |
| networking, server | future `crates/net`, `crates/server`; see ADR 0004 |
| world, physics, animation | future `crates/world`, `crates/physics`, `crates/anim`, when their first milestone starts |

## Layering rules

1. **Simulation never depends on presentation.** `chernarus-sim` (and any future gameplay crate)
   must not depend on rendering, windowing, audio, input devices or networking crates.
   Presentation reads simulation state; it never owns gameplay logic.
2. **Simulation never reads reference formats.** The runtime consumes project data (`data/`,
   `assets/`), never PBOs or other reference files. `chernarus-sim` must not depend on
   `chernarus-formats` or `chernarus-assets`.
3. **Behaviour numbers are data, with status.** Tunable values that are meant to reproduce the
   reference are `Param<T>` in `data/*.toml`: value, `status`, `source`. No magic numbers in
   gameplay code.
4. **Confidence propagates.** Simulation runs record the parameters they read (`ParamLedger`).
   A result is never reported as better known than the weakest parameter behind it.
5. **Deterministic, headless, fixed step.** Same inputs → bit-identical state on the same platform
   (tested). No wall-clock time, no unordered-map iteration in simulation logic.
6. **Readers never panic on input.** Discovery runs readers over whole installations; malformed
   files produce errors and scan issues.
7. **Small files, explicit modules.** One concern per module; no global mutable state.

## Data locations

| Path | Contents | Provenance |
|---|---|---|
| `data/` | Behaviour definitions (`Param`s with status/source) | PROJECT-CREATED (values are facts with citations) |
| `assets/` | Distributable runtime assets (none yet) + `provenance.toml` | per-file record, enforced |
| `tests/parity/scenarios/` | Parity scenario definitions | PROJECT-CREATED |
| `tests/parity/reference/` | Captured reference traces (our measurements) | PROJECT-CREATED observations |
| `reference/`, `reference-data/`, `local/` | Local reference install, catalogs, scratch | git-ignored, never committed |
