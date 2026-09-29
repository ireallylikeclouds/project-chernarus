# Parity matrix

Last updated: 2026-09-29 (milestone M0). Levels and statuses are defined in [README.md](README.md).

**No system has been compared against the reference game yet.** No reference installation or
capture has been available, so every confidence is Low or none.

## Gameplay systems

| System | Level | Confidence | Reference status | Difference / notes | Next step |
|---|---|---|---|---|---|
| Movement | **P1** | Low | UNKNOWN (0 captures) | Kinematic stand-in, not animation-driven; all 53 parameters are UNKNOWN placeholders; 3 scenarios defined (walk/run/sprint forward) and runnable, verdict NO_REFERENCE | Capture 3 runs per scenario; RTM step reader |
| Camera (first person) | P0 | — | UNKNOWN | — | Needs renderer (ADR 0003) |
| Weapons | P0 | — | UNKNOWN | — | Config merge → `CfgWeapons`/`CfgAmmo` extraction |
| Inventory | P0 | — | UNKNOWN | — | Document gear slots from the reference |
| Zombies | P0 | — | UNKNOWN | — | Read DayZ zombie scripts (pinned version) |
| Medical | P0 | — | UNKNOWN | — | Read DayZ medical scripts |
| Survival (hunger, thirst, temperature) | P0 | — | UNKNOWN | — | Read DayZ survival scripts |
| Loot | P0 | — | UNKNOWN | — | Locate loot definitions in the pinned version |
| Vehicles | P0 | — | UNKNOWN | — | After Phase 2 |
| World | P0 | — | Coordinates ESTIMATED | Convention implemented in `chernarus_core::coords`; flat ground only | WRP header reader |
| Weather / day-night | P0 | — | UNKNOWN | — | — |
| Audio | P0 | — | UNKNOWN | — | — |
| Multiplayer | P0 | — | Model ESTIMATED | Server authority is an intended divergence (ADR 0004) | Confirm scope |
| Persistence | P0 | — | UNKNOWN | — | — |

## Research tooling

These are not gameplay systems, but they carry parity work.

| Capability | State | Verification |
|---|---|---|
| PBO reader (header, entries, SHA-1, LZSS) | Implemented | Synthetic archives only (ESTIMATED) |
| Rapified config reader + text printer | Implemented | Own writer only (ESTIMATED) |
| Config inheritance / cross-addon merge | Not implemented | — |
| Format identification (15 formats by signature, more by extension) | Implemented | Not yet run on a reference scan (ESTIMATED) |
| Dependency graph (config, text, binary heuristic) | Implemented | Synthetic installation |
| Asset query (7 questions) | Implemented | Synthetic installation |
| Provenance enforcement + repository guard | Implemented | Tests + CI |
| Capture harness (SQF) | Written | **UNTESTED in game** |
| RPT import, metrics, comparison | Implemented | Synthetic references; analytic metric tests |
| Renderer / client | Not started | ADR 0003 proposed |
| Networking | Not started | ADR 0004 proposed |
