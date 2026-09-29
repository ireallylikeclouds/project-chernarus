# File formats

Reader status for each format and the signatures used to identify it
(`crates/formats/src/identify.rs`). All signatures are ESTIMATED: they come from public
documentation and have not yet been confirmed on a scan of the reference installation. The scanner
flags every file whose signature and extension disagree, and those files are the leads for
correcting this table.

| Format | Extensions | Signature | Reader | Verification |
|---|---|---|---|---|
| PBO archive | `.pbo` | `\0sreV` (product entry) when present; otherwise extension only | Header, entries, SHA-1 trailer, uncompressed and LZSS entries ([pbo.md](pbo.md)) | ESTIMATED — synthetic archives only |
| LZSS (in PBOs) | — | — | Full + prefix decode, both checksum variants ([lzss.md](lzss.md)) | ESTIMATED — hand-built vectors and own encoder |
| Rapified config | `.bin`, binarized `.rvmat`/`.cpp`/`.bisurf` | `\0raP` | Full tree, enums, config-text printer ([rap.md](rap.md)) | ESTIMATED — own writer only |
| Texture | `.paa`, `.pac` | u16 type, then `GGAT` at offset 2 | Signature only | ESTIMATED |
| Model, editable | `.p3d` | `MLOD` | Signature only; path-string heuristic for dependencies | ESTIMATED |
| Model, binarized | `.p3d` | `ODOL` | Signature only; path-string heuristic for dependencies | ESTIMATED |
| Terrain, binarized | `.wrp` | `OPRW` | Signature only; path-string heuristic | ESTIMATED |
| Terrain, editable | `.wrp` | `4WVR` / `8WVR` | Signature only | ESTIMATED |
| Animation, editable | `.rtm` | `RTM_` | Signature only | ESTIMATED |
| Animation, binarized | `.rtm` | `BMTR` | Signature only | ESTIMATED |
| Sound | `.wss` | `WSS0` | Signature only | ESTIMATED |
| Ogg / WAV / JPEG / PNG | usual | standard magic numbers | Signature only | standard |
| Windows executable / DLL | `.exe`, `.dll` | `MZ` | Recorded only; never parsed, never copied | — |
| Signing key / signature | `.bikey`, `.bisign` | none known | Extension only | ESTIMATED |
| Text (config, script, mission, stringtable) | `.cpp .hpp .h .rvmat .bisurf .ext .cfg .sqf .sqs .fsm .sqm .csv .xml .txt …` | text heuristic | Path-reference scan | — |

## Sources

- Bohemia Interactive community wiki format pages (not reachable from the development container's
  network; content known from secondary knowledge).
- Open-source implementations, read for format facts only (no code copied):
  - `armake2` (KoffeinFlummi, GPL-2.0): PBO read/write, rapified config read/write.
  - `bis-file-formats` (Braini01, MIT): PBO entry handling, LZSS decoder.

## Next readers (priority order)

1. **RTM header + step vector** — gives animation-driven movement speeds from data (movement parity).
2. **Config merging** across addons (`CfgPatches` load order, class inheritance), so effective
   `CfgMoves*`, `CfgWeapons` and `CfgAmmo` values can be queried.
3. **ODOL header** (version, LOD list, named selections, texture/material tables), replacing the
   string heuristic for model dependencies.
4. **PAA header** (dimensions, mip count) for texture metadata.
5. **WRP header** (grid size, cell size, object table), for world placement.
