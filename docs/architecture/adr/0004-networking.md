# ADR 0004 — Networking model

Status: **Proposed** — a choice is needed before Phase 3 (basic multiplayer). No networking code exists in M0.

## Context

The GDD asks for modern Rust networking with server authority
(client input → server simulation → replication → client presentation) while reproducing
observable multiplayer behaviour.

The reference is believed to work differently (ESTIMATED; to be verified by observation and by
reading DayZ Mod server scripts on the reference installation):

- ARMA 2 multiplayer is largely **client-authoritative for units a client controls**; each machine
  simulates its local units and broadcasts their state.
- DayZ Mod zombies are spawned by scripts and simulated where they are local, and locality moves
  between machines.
- DayZ Mod persistence is script-driven through a server-side database extension (the "hive").

Several well-known DayZ Mod behaviours (rubber-banding, zombie desync, some damage anomalies)
follow from that model.

## Proposal

- **Server-authoritative, fixed-tick simulation** running `chernarus-sim`; clients send input
  intents (`MoveIntent` and successors) with sequence numbers.
- Client-side prediction and reconciliation for the local player; snapshot interpolation for
  everything else.
- Transport: UDP with a reliability/ordering layer. Candidates are `renet` and QUIC via `quinn`;
  choose after a latency, packet-loss and reordering test harness exists, since the GDD requires those tests.
- Persistence: server-side, behind a storage interface; the schema follows the observable
  persistence rules documented from the reference.

## Consequence: an intentional divergence to confirm

Server authority means **not** reproducing behaviour that exists only because of client authority
(exploitable trust, locality desync artefacts). Parity for networking will target observable
gameplay outcomes under normal conditions, such as hit registration timing and zombie pursuit under
latency. Divergences will be listed in the parity matrix.

## Needs from the project owner

Confirm that client-authority artefacts are out of scope for parity.
