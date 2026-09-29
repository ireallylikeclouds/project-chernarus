# Task report — Linux support (2026-09-29)

## TASK

Make sure everything works on Linux as well.

## CHANGED

`crates/assets/src/discovery.rs`, `crates/parity/src/{compare,trace,rpt}.rs`, both CLIs, tests,
`.gitignore`, `.gitattributes`, CI workflow, docs (installation, capture guide, toolchain, pipeline,
methodology), `README.md`, `CLAUDE.md`.

## IMPLEMENTED

Bugs found by exercising Linux-typical setups (each reproduced first, then fixed):

1. **Symlinked mod folders and archives were silently skipped** by discovery. A symlinked `@SynthMod`
   folder plus a symlinked PBO gave 7 assets instead of 25. The walk now follows symlinks; loops become
   scan issues.
2. **CLIs panicked when output was piped** (`parity run | head`: panic, exit 101). Output now goes through
   fallible writes, and a closed pipe exits 0.
3. **`.gitignore` was case-sensitive on Linux**: `X.PBO` was not ignored. Patterns are now case-insensitive.
   (The content-based repository guard already caught such files.)

Additions:

- `parity import-rpt --meta KEY=VALUE` records the capture platform (native Windows vs Proton/Wine) and
  FPS, so Linux and Windows captures are never mixed unnoticed. Capture-owned keys cannot be overridden.
- Linux documentation: finding the installation (native and Flatpak Steam, Wine), the RPT location inside
  a Proton prefix, and case-sensitivity and symlink behaviour.
- CI runs on `ubuntu-latest` and `windows-latest`; `.gitattributes` enforces LF.

Found along the way (not Linux-specific): **a failing metric was hidden** when another metric in the
same scenario had no reference; the overall verdict read NO_REFERENCE. FAIL now takes precedence
(regression test added; methodology updated).

## VALIDATED

- fmt, clippy `-D warnings`, `cargo test --workspace`: 80 passed, 0 failed (new: symlinked mod folder,
  symlinked archive and loop; upper-case extensions; CRLF RPT; CRLF trace CSV; metadata validation;
  failure masking). The symlink and masking tests were confirmed to **fail** on the old code.
- Manual: scan of a symlinked layout; `refscan`/`parity` piped into `head` (exit 0); CRLF RPT imported
  from a Proton-style prefix path containing spaces, with `--meta`; reserved-key override rejected.
- **GitHub CI never executed.** All three runs on this branch (M0, this change, and a follow-up that
  switched CI to GitHub-owned actions only) failed at job setup with no runner and no logs. The cause is
  account- or repository-level and needs the owner (see `docs/architecture/toolchain.md`). Linux is
  verified locally only; **Windows is unverified**.

## PARITY

Unchanged: movement P1 (low), all else P0.

## KNOWN DIFFERENCES / RISKS

- Running the reference game under Proton/Wine is untested by the project; its effect on measured
  timing is UNKNOWN (hence the platform metadata).
- macOS is not tested.

## NEXT

Unchanged: pin reference versions, scan the real installation, validate the capture harness.
