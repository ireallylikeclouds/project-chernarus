# Operating contract

Condensed from GDD §15; the GDD is authoritative.

## Loop

```text
INSPECT → UNDERSTAND → PLAN → IMPLEMENT → BUILD → TEST → COMPARE → DOCUMENT → REPORT
```

## Rules in practice

| Rule | How this repository applies it |
|---|---|
| Never invent unknown behaviour | Placeholders are `Param`s with status `UNKNOWN`; the parity harness reports `NO_REFERENCE` instead of fabricating reference values |
| Never present an assumption as fact | Every doc statement about the reference carries a status |
| Never claim completion for placeholders | Parity levels need evidence (see [../parity/README.md](../parity/README.md)) |
| Keep proprietary assets out | Repository guard + provenance manifest, in tests and CI (ADR 0005) |
| Keep the repository buildable | CI: fmt, clippy `-D warnings`, tests, provenance check |
| Keep the parity matrix updated | Any change to a system's behaviour or evidence updates [../parity/parity-matrix.md](../parity/parity-matrix.md) |
| Stop and document blockers | Open questions go in the task report and the relevant doc, marked as decisions for the project owner |

## Definition of done for a change

1. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass.
2. New behaviour values are in `data/` with status and source.
3. Docs and the parity matrix reflect the change.
4. A task report exists for milestone-sized work (`docs/reports/YYYY-MM-DD-<topic>.md`).

## Task report template

```text
TASK            What was requested.
CHANGED         Files/systems modified.
IMPLEMENTED     What now works.
VALIDATED       Builds, tests and runtime checks performed.
PARITY          Current parity level and confidence.
KNOWN DIFFERENCES  Observable deviations.
RISKS / DEBT    Important issues.
NEXT            Smallest logical next step.
```
