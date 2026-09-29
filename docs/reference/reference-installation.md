# Reference installation

## Required (to be pinned)

| Component | Version | Status |
|---|---|---|
| ARMA 2 (base game data, needed by OA for Chernarus) | UNKNOWN — to be pinned | **Open question** |
| ARMA 2: Operation Arrowhead (engine) | UNKNOWN — to be pinned (final 1.6x build series expected) | **Open question** |
| DayZ Mod | UNKNOWN — to be pinned | **Open question** |

DayZ Mod changed substantially across its releases: the original mod era and the later
community-maintained releases differ in loot, zombies, medical rules and more. **Parity is only
meaningful against one pinned version.** Choosing it is a project-owner decision. The rest of
the documentation is written so the version can be recorded once and cited everywhere.

When pinned, record here: product versions (from `productVersion` in the capture harness or from
file metadata), the file list with SHA-256 (from `refscan scan`), and the launch parameters used.

## Location

Keep the installation outside the repository, or inside the git-ignored `reference/` directory.
Point tools at it with an argument or `CHERNARUS_REFERENCE_DIR`:

```sh
export CHERNARUS_REFERENCE_DIR="/path/to/ARMA 2 Operation Arrowhead"
cargo run -p chernarus-refscan -- scan            # writes reference-data/catalog.json (git-ignored)
cargo run -p chernarus-refscan -- summary reference-data/catalog.json
```

## Expected layout (ESTIMATED)

Mod folders such as `@DayZ/Addons/*.pbo`, base and expansion addon folders containing `*.pbo`,
the executable and DLLs at the root, and signing keys (`*.bikey`) in `Keys/` folders. The scanner
makes no assumption about the layout: it walks everything and records what it finds.

## Current state

No reference installation has been scanned. Discovery has only been exercised on the
PROJECT-CREATED synthetic installation (`refscan synth-install`).
