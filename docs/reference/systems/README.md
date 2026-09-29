# Systems: reference knowledge index

The reference layer for each system is ESTIMATED (see [../sources.md](../sources.md)). "Known" means
documented in this repository with a status. Almost nothing is known yet, by design: nothing has been
checked against a reference installation.

| System | Likely reference layer | Known | Page |
|---|---|---|---|
| Movement (walk/run/sprint, stances, transitions) | Engine + `CfgMoves*` + RTM animations | Structure ESTIMATED; all values UNKNOWN | [movement.md](movement.md) |
| Camera (first person, head bob, freelook) | Engine + unit config | UNKNOWN | — |
| Weapons, ballistics, recoil, sway | Engine + `CfgWeapons`/`CfgMagazines`/`CfgAmmo`/`CfgRecoils` | UNKNOWN | — |
| Inventory and gear slots | Engine gear system + DayZ config and scripts | UNKNOWN | — |
| Zombies (senses, pursuit, attacks, spawning) | DayZ scripts (SQF/FSM) + `CfgVehicles` + engine AI | UNKNOWN | — |
| Medical and survival (blood, bleeding, infection, shock, pain, fractures, temperature, hunger, thirst) | DayZ scripts | UNKNOWN | — |
| Loot (tables, spawn rules, respawn) | DayZ config + scripts (version-dependent) | UNKNOWN | — |
| Vehicles | Engine + `CfgVehicles` + DayZ scripts (spawning, persistence) | UNKNOWN | — |
| World (terrain, objects, doors) | Terrain WRP + building models + configs | Coordinates ESTIMATED | [../conventions/coordinates.md](../conventions/coordinates.md) |
| Weather and day/night | Engine + DayZ scripts | UNKNOWN | — |
| Audio and AI hearing | Engine + `CfgSounds` + DayZ scripts | UNKNOWN | — |
| Multiplayer and persistence | Engine netcode + DayZ server scripts + database extension | Model ESTIMATED (ADR 0004) | [../../architecture/adr/0004-networking.md](../../architecture/adr/0004-networking.md) |
