# ADR 0005 — Reference data boundary and provenance enforcement

Status: Accepted (2026-09-29)

## Context

The GDD forbids committing or distributing proprietary ARMA/DayZ files and requires a provenance
system (REFERENCE / LICENSED / PROJECT-CREATED / UNKNOWN) that never silently mixes categories.
Research, meanwhile, needs to read the reference installation in depth.

## Decision

1. **The reference installation stays outside the repository** (or inside the git-ignored
   `reference/`). Tools only read it.
2. **Outputs of reading it stay local**: catalogs, config dumps and extracted files go to
   git-ignored `reference-data/` or `local/`.
3. **What may be committed**: our own code and tools; our own measurements (traces captured by
   our harness); facts such as numbers, behaviours and formulas, **written in our own words with a
   citation** (file, version, hash). Verbatim script or config text is not committed.
4. **A converted reference asset is still REFERENCE.** Converting a model or texture into a
   project format does not change its class. Only LICENSED or PROJECT-CREATED sources may become
   distributable runtime assets.
5. **Enforcement**:
   - `.gitignore` excludes reference formats and local directories;
   - the repository guard test (`crates/assets/tests/repository_guard.rs`) fails on any file whose
     content or extension is a reference format, and on images or audio outside `assets/`;
   - every file in `assets/` needs a `provenance.toml` record with class PROJECT-CREATED or
     LICENSED (LICENSED requires source, licence and licence file); REFERENCE and UNKNOWN fail;
   - CI runs all of the above.

## Consequences

- A PNG diagram in `docs/` fails the guard; it must live under `assets/` with a record.
  Deliberately strict: a converted texture looks like any other PNG.
- Legitimately usable Bohemia content, if any, must enter as LICENSED with its licence text
  (see [docs/assets/provenance.md](../../assets/provenance.md#licensed-bohemia-content)).

## Revisit when

A legitimate need for a reference-format file in the repository appears (for example a
PROJECT-CREATED P3D test model). The guard would then gain an explicit allow-list in the manifest,
never a blanket exception.
