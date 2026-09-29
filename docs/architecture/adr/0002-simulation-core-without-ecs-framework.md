# ADR 0002 — Simulation core without an ECS framework

Status: Accepted for Phase 0–2; revisit at the end of Phase 2 (2026-09-29)

## Context

The task asked to "determine the ECS architecture". Options:

1. A full engine ECS in the simulation (for example `bevy_ecs` via Bevy).
2. A standalone ECS crate in the simulation (`bevy_ecs` standalone, `hecs`, `flecs` bindings).
3. Plain Rust: typed collections and explicitly ordered system functions.

Requirements that matter most right now: determinism for parity tests, headless execution, easy
unit testing of each system, data-driven parameters with provenance, and freedom to choose the
client engine later (ADR 0003). Entity counts in DayZ Mod are modest (players, a few hundred
zombies, loot piles, vehicles). The reference is also far from ECS-shaped: behaviour is spread
over engine code, configs and mod scripts.

## Decision

Option 3 for now:

- `Simulation` owns typed collections (M0: `Vec<PlayerMotion>` indexed by `PlayerId`) and calls
  systems in a fixed, explicit order each tick.
- A system is a function over plain state plus its parameter tables
  (`PlayerMotion::step(&MovementParams, &MoveIntent, dt, &mut ParamLedger)`), testable alone.
- No scheduler, no dynamic component storage, no reflection.

## Consequences

- Determinism is easy to reason about and is tested (bit-identical positions for identical inputs).
- Adding cross-cutting queries (for example "all entities that can hear this sound") needs
  explicit code; that becomes the signal to revisit.
- If a presentation ECS (Bevy) is adopted, it mirrors simulation state through a bridge instead of
  owning it.

## Revisit when

- five or more entity kinds need shared components and cross-cutting queries (expected in Phase 3); or
- replication needs change detection that plain collections make awkward.

The likely next step is `bevy_ecs` standalone *inside* the simulation, keeping system functions
testable without the scheduler.
