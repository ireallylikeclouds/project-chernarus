# Agent notes

Read first: `docs/design/gdd.md` (brief), `docs/process/operating-contract.md` (loop, rules, report
format), `docs/architecture/overview.md` (crates and layering rules).

## Commands

- Build and test everything: `cargo test --workspace`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`
- Asset boundary: `cargo run -p chernarus-refscan -- provenance`
- Parity: `cargo run -p chernarus-parity-cli -- run`

## Non-negotiables

- Never commit reference (Bohemia/DayZ) files or anything converted from them; the repository guard test will fail.
- Never invent reference behaviour: unknown values are `Param`s with `status = "UNKNOWN"`, and the parity harness reports `NO_REFERENCE`.
- Gameplay crates must not depend on rendering, networking or reference-format crates.
- Update `docs/parity/parity-matrix.md` whenever a system's behaviour or evidence changes.
- Everything must work on Linux and Windows (CI runs both): use `Path` APIs, accept CRLF input, match extensions case-insensitively.
- Milestone work ends with a report in `docs/reports/`.
