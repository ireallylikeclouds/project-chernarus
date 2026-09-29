# Asset discovery pipeline

Design and current state. Code: `crates/assets/src/discovery.rs`, `crates/formats`, CLI `refscan`.

```text
Reference Installation ──► Asset Discovery ──► Format Identification ──► Metadata Extraction
        ──► Dependency Graph ──► Reference Viewer ──► Project Conversion ──► Rust Runtime Asset
```

| Stage | What it does | State |
|---|---|---|
| Reference Installation | Path argument or `CHERNARUS_REFERENCE_DIR`; read-only | **Done** |
| Asset Discovery | Walks every file (sorted; symlinks followed with loop detection, since Linux setups often symlink mod folders); opens PBOs and treats each entry as an asset mounted at `<prefix>\<entry>` | **Done** |
| Format Identification | Content signature first, then extension; both recorded; disagreement flagged (`format_conflict`) | **Done**; signatures ESTIMATED |
| Metadata Extraction | Size, SHA-256; PBO header extensions, prefix, SHA-1 trailer check; LZSS checksum variant; full parse of rapified configs | **Partial**: PBO + raP; other formats are signature-only |
| Dependency Graph | References extracted and resolved against the virtual file system (below) | **Done** for configs, text and a binary heuristic |
| Reference Viewer | `refscan query` (7 questions), `refscan pbo`, `refscan config`, `refscan summary`: text only | **Partial**: no visual viewer (needs renderer) |
| Project Conversion | Converters from **LICENSED / PROJECT-CREATED** sources to runtime formats. Reference conversions are local-only previews (ADR 0005) | **Not started** |
| Rust Runtime Asset | Runtime formats consumed by client and simulation | **Not started** |

## Dependency extraction

| Method | Source | Confidence |
|---|---|---|
| `config_parse` | String values ending in an asset extension, in parsed rapified configs (context = config path) | High (real parse) |
| `config_implicit_extension` | Extension-less path strings in configs (for example sound paths) that exist once an engine default extension is added (`p3d wss ogg wav paa sqf`); kept only if the result exists | Medium (extension rule ESTIMATED) |
| `text_scan` | Path-like tokens with asset extensions in text files (configs, scripts, rvmat, missions) | Medium |
| `binary_string_scan` | Path-like NUL-terminated strings in formats not parsed yet (ODOL/MLOD models, terrains). If the full run does not resolve, up to 32 leading bytes are trimmed, since printable bytes before a string can glue onto it | Low (heuristic, replaced by real readers later) |

Resolution: normalise (lower case, backslashes, no leading separator, `.`/`..` collapsed), try the
absolute path, then the path relative to the referencing asset's directory. **Every** provider of a
path is recorded: when several PBOs mount the same path, the engine's choice depends on load order (UNKNOWN).
Unresolved references stay in the graph with no providers: missing content, or content in addons not scanned.

## Robustness

Readers never panic on malformed input: size limits on strings, entry counts and nesting; cycle
detection for config class offsets; a bounds check of PBO data against the file size. A file that
cannot be read becomes a scan issue and the scan continues.

## Tested on

Only the PROJECT-CREATED synthetic installation (`refscan synth-install`,
`crates/assets/src/fixture.rs`): two addons with cross-addon references, relative model paths, an
implicit-extension sound, a compressed rvmat, a missing texture, a mislabelled file, an old-style
PBO without a product entry, and a corrupt PBO.

## Next

1. Run on the pinned reference installation; record the format table corrections and the LZSS checksum variant.
2. RTM and ODOL header readers to replace the heuristic for models and animations.
3. Config merge across addons for effective values.
4. SQLite catalog storage once catalog size demands it.
