# Project Chernarus

A high-fidelity Rust reimplementation of **ARMA 2: Operation Arrowhead + DayZ Mod**, built by
measuring the original and reproducing its observable behaviour. Not a DayZ-inspired game.

> Observe → Specify → Implement → Measure → Compare → Correct

The governing brief is [docs/design/gdd.md](docs/design/gdd.md).

## Status: milestone M0 (foundation)

There is **no playable game and no renderer yet**. M0 proves the research loop:

```text
Inspect reference → discover assets → document behaviour → represent data → run simulation → compare
```

| Piece | State |
|---|---|
| Reference discovery (`refscan`): PBO / LZSS / rapified config readers, format identification, dependency graph, asset queries | Working; validated on a synthetic installation only |
| Asset provenance (REFERENCE / LICENSED / PROJECT-CREATED / UNKNOWN) with repository guard | Enforced in tests and CI |
| Headless deterministic simulation: infantry movement | P1; every parameter an UNKNOWN placeholder |
| Parity harness (`parity`): scenarios, RPT capture import, metrics, verdicts | Working; no reference captures exist yet, so every verdict is `NO_REFERENCE` |
| In-game capture script (`tools/capture/chernarus_capture.sqf`) | Written, untested in game |

See the [parity matrix](docs/parity/parity-matrix.md) and the [M0 report](docs/reports/2026-09-29-m0-foundation.md).

## Legal boundary

This repository contains only new code, tools, documentation and our own measurements. It
never contains Bohemia Interactive or DayZ Mod files (executables, PBOs, models, textures, sounds,
animations, maps, scripts or configs), in original or converted form. Tests and CI enforce this
([ADR 0005](docs/architecture/adr/0005-reference-data-boundary.md),
[provenance](docs/assets/provenance.md)). You need your own legitimate installation as the reference.

## Quick start

Works on Linux and Windows (both covered by CI). The reference game itself runs on Windows, or on
Linux via Steam Play/Proton or Wine ([details](docs/reference/reference-installation.md#linux)).

```sh
cargo test --workspace                                   # all checks, including the asset boundary
cargo run -p chernarus-parity-cli -- run                 # run parity scenarios

# Discovery demo on the synthetic (invented) installation
cargo run -p chernarus-refscan -- synth-install local/synth
cargo run -p chernarus-refscan -- scan local/synth --out reference-data/synth.json
cargo run -p chernarus-refscan -- query reference-data/synth.json 'synth\core\data\thing_co.paa'

# Your reference installation (outputs stay in git-ignored reference-data/)
cargo run -p chernarus-refscan -- scan "/path/to/ARMA 2 Operation Arrowhead"
```

## Layout

```text
crates/core      knowledge status, sourced parameters, coordinates, fixed tick
crates/formats   PBO, LZSS, raP readers; format identification; fixture writers
crates/assets    provenance + guard; discovery, catalog, queries
crates/sim       deterministic simulation (M0: movement)
crates/parity    scenarios, traces, RPT import, metrics, comparison
tools/refscan    CLI: discovery and provenance
tools/parity-cli CLI: parity runs and capture import
tools/capture    SQF capture harness for the reference game
data/            behaviour parameters (value + status + source)
assets/          distributable assets + provenance manifest (empty)
tests/parity/    scenarios and reference traces
docs/            design, architecture, reference, parity, assets, reports
```
