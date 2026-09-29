# ADR 0001 — Cargo workspace and layering

Status: Accepted (2026-09-29)

## Context

The GDD asks for a modular architecture in which simulation does not depend on rendering, systems
are independently testable, and content is data-driven. The repository was empty.

## Decision

- One Cargo workspace; libraries in `crates/`, binaries in `tools/`, shared dependency versions
  where they recur.
- Dependency direction: `core` ← `sim` ← `parity`; `formats` ← `assets`; tools on top. Gameplay
  crates never depend on `formats`/`assets` (reference formats) or on presentation/network crates.
- Crates are created when their first real code exists, not as empty placeholders.
- Behaviour values live in `data/` as `Param<T>` (value, status, source).

## Consequences

- The simulation can run in tests and in the parity harness with no window, GPU or network.
- Moving to a different client engine later only touches the (future) client crate.
- The GDD's suggested top-level folders map onto crates as described in
  [overview.md](../overview.md); the mapping is a naming choice, not a behaviour change.

## Revisit when

A crate grows past one clear responsibility (for example `chernarus-sim` once weapons, inventory
and zombies land): split per system.
