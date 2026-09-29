# Reference sources: where behaviour lives

DayZ Mod behaviour comes from three layers of the reference installation. Each needs a different
research method. This split is itself ESTIMATED, from general knowledge of Real Virtuality 3 modding;
it will be confirmed as catalogs of the real installation become available.

| Layer | What it holds | How we research it | Tooling status |
|---|---|---|---|
| **Engine** (ARMA 2 OA executable) | Movement integration and animation playback, ballistics, collision, AI pathfinding and senses, rendering, sound propagation, networking | **Observe and measure** in the running game (capture harness, controlled scenarios). The executable is never decompiled, copied or redistributed. | Capture harness written, UNTESTED |
| **Config** (rapified `config.bin` inside PBOs) | Data definitions: `CfgMoves*` (animation states and speeds), `CfgWeapons`, `CfgMagazines`, `CfgAmmo`, `CfgVehicles` (units, zombies, buildings, vehicles), `CfgSounds`, loot definitions (version-dependent) | **Extract** with `refscan config` / catalog; record values with citations | raP reader done (ESTIMATED); inheritance and cross-addon merge not implemented |
| **Mod scripts** (SQF/FSM text inside DayZ Mod PBOs) | DayZ gameplay rules: blood, bleeding, infection, hunger and thirst, temperature, zombie spawning and behaviour scripts, loot spawning, persistence calls | **Read locally**; describe formulas in our own words with file and version citations; confirm important ones by observation | Text files discoverable and scannable; no SQF analysis tooling |

Consequences for planning:

- **Survival and medical formulas** are most likely in mod scripts, so reading the pinned version
  is the fastest way to reach ESTIMATED to PARTIALLY VERIFIED, followed by in-game confirmation.
- **Movement, ballistics and AI senses** are engine behaviour driven by config data. They need two
  independent routes that should agree: extracting the config/animation data, and measuring in game.
- **Networking behaviour** is engine plus scripts, and is observable only in multiplayer sessions
  (see ADR 0004 for intended divergences).

Secondary sources (community wikis, open-source tools, forum knowledge) can only produce ESTIMATED.
Cite them.
