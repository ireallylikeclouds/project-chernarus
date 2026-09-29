//! File-format support for the reference installation (ARMA 2: Operation
//! Arrowhead / Real Virtuality 3 and DayZ Mod addons).
//!
//! Scope and rules:
//! * Readers are written from public format documentation and checked against
//!   open-source implementations; see `docs/reference/formats/`. Each format
//!   records its own verification status there. Until a reader has been run
//!   against the reference installation it is at most `ESTIMATED`.
//! * Readers must never panic on malformed input: asset discovery runs them
//!   over every file of an installation, including files of unknown formats.
//! * Writers exist only to build PROJECT-CREATED synthetic fixtures for tests
//!   and demos (see [`synth`]). They are not a modding toolchain.

pub mod error;
pub mod identify;
pub mod lzss;
pub mod pbo;
pub mod rap;
pub mod read;
pub mod strings;
pub mod synth;

pub use error::FormatError;
pub use identify::{Format, Identification, identify};
