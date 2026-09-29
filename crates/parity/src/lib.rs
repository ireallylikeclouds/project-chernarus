//! Parity harness: Observe → Specify → Implement → Measure → Compare.
//!
//! * [`scenario`] — what to do (initial state, input timeline) and what to
//!   measure (metrics with tolerances), in TOML under `tests/parity/scenarios`.
//! * [`trace`] — time-stamped position/heading samples, from the simulation
//!   or captured in the reference game; CSV on disk.
//! * [`rpt`] — import of captures written to the ARMA 2 RPT log by
//!   `tools/capture/chernarus_capture.sqf`.
//! * [`metrics`] — measurements computed identically for both sides.
//! * [`compare`] — runs a scenario and produces a report with verdicts and
//!   knowledge statuses. With no reference captures the verdict is
//!   NO_REFERENCE: the harness never fabricates reference values.

pub mod compare;
pub mod metrics;
pub mod rpt;
pub mod scenario;
pub mod trace;
