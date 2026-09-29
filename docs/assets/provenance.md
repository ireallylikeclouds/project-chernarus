# Asset provenance: model and database design

Implements GDD §3 (legal and asset boundary) and ADR 0005. Code: `crates/assets/src/provenance.rs`,
`crates/assets/src/guard.rs`, `crates/assets/src/catalog.rs`.

## Classes

| Class | Meaning | May be committed? | May be distributed? |
|---|---|---|---|
| `REFERENCE` | Part of the original game or mod installation, **including anything converted from it** | No | No |
| `LICENSED` | Third-party content with a licence that permits our use | Yes, with licence record | Per licence |
| `PROJECT-CREATED` | Made for this project (code, data, assets, measurements) | Yes | Yes |
| `UNKNOWN` | Origin not established | No | No |

Classes never change silently. Reclassifying a file means editing its record in a reviewed commit.

## Two stores, two sides of the boundary

### 1. Project provenance manifest (committed): `assets/provenance.toml`

The source of truth for everything distributable. TOML, so it is reviewable in diffs.

```toml
schema = 1

[[asset]]
path = "zombies/zombie_01/body.glb"        # relative to assets/, "/"-separated (required)
class = "PROJECT-CREATED"                  # PROJECT-CREATED | LICENSED (required)
author = "…"                               # person or tool that produced it
description = "…"
sha256 = "…"                               # optional; verified when present
reference_equivalent = ["dz\\…\\zombie.p3d"] # reference virtual paths this stands in for (no data copied)

# LICENSED additionally requires:
source = "https://…"                       # where it was obtained
licence = "CC-BY-4.0"                      # SPDX id where one exists
licence_file = "licences/CC-BY-4.0.txt"    # licence text in the repository
```

Rules, enforced by `check_assets_dir` (in tests, `refscan provenance` and CI):

1. Every file under `assets/` (except `provenance.toml` and `README.md`) has exactly one record.
2. Every record points at an existing file.
3. `REFERENCE` and `UNKNOWN` records fail: those files cannot be in the distributable tree.
4. `LICENSED` records need `source`, `licence` and `licence_file`.
5. A recorded `sha256` must match the file.
6. Unknown fields are rejected, so typos cannot silently drop a rule.

Repository-wide guard (`guard::scan_repository`), also enforced:

7. No file anywhere in the working tree may be a reference container or asset format
   (identified by **content signature** as well as extension): PBO, PAA/PAC, P3D, WRP, RTM, WSS,
   rapified config, BIKEY/BISIGN, Windows executables and DLLs.
8. Images and audio (PNG, JPEG, OGG, WAV) are allowed only under `assets/`, where rule 1 applies.
   A texture converted to PNG cannot be distinguished from original work by content.

Skipped: `.git/`, `target/`, and the git-ignored local directories `reference/`, `reference-data/` and `local/`.

### 2. Reference catalog (local, never committed): `reference-data/catalog.json`

Written by `refscan scan` (schema: `crates/assets/src/catalog.rs`, `SCHEMA_VERSION = 1`). One
record per file and per PBO entry:

| Field | Meaning |
|---|---|
| `id` | Installation-relative path, or `<pbo path>::<entry name>` |
| `virtual_path` | Normalised engine path (`<prefix>\<entry>`) for PBO entries |
| `location` | File, or PBO entry with its packing method |
| `size`, `sha256` | Logical size and content hash (decompressed for compressed entries) |
| `format`, `identified_by`, `format_conflict` | Identification result and whether signature and extension disagree |
| `provenance` | Always `REFERENCE` |
| `redistributable` | Always `false` |
| `pbo` | Prefix (and whether it came from the header or the file name), header extensions, entry count, SHA-1 result |
| `lzss_checksum` | Which LZSS checksum variant matched (compressed entries) |

Plus `edges` (dependency references, with method and resolution; see
[discovery-pipeline.md](discovery-pipeline.md)) and `issues` (problems met; the scan continues past them).

The catalog holds metadata, not content. It is still derived from the reference and describes the
user's installation, so it stays local. Output is sorted and has no timestamps, so two scans of the
same installation are byte-identical and diffs show real changes.

### The link between them

`reference_equivalent` in the manifest relates a project asset to reference virtual paths. It
answers "what is the project's equivalent?" in `refscan query`
(`Status: REFERENCE_ONLY` when nothing stands in for the reference asset yet). Recording an
equivalence copies nothing.

## Converted and derived assets

- Converting a REFERENCE asset (to glTF, PNG, …) gives a **REFERENCE** asset. Such conversions are
  for local viewing and comparison only (`local/`).
- A PROJECT-CREATED asset made *to match* a reference asset (same dimensions, similar look, new
  work) is PROJECT-CREATED; record the reference it matches in `reference_equivalent` and describe
  how it was made in `description`.

## LICENSED Bohemia content

Lead, **not verified**: Bohemia Interactive is understood to have released some ARMA 2-era
content under its APL-family licences (Arma Public Licence / APL Share-Alike). Those licences are
generally understood to restrict use to Bohemia's own games, which would exclude this project.
Before any such content is used: obtain the exact licence text and release notes, and have them reviewed.
Until then, treat all Bohemia content as REFERENCE.

## Planned extensions

- `licences/` directory with full licence texts once the first LICENSED asset arrives.
- A `derived_from` field recording conversion lineage from project-owned or licensed sources, with a
  check that no lineage passes through REFERENCE.
- Catalog storage in SQLite once real catalogs (hundreds of thousands of entries) make JSON slow
  to query; the schema above maps directly to tables `asset`, `edge` and `issue`.
