# Toolchain and development environment

Recorded during the M0 inspection (2026-09-29).

## Rust

| Item | Value |
|---|---|
| Toolchain | `1.94.1` stable, pinned in [`rust-toolchain.toml`](../../rust-toolchain.toml) with `rustfmt` and `clippy` |
| Edition | 2024 (workspace-wide), resolver 3 |
| Lints | `unsafe_code = "forbid"`; clippy `-D warnings` in CI; `unwrap_used`, `todo`, `dbg_macro` warn |
| Formatting | `rustfmt.toml`: `max_width = 120` |
| Dev profile | dependencies built at `opt-level = 2` so tests on large inputs stay fast |

## Supported platforms

| Platform | Status |
|---|---|
| Linux x86_64 | Developed and tested locally (all checks pass); CI job configured (`ubuntu-latest`) |
| Windows x86_64 | CI job configured (`windows-latest`); **never executed**, see below |
| macOS | Not tested |

Portability rules: paths are built with `Path`/`PathBuf` (never string concatenation with `\` or `/`);
identifiers stored in catalogs and traces use `/` for host paths and `\` for engine virtual paths,
independent of the host OS. Text files are LF (`.gitattributes`) and readers accept CRLF. File
extensions are matched case-insensitively. Directory walks follow symlinks, with loop detection. The
CLIs exit cleanly when their output pipe closes (`refscan … | head`). The only platform-specific
test is the Unix symlink test.

## Commands

```sh
cargo build --workspace
cargo test --workspace                                  # unit + integration + repository guard
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo run -p chernarus-refscan -- provenance            # asset boundary check (also a test)
cargo run -p chernarus-refscan -- synth-install /tmp/synth && \
  cargo run -p chernarus-refscan -- scan /tmp/synth --out reference-data/synth.json
cargo run -p chernarus-parity-cli -- run                # all parity scenarios
cargo run -p chernarus-parity-cli -- params             # knowledge status of movement data
```

CI runs the same checks: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

**CI status (2026-09-29): no job has executed yet.** Every run so far failed within seconds with no
runner assigned and no logs, including runs that use only GitHub-owned actions. That points at an
account- or repository-level Actions setting (Settings → Actions → General, or account billing), not
at the workflow. The reason is shown as an annotation on the run page.

## Dependencies (M0)

Deliberately small: `serde`, `serde_json`, `toml`, `thiserror`, `anyhow` (CLIs only), `clap`
(CLIs only), `sha1` (PBO checksums), `sha2` (content hashes), `walkdir`, `tempfile` (tests).
No engine, renderer, ECS or networking crate yet (see ADRs 0002–0004).

## Environment findings

| Item | Finding | Consequence |
|---|---|---|
| Repository | Empty at start (no commits) | Everything in M0 is new |
| GPU / display | None in the cloud development container | Renderer work cannot be verified here; needs a local machine or a software adapter in CI (ADR 0003) |
| Reference installation | Not present in the container | Discovery was validated only on the synthetic installation; real scans must run on the user's machine |
| Network | crates.io reachable; the Bohemia community wiki is blocked by the environment's egress policy; raw GitHub content is reachable | Format facts were cross-checked against open-source implementations on GitHub (see `docs/reference/formats/`) |
| Other tools | git, python3, jq, clang/gcc, cmake, node available; no `cargo-nextest`, `cargo-deny`, `cargo-audit`, `sqlite3` | Plain `cargo test`; licence/advisory auditing of dependencies is a follow-up |
| Machine | 4 cores, 15 GiB RAM | Adequate for the workspace |
