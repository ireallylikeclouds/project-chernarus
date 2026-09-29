//! Asset provenance and reference-installation discovery.
//!
//! Two sides, deliberately kept apart:
//!
//! * **Project side** ([`provenance`], [`guard`]): every file in the
//!   distributable `assets/` tree has a provenance record, and no file in the
//!   repository is a Bohemia/DayZ container or asset format. Both are enforced
//!   by tests and by `refscan provenance check`.
//! * **Reference side** ([`discovery`], [`catalog`], [`query`]): scanning a
//!   local reference installation produces a catalog of metadata (names,
//!   sizes, hashes, formats, dependency edges). Catalogs are written outside
//!   the repository and are never committed.

pub mod catalog;
pub mod discovery;
pub mod fixture;
pub mod guard;
pub mod provenance;
pub mod query;
pub mod vfs;

pub use catalog::{AssetRecord, Catalog, Edge, EdgeMethod, Location};
pub use provenance::{ProvenanceClass, ProvenanceManifest, ProvenanceRecord};
