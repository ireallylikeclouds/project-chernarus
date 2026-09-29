# DayZ Mod — Rust Reimplementation

> Concise GDD & build prompt, as given at project start (2026-09-29). This is the
> project's governing brief. Changes to it are decisions and should be recorded
> as such (see `docs/architecture/adr/`).

## Concise GDD & Build Prompt

## 1. Project Goal

Create a **high-fidelity Rust reimplementation of the original ARMA 2: Operation Arrowhead + DayZ Mod experience**.

This is **not** a DayZ-inspired survival game and not a modern redesign.

The goal is to reproduce the original game's observable behaviour as closely as practical:

* Player movement and controls
* First-person camera
* Weapons, aiming, recoil and damage
* Inventory and item interaction
* Zombies and AI behaviour
* Medical/survival systems
* Vehicles
* Loot
* World simulation
* Weather and day/night
* Audio
* Animations
* Multiplayer
* Persistence
* Map/world behaviour

Internal implementation may be completely different. **Observable behaviour is what matters.**

Guiding principle:

> **Observe → Specify → Implement → Measure → Compare → Correct**

---

# 2. Reference Philosophy

The original game is the **behavioural reference**.

Do not guess how something works when it can be observed, measured, documented or otherwise legitimately researched.

Every major system should have:

* Reference behaviour
* Rust implementation
* Parity status
* Known differences
* Verification method

Use:

```text
UNKNOWN
PARTIALLY VERIFIED
ESTIMATED
VERIFIED
```

Never present an assumption as fact.

---

# 3. Legal / Asset Boundary

The project consists of **new Rust code, tools and project-owned implementation**.

The original installation may be treated as an **external reference/source-data installation where permitted**.

Do not commit or distribute proprietary ARMA/DayZ:

* Executables
* DLLs
* Source code
* PBOs
* Models
* Textures
* Sounds
* Animations
* Maps
* Other copyrighted game files

Where licensed Bohemia data or other legitimately usable content exists, track its **exact provenance and licence** before incorporating it.

The project must maintain an asset provenance system:

```text
REFERENCE / LICENSED / PROJECT-CREATED / UNKNOWN
```

Do not silently mix these categories.

---

# 4. Asset Discovery & Reconstruction

Asset handling is a first-class subsystem.

Build a tool capable of examining the user's legitimate reference installation and discovering relevant data.

Pipeline:

```text
Reference Installation
        ↓
Asset Discovery
        ↓
Format Identification
        ↓
Metadata Extraction
        ↓
Dependency Graph
        ↓
Reference Viewer
        ↓
Project Conversion
        ↓
Rust Runtime Asset
```

Initially support investigation of relevant ARMA/DayZ asset types such as:

* Models
* Textures
* Materials
* Skeletons
* Animations
* Sounds
* Terrain
* Buildings
* Vegetation
* Objects
* Configuration/data definitions
* World/object placement data

The tool should answer:

```text
What is this asset?
Where did it come from?
What does it depend on?
What depends on it?
What format is it?
Can we legally/usefully use it?
What is the project's equivalent?
```

Example:

```text
Asset: zombie_model
Source: Reference Installation
Format: P3D
Dependencies:
  zombie textures
  materials
  skeleton
  animations

Status: REFERENCE_ONLY
Project Asset: assets/zombies/zombie_01
```

Do not assume an asset can be redistributed merely because it can be extracted.

---

# 5. Technical Architecture

Use a modular Rust architecture.

Suggested high-level structure:

```text
engine/
simulation/
world/
assets/
rendering/
physics/
audio/
animation/
networking/
input/
tools/
server/
client/
tests/
docs/
```

Simulation must not depend directly on rendering.

Core simulation systems should be independently testable:

```text
Player
Weapons
Inventory
Medical
Survival
Zombies
Loot
Vehicles
World
Weather
Time
Networking
Persistence
```

Use data-driven definitions wherever possible.

Avoid:

* Giant files
* Global mutable state
* Hardcoded item logic
* Gameplay logic inside rendering code
* Systems that cannot be tested independently

---

# 6. Reference & Parity System

Create:

```text
docs/reference/
docs/parity/
docs/assets/
docs/architecture/
```

Maintain a parity matrix:

