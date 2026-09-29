# Reference knowledge

What the original game does, how sure we are, and how we know.

## Knowledge status

Every fact, parameter and behaviour description carries one of these statuses (`chernarus_core::Verification`):

| Status | Meaning | Minimum evidence |
|---|---|---|
| `UNKNOWN` | Nothing is known. A value may exist only as a placeholder so code can run. | — |
| `ESTIMATED` | From secondary sources (community documentation, open-source tools, memory, reasoning) and **not yet checked** against the reference. | A cited source |
| `PARTIALLY VERIFIED` | Checked against the reference installation, under limited conditions or with limited precision. | A capture id or a reference file plus version/hash, and a statement of what was *not* checked |
| `VERIFIED` | Reproducibly measured or extracted, with a documented method and a comparison test. | A parity scenario with at least 3 consistent reference runs, or extraction from pinned reference data with a test |

Rules:

- Never raise a status without recording the evidence (`source` field or a doc citation).
- A derived result is never better known than its weakest input (enforced for simulation runs by `ParamLedger`).
- Placeholders are `UNKNOWN`, even when they look plausible.

## Layout

| Path | Contents |
|---|---|
| [sources.md](sources.md) | Where each kind of behaviour lives in the reference (engine / config / mod scripts) and how to research it |
| [reference-installation.md](reference-installation.md) | Which reference versions are needed, where to put them, what is still open |
| [conventions/](conventions/) | Coordinate system, units, time |
| [formats/](formats/) | File formats: layout, verification status, sources |
| [systems/](systems/) | Behaviour per gameplay system |

## Template for a behaviour entry

```markdown
### <Behaviour name>

- Status: UNKNOWN | ESTIMATED | PARTIALLY VERIFIED | VERIFIED
- Reference layer: engine | config (<class path>) | mod script (<file>, version)
- Observation / formula (own words): …
- Evidence: capture ids / reference file + version + sha256 / document link
- Not checked: …
- Parity scenario: tests/parity/scenarios/<file>.toml
- Rust implementation: crates/…
```
