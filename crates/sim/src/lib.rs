//! Gameplay simulation.
//!
//! Rules for this crate (see `docs/architecture/overview.md`):
//! * no rendering, windowing, audio or networking dependencies;
//! * fixed timestep, deterministic for identical inputs on the same platform;
//! * behaviour parameters come from data files as [`chernarus_core::Param`]s,
//!   and every run records which parameters it used ([`ledger::ParamLedger`]),
//!   so results can report the weakest knowledge status they depend on.
//!
//! Milestone M0 implements infantry movement on flat ground only.

pub mod ledger;
pub mod movement;
pub mod world;

pub use movement::{Direction, MoveIntent, MovementParams, Pace, PlayerMotion, Stance};
pub use world::{PlayerId, Simulation};