| System    | Status | Confidence | Difference                     |
| --------- | ------ | ---------- | ------------------------------ |
| Movement  | P3     | High       | Minor acceleration difference  |
| Inventory | P2     | Medium     | Container behaviour incomplete |
| Zombies   | P1     | Low        | Detection unknown              |

Parity levels:

```text
P0 — Not implemented
P1 — Conceptually implemented
P2 — Functionally similar
P3 — Behaviourally close
P4 — High fidelity
P5 — Verified parity
```

For important systems, build reproducible comparison tests.

Example:

```text
Scenario:
Player standing 10m from zombie

Input:
Move forward for 2 seconds

Reference:
Observed position = X

Rust:
Observed position = Y

Difference:
0.12m

Status:
PARTIALLY VERIFIED
```

---

# 7. Core Gameplay

## Player

Reproduce:

* Walking
* Running
* Sprinting
* Crouching
* Prone
* Stance transitions
* Acceleration/deceleration
* Stamina
* Falling
* Swimming
* Injured states
* Unconsciousness
* Death
* Camera behaviour
* Weapon positioning
* Interaction

Measure the original rather than inventing modern movement.

## Inventory

Reproduce:

* Containers
* Equipment slots
* Clothing
* Weapons
* Magazines
* Ammunition
* Item stacking
* Moving items
* Dropping
* Picking up
* Weight/capacity
* Restrictions
* Item condition

Do not redesign the inventory unless required by implementation constraints.

## Weapons

Reproduce:

* Fire modes
* Fire rate
* Magazine capacity
* Chambering
* Reload behaviour
* Recoil
* Spread
* Projectile behaviour
* Damage
* Hit locations
* Weapon condition
* Aiming
* Sway
* Attachments

## Zombies

Reproduce observable:

* Detection
* Sight
* Hearing
* Pursuit
* Pathfinding
* Attacks
* Attack timing
* Damage
* Staggering
* Reaction to weapons
* Reaction to sound
* Doors/obstacles
* Spawning
* Despawning
* Idle/group behaviour

Do not replace the original behaviour with a generic modern zombie AI.

---

# 8. Survival & Medical Systems

Reconstruct the original behaviour of:

* Health
* Blood
* Bleeding
* Pain
* Shock
* Temperature
* Hunger
* Thirst
* Infection
* Fractures/injuries
* Unconsciousness
* Death

Formulas should be documented when known.

Unknown formulas remain marked `UNKNOWN` until investigated.

---

# 9. World

The world pipeline must eventually support:

* Terrain
* Roads
* Buildings
* Trees
* Vegetation
* Fences
* Walls
* Doors
* Windows
* Bridges
* Water
* Props
* Collision
* Navigation
* Loot locations
* Zombie spawn locations
* Object placement

Preserve reference coordinate semantics:

```text
Origin
Axes
Scale
Rotation
Terrain height
Object placement
```

Build tooling that converts discovered/reference world information into a project-owned runtime representation where legally appropriate.

---

# 10. Loot

Reconstruct the original loot model rather than inventing a new economy.

Investigate:

```text
Location
→ Building Type
→ Loot Category
→ Spawn Rules
→ Probability
→ Item Selection
→ Quantity
→ Respawn/Persistence
```

Keep loot definitions data-driven.

---

# 11. Vehicles

Implement vehicles incrementally.

First establish one complete vehicle.

Reproduce:

* Acceleration
* Braking
* Steering
* Fuel
* Seats
* Inventory
* Damage
* Collision
* Doors
* Engine
* Persistence
* Multiplayer behaviour

Then expand the vehicle system.

---

# 12. Environment

Reproduce:

### Time

* Day/night cycle
* Sun
* Moon
* Lighting
* Ambient conditions

### Weather

* Weather states
* Transitions
* Visibility
* Lighting
* Audio
* Temperature effects

### Audio

* Weapons
* Zombies
* Footsteps
* Doors
* Items
* Environment
* Weather
* Injuries
* Distant sounds

Sound events should also interact with AI hearing where applicable.

---

# 13. Multiplayer

Use modern Rust networking internally while reproducing observable multiplayer behaviour.

Architecture:

```text
Client Input
     ↓
Server Authority
     ↓
Simulation
     ↓
Replication
     ↓
Client Presentation
```

