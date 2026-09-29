# Task report — M0 foundation (2026-09-29)

## TASK

GDD §16 "First Agent Task": inspect the repository; determine the toolchain, renderer/engine, ECS,
networking and available tools; create the documentation structure, the parity matrix and the
provenance database design; design the asset discovery pipeline; build the smallest working
prototype of *inspect reference → discover assets → document behaviour → represent data →
run simulation → compare results*; compile, test and report exactly what was verified.

## CHANGED

The repository was empty (no commits). Everything is new:

- Workspace: `Cargo.toml`, `rust-toolchain.toml` (1.94.1), `rustfmt.toml`, `.gitignore`, CI (`.github/workflows/ci.yml`), `README.md`, `CLAUDE.md`.
- Crates: `crates/core`, `crates/formats`, `crates/assets`, `crates/sim`, `crates/parity`.
- Tools: `tools/refscan` (CLI `refscan`), `tools/parity-cli` (CLI `parity`), `tools/capture` (SQF harness + guide).
- Data: `data/movement/infantry.toml`, `assets/provenance.toml`, `tests/parity/scenarios/*.toml` (3).
- Docs: `docs/design/gdd.md` (the brief, verbatim), `docs/architecture/` (overview, toolchain, ADRs 0001–0005),
  `docs/reference/` (statuses, sources, installation, coordinates, formats, systems, movement),
  `docs/parity/` (levels, methodology, matrix), `docs/assets/` (provenance design, discovery pipeline),
  `docs/process/operating-contract.md`, this report.

## IMPLEMENTED

1. **Inspection** (items 1, 2, 6): empty repository; Rust 1.94.1 with clippy and rustfmt; no GPU, display
   or reference installation in the container; the Bohemia wiki is blocked by egress policy, so format
   facts were cross-checked against open-source readers on GitHub (`docs/architecture/toolchain.md`).
2. **Architecture decisions** (items 3–5):
   - layering: simulation independent of presentation and of reference formats (ADR 0001);
   - ECS: none in the simulation for now; plain typed state and ordered system functions, revisit end of Phase 2 (ADR 0002);
   - renderer: **proposed** Bevy (pinned) as a presentation-only client (ADR 0003, needs owner decision);
   - networking: **proposed** server-authoritative fixed tick with prediction, with an intended divergence
     from ARMA's client authority (ADR 0004, needs owner decision).
3. **Documentation structure and parity matrix** (items 7, 8): as listed above. The matrix has every
   system at P0 except movement at P1, all with low or no confidence.
4. **Provenance database** (item 9): four classes; a committed, enforced project manifest
   (`assets/provenance.toml`); a local-only reference catalog; the link between them via
   `reference_equivalent`. Enforcement: manifest rules plus a repository guard that detects reference
   formats by content, and media outside `assets/` (`docs/assets/provenance.md`, ADR 0005).
5. **Discovery pipeline** (item 10): designed end to end; stages 1–5 implemented, the viewer is partial
   (text), conversion and runtime assets not started (`docs/assets/discovery-pipeline.md`).
6. **Prototype** (item 11), every link of the chain:
   - *Inspect / discover*: `refscan scan` walks an installation, opens PBOs (incl. LZSS entries), identifies
     formats, hashes content, verifies PBO SHA-1 trailers, parses rapified configs, and builds a resolved
     dependency graph; `refscan query` answers the seven GDD questions for any asset.
   - *Document*: reference docs with explicit statuses; nothing claimed beyond ESTIMATED.
   - *Represent*: `Param<T>` (value + status + source) in `data/`; the simulation records which
     parameters each run used.
   - *Run*: deterministic, headless, fixed-tick movement simulation (stances, paces, 5 direction classes,
     acceleration/deceleration, stance transitions).
   - *Compare*: scenarios → simulated trace → metrics (steady speed, time to 90% speed, stopping
     distance) → comparison with reference traces imported from RPT logs → verdict plus reference status plus
     simulation input status. A project-written SQF script captures reference traces in the real game.

## VALIDATED

- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: **74 passed, 0 failed**:
  - formats (30): LZSS hand-built vectors including edge cases and both checksum variants, round trips,
    corruption and truncation; PBO parse, compressed reads, checksum valid/mismatch/absent, no product entry, junk input;
    raP round trip, case-insensitive lookup, string visiting, text rendering, truncation, offset cycles;
    signature/extension identification and conflicts; path-string extraction;
  - assets (14): provenance rules (each violation), guard (renamed PBO, extension-only key, untracked PNG,
    skipped local dirs), end-to-end discovery on the synthetic installation (identification, mounting,
    cross-addon, relative and implicit-extension resolution, unresolved references, corrupt archive,
    queries, byte-identical rescans), and the guard plus manifest check **on this repository**;
  - sim (10): kinematics against analytic values, heading convention, deceleration, stance transitions,
    validation, bit-identical determinism, shipped data file complete and honest;
  - core (8), parity (12): metrics against analytic motion (regular and jittered sampling), RPT parsing with
    malformed lines, scenario validation, the full loop with a **synthetic** reference (PASS with identical
    parameters, FAIL with the correct +0.3 m/s difference, status grading by run count), and all committed
    scenarios reporting `NO_REFERENCE`.
- CLI runtime checks: `refscan synth-install / scan / summary / query / pbo --verify / config / identify / provenance`
  (passes on the repository; exits 1 on a planted renamed PBO and an untracked PNG);
  `parity run --strict`, `parity params` (53 parameters, all UNKNOWN).
- CI workflow YAML parses; **it has not run on GitHub yet**.

**Not validated:** nothing has been run against a real ARMA 2 / DayZ Mod installation (none available);
the SQF capture script has never run in game; no renderer exists.

## PARITY

| System | Level | Confidence |
|---|---|---|
| Movement | P1 | Low: kinematic stand-in, all values UNKNOWN placeholders, 0 reference captures |
| All other gameplay systems | P0 | — |

## KNOWN DIFFERENCES

- Movement is kinematic (target speed + linear ramps); the reference is believed animation-driven (ESTIMATED).
- All movement values are placeholders; left/right symmetry, diagonals, slope, weapon state and fatigue are not modelled.
- Flat ground only; no terrain, collision or camera.
- The simulation runs at a fixed 60 Hz; the reference runs at the frame rate.

## RISKS / DEBT

- **Reference version not pinned** (ARMA 2, OA and DayZ Mod). Parity is meaningless until it is; DayZ Mod
  behaviour differs substantially between releases.
- Format readers are verified only against our own writers and hand-built vectors; the first real scan may
  require corrections (signatures, LZSS checksum variant, prefix rules).
- The binary string heuristic for model and terrain dependencies can mis-resolve; real ODOL/WRP readers should replace it.
- Config inheritance and cross-addon merging are missing, so config values are not yet citable as effective values.
- Catalog JSON will be large for a full installation; SQLite is planned.
- Human-timed captures limit precision; the onset-aligned metrics mitigate this but frame-rate effects are UNKNOWN.
- Project licence not chosen (no `LICENSE` file, no licence field in manifests).
- Renderer and networking choices are proposals awaiting owner decisions.

## NEXT

Smallest logical step: **pin the reference versions, then run `refscan scan` on the real installation**
(on the owner's machine) and feed the findings back: format-table corrections, the LZSS checksum
variant, the prefix layout, and the location of movement configs and DayZ scripts. In parallel, validate the capture
harness in game (validation table in `tools/capture/README.md`) and capture three runs of
`movement.stand_run_forward`. Those are the first real numbers that can move a status above UNKNOWN.
