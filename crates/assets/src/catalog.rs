//! Reference catalog: metadata about a scanned reference installation.
//!
//! A catalog contains names, sizes, hashes, formats and dependency edges; it
//! never contains asset content. It is still derived from the reference
//! installation, so it is written to a git-ignored location and not
//! committed. Output is sorted so two scans of the same installation are
//! byte-identical.

use crate::provenance::ProvenanceClass;
use chernarus_formats::Format;
use chernarus_formats::identify::Basis;
use chernarus_formats::lzss::ChecksumKind;
use chernarus_formats::pbo::{Checksum, Packing};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::Path;

pub const SCHEMA_VERSION: u32 = 1;

/// Where an asset physically lives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    /// A file on disk, relative to the installation root (`/`-separated).
    File { path: String },
    /// An entry inside a PBO (identified by the PBO's asset id).
    PboEntry { pbo: String, entry: String, packing: Packing },
}

/// PBO-specific metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PboDetails {
    pub prefix: Option<String>,
    /// Where the prefix came from: the header, or the file name (older archives).
    pub prefix_source: PrefixSource,
    pub extensions: Vec<(String, String)>,
    pub entry_count: usize,
    pub checksum: Option<Checksum>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrefixSource {
    Header,
    /// No `prefix` extension; the file stem is used. ESTIMATED engine rule.
    FileName,
}

/// One discovered asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetRecord {
    /// Unique id: the installation-relative path for files, or
    /// `<pbo id>::<entry name>` for PBO entries.
    pub id: String,
    /// Normalised engine path, for PBO entries.
    pub virtual_path: Option<String>,
    pub location: Location,
    /// Logical size in bytes (decompressed for compressed entries).
    pub size: u64,
    pub sha256: Option<String>,
    pub format: Format,
    pub identified_by: Basis,
    /// Content signature and extension disagree.
    pub format_conflict: bool,
    pub provenance: ProvenanceClass,
    pub redistributable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pbo: Option<PboDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lzss_checksum: Option<ChecksumKind>,
}

/// How a dependency edge was found. Confidence differs by method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeMethod {
    /// A string value with an asset extension in a parsed rapified config.
    ConfigParse,
    /// A path-like config string without extension that exists in the
    /// installation once an engine default extension is added.
    ConfigImplicitExtension,
    /// A path-like token in a text file.
    TextScan,
    /// A path-like NUL-terminated string in a binary file we cannot parse yet.
    BinaryStringScan,
}

/// A reference from one asset to a virtual path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    /// Normalised target path as resolved (or as referenced, if unresolved).
    pub target: String,
    /// Assets providing `target`; more than one means several PBOs mount the
    /// same path and the engine's choice depends on load order (UNKNOWN).
    pub providers: Vec<String>,
    pub method: EdgeMethod,
    /// Config entry path for config edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl Edge {
    pub fn resolved(&self) -> bool {
        !self.providers.is_empty()
    }
}

/// A problem met while scanning (the scan continues).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanIssue {
    pub asset: String,
    pub message: String,
}

/// Complete scan result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub schema: u32,
    pub tool: String,
    /// Installation root as given to the scanner.
    pub root: String,
    pub assets: Vec<AssetRecord>,
    pub edges: Vec<Edge>,
    pub issues: Vec<ScanIssue>,
}

impl Catalog {
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        std::fs::write(path, json)
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let catalog: Catalog = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if catalog.schema != SCHEMA_VERSION {
            return Err(io::Error::other(format!(
                "catalog schema {} is not supported (expected {SCHEMA_VERSION}); rescan",
                catalog.schema
            )));
        }
        Ok(catalog)
    }

    /// Build lookup tables.
    pub fn index(&self) -> CatalogIndex<'_> {
        let mut by_id = HashMap::new();
        let mut by_virtual_path: HashMap<&str, Vec<&AssetRecord>> = HashMap::new();
        for a in &self.assets {
            by_id.insert(a.id.as_str(), a);
            if let Some(vp) = &a.virtual_path {
                by_virtual_path.entry(vp.as_str()).or_default().push(a);
            }
        }
        let mut outgoing: HashMap<&str, Vec<&Edge>> = HashMap::new();
        let mut incoming: HashMap<&str, Vec<&Edge>> = HashMap::new();
        for e in &self.edges {
            outgoing.entry(e.from.as_str()).or_default().push(e);
            for p in &e.providers {
                incoming.entry(p.as_str()).or_default().push(e);
            }
        }
        CatalogIndex { by_id, by_virtual_path, outgoing, incoming }
    }

    /// Counts by format, for summaries.
    pub fn format_counts(&self) -> BTreeMap<Format, usize> {
        let mut counts = BTreeMap::new();
        for a in &self.assets {
            *counts.entry(a.format).or_insert(0) += 1;
        }
        counts
    }
}

/// Lookup tables over a catalog.
#[derive(Debug)]
pub struct CatalogIndex<'a> {
    pub by_id: HashMap<&'a str, &'a AssetRecord>,
    pub by_virtual_path: HashMap<&'a str, Vec<&'a AssetRecord>>,
    /// Edges leaving an asset ("depends on").
    pub outgoing: HashMap<&'a str, Vec<&'a Edge>>,
    /// Edges resolved to an asset ("depended on by").
    pub incoming: HashMap<&'a str, Vec<&'a Edge>>,
}