Support:

* Player movement
* Combat
* Damage
* Inventory
* Item spawning/pickup/drop
* Zombies
* Vehicles
* World state
* Persistence
* Connect/disconnect
* Reconnect

Test:

* Latency
* Packet loss
* Reordering
* Disconnects
* Reconnects
* Multiple players
* Server authority

---

# 14. Development Phases

### Phase 0 — Foundation

* Repository
* Rust workspace
* Architecture
* Build system
* Basic renderer
* Basic input
* Testing framework
* Documentation

### Phase 1 — Reference Research

* Reference installation analysis
* Behaviour documentation
* Asset discovery tooling
* Format identification
* Asset dependency database

### Phase 2 — Prototype

Implement only:

```text
Small test environment
+
First-person player
+
Movement
+
One weapon
+
One zombie
+
One inventory
+
One loot container
+
Basic health
```

Everything must be measurable.

### Phase 3 — Vertical Slice

Add:

* Multiple weapons
* Proper inventory
* Zombies
* Medical system
* Survival
* Buildings
* Terrain
* Loot
* Weather
* Day/night
* Audio
* Basic multiplayer

### Phase 4 — World

Implement the complete target world pipeline:

```text
Terrain
Buildings
Roads
Vegetation
Objects
Collision
Navigation
Loot
Spawns
Streaming
```

### Phase 5 — Multiplayer & Persistence

Implement:

* Dedicated server
* Replication
* Authority
* Persistent characters
* Persistent world state
* Vehicles
* Loot persistence

### Phase 6 — Parity

Systematically compare the reimplementation against the reference.

Prioritize:

```text
Movement
Combat
Inventory
Zombies
Medical
Loot
World
Vehicles
Networking
Audio
Environment
```

---

# 15. Build-Agent Operating Contract

The development agent operates as a:

**Senior Rust engineer + reverse-engineering analyst + parity-focused implementation agent.**

Required loop:

```text
INSPECT
→ UNDERSTAND
→ PLAN
→ IMPLEMENT
→ BUILD
→ TEST
→ COMPARE
→ DOCUMENT
→ REPORT
```

Rules:

1. Inspect before editing.
2. Understand existing architecture before changing it.
3. Implement the smallest complete milestone.
4. Do not rewrite working systems without justification.
5. Reference behaviour takes priority over assumptions.
6. Never invent unknown behaviour.
7. Prefer measurements and tests over visual guesses.
8. Keep systems modular.
9. Keep content data-driven.
10. Keep proprietary reference assets outside the distributable project.
11. Build and test continuously.
12. Never claim completion for placeholder functionality.
13. Fix root causes rather than adding symptom patches.
14. Profile before optimizing.
15. Document important discoveries.
16. Keep the parity matrix updated.
17. Stop and document blockers rather than fabricating solutions.
18. Keep the repository buildable.
19. Prefer small, reversible changes.
20. Write code that another developer can understand months later.

### Required Task Report

```text
TASK
What was requested.

CHANGED
Files/systems modified.

IMPLEMENTED
What now works.

VALIDATED
Builds, tests and runtime checks performed.

PARITY
Current parity level and confidence.

KNOWN DIFFERENCES
Observable deviations.

RISKS / DEBT
Important issues.

NEXT
Smallest logical next step.
```

---

# 16. First Agent Task

Before implementing gameplay:

1. Inspect the repository.
2. Determine the Rust toolchain.
3. Determine the renderer/engine architecture.
4. Determine ECS architecture.
5. Determine networking architecture.
6. Determine available development tools.
7. Create the documentation structure.
8. Create the parity matrix.
9. Create the asset/provenance database design.
10. Design the asset discovery pipeline.
11. Build the smallest working technical prototype.
12. Compile and test everything.
13. Report exactly what was implemented and verified.

**Do not attempt to build the entire game immediately.**

The first milestone is proving that the project can:

```text
Inspect Reference
→ Discover Assets
→ Document Behaviour
→ Represent Data
→ Run Rust Simulation
→ Compare Results
```

The project succeeds by steadily increasing **verified parity**, not by producing the largest amount of code.

## Core Principle

> **Do not make a game that feels like DayZ. Reconstruct the game that DayZ Mod actually was.**
