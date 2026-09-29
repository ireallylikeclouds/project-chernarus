//! Shared vocabulary for every Chernarus crate.
//!
//! This crate deliberately has no gameplay logic. It defines how the project
//! talks about *knowledge* (how sure we are that a value matches the reference
//! game), *space* (the reference coordinate conventions) and *time* (the fixed
//! simulation step).

pub mod coords;
pub mod param;
pub mod time;
pub mod verification;

pub use param::Param;
pub use verification::Verification;
