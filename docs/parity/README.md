# Parity

How close the Rust implementation is to the reference, and how we know.

- [parity-matrix.md](parity-matrix.md): current level per system. **Keep it updated** with every change that affects a system.
- [methodology.md](methodology.md): scenarios, metrics, verdicts and statuses.

## Parity levels (per system)

| Level | Meaning | Required evidence |
|---|---|---|
| P0 | Not implemented | — |
| P1 | Conceptually implemented | Code exists with the right inputs and outputs; values may be placeholders |
| P2 | Functionally similar | Main behaviours present; values at least ESTIMATED |
| P3 | Behaviourally close | Parity scenarios pass for the main behaviours against PARTIALLY VERIFIED references |
| P4 | High fidelity | Scenarios pass across conditions (stances, terrain, load, and so on) |
| P5 | Verified parity | All scenarios pass against VERIFIED references |

Parity levels describe systems. Knowledge statuses (UNKNOWN / ESTIMATED / PARTIALLY VERIFIED /
VERIFIED) describe individual facts and measurements (see [../reference/README.md](../reference/README.md)).
