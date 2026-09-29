# ADR 0003 — Renderer and client engine

Status: **Proposed** — a choice is needed before the Phase 0 renderer milestone.
No renderer exists in M0.

## Context

The client needs a first-person camera, large terrain (Chernarus is about 15 km × 15 km, ESTIMATED),
long view distances, dense vegetation, many buildings, animation playback, positional audio and
input. The simulation is already engine-independent (ADR 0001/0002), so the client choice mainly
affects presentation code. The cloud development container has no GPU, so rendering cannot be
verified there.

## Options

| | Bevy (pinned) as client | Custom `wgpu` + `winit` (+ `kira`/`rodio` audio) | Other engines (Fyrox, …) |
|---|---|---|---|
| Time to first window, input, audio | Fast | Slow | Medium |
| Control over terrain/vegetation/LOD pipeline | Custom render nodes possible, but within Bevy's render architecture | Full | Engine-specific |
| API stability | Breaking releases every few months; must pin and upgrade deliberately | Our own | Varies |
| Fit with ADR 0002 | Bevy ECS as a presentation mirror of simulation state | Neutral | Neutral |

## Proposal

Start the client on **Bevy, pinned to one release**, strictly as a presentation layer:

- `crates/client` depends on `chernarus-sim`; simulation state is read, never owned, by Bevy.
- Rendering-specific code stays behind a small boundary so a later move to custom `wgpu`
  rendering (likely needed for terrain and vegetation at ARMA scale) only touches the client.
- Headless render checks in CI use a software adapter (for example lavapipe or llvmpipe) for screenshot
  and frame-time smoke tests; visual parity checks run on a real GPU.

## Needs from the project owner

Confirm Bevy vs custom `wgpu`, and the minimum target hardware (sets the terrain and vegetation budget).

## Revisit when

Terrain or vegetation performance targets cannot be met inside Bevy's renderer.
