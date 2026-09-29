//! Answers, for one asset, the questions the discovery tool exists for:
//! what is it, where did it come from, what does it depend on, what depends
//! on it, what format is it, can we use it, and what is the project's
//! equivalent.

use crate::catalog::{AssetRecord, Catalog, CatalogIndex, EdgeMethod, Location};
use crate::provenance::{ProvenanceClass, ProvenanceManifest};
use crate::vfs;
use serde::Serialize;
use std::fmt;

/// Result of looking up one asset.
#[derive(Debug, Clone, Serialize)]
pub struct AssetCard {
    pub id: String,
    pub virtual_path: Option<String>,
    pub source: String,
    pub format: String,
    pub size: u64,
    pub sha256: Option<String>,
    pub provenance: ProvenanceClass,
    pub usage: String,
    pub depends_on: Vec<DependencyLine>,
    pub depended_on_by: Vec<DependencyLine>,
    pub project_equivalents: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyLine {
    pub path: String,
    pub method: EdgeMethod,
    pub resolved: bool,
}

/// Find an asset by id or virtual path (case-insensitive, either slash).
/// Returns every match: a virtual path may be provided by several PBOs.
pub fn find<'a>(index: &CatalogIndex<'a>, query: &str) -> Vec<&'a AssetRecord> {
    if let Some(a) = index.by_id.get(query) {
        return vec![*a];
    }
    let wanted = vfs::normalize(query);
    if let Some(list) = index.by_virtual_path.get(wanted.as_str()) {
        return list.clone();
    }
    let mut hits: Vec<&AssetRecord> =
        index.by_id.values().filter(|a| a.id.eq_ignore_ascii_case(query)).copied().collect();
    hits.sort_by(|a, b| a.id.cmp(&b.id));
    hits
}

/// Describe one asset.
pub fn describe(
    catalog: &Catalog,
    index: &CatalogIndex<'_>,
    asset: &AssetRecord,
    manifest: Option<&ProvenanceManifest>,
) -> AssetCard {
    let source = match &asset.location {
        Location::File { path } => format!("Reference installation file {path} (root {})", catalog.root),
        Location::PboEntry { pbo, entry, packing } => {
            format!("Reference installation, {pbo} entry {entry} ({packing:?})")
        }
    };
    let format = format!(
        "{:?} (by {:?}{})",
        asset.format,
        asset.identified_by,
        if asset.format_conflict { "; signature/extension CONFLICT" } else { "" }
    );
    let depends_on = index
        .outgoing
        .get(asset.id.as_str())
        .map(|edges| {
            edges
                .iter()
                .map(|e| DependencyLine { path: e.target.clone(), method: e.method, resolved: e.resolved() })
                .collect()
        })
        .unwrap_or_default();
    let depended_on_by = index
        .incoming
        .get(asset.id.as_str())
        .map(|edges| {
            edges.iter().map(|e| DependencyLine { path: e.from.clone(), method: e.method, resolved: true }).collect()
        })
        .unwrap_or_default();
    let project_equivalents: Vec<String> = match (manifest, &asset.virtual_path) {
        (Some(m), Some(vp)) => m.equivalents_of(vp).iter().map(|r| format!("assets/{}", r.path)).collect(),
        _ => Vec::new(),
    };
    let usage = match asset.provenance {
        ProvenanceClass::Reference => {
            "REFERENCE: inspect locally for research only; do not commit, convert into the repository, or distribute"
                .to_owned()
        }
        other => format!("{other}"),
    };
    let status =
        if project_equivalents.is_empty() { "REFERENCE_ONLY".to_owned() } else { "HAS_PROJECT_EQUIVALENT".to_owned() };
    AssetCard {
        id: asset.id.clone(),
        virtual_path: asset.virtual_path.clone(),
        source,
        format,
        size: asset.size,
        sha256: asset.sha256.clone(),
        provenance: asset.provenance,
        usage,
        depends_on,
        depended_on_by,
        project_equivalents,
        status,
    }
}

impl fmt::Display for AssetCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Asset: {}", self.virtual_path.as_deref().unwrap_or(&self.id))?;
        writeln!(f, "Id: {}", self.id)?;
        writeln!(f, "Source: {}", self.source)?;
        writeln!(f, "Format: {}", self.format)?;
        writeln!(f, "Size: {} bytes", self.size)?;
        if let Some(h) = &self.sha256 {
            writeln!(f, "SHA-256: {h}")?;
        }
        writeln!(f, "Provenance: {}", self.provenance)?;
        writeln!(f, "Usage: {}", self.usage)?;
        writeln!(f, "Dependencies:")?;
        if self.depends_on.is_empty() {
            writeln!(f, "  (none found)")?;
        }
        for d in &self.depends_on {
            let state = if d.resolved { "" } else { "  [UNRESOLVED]" };
            writeln!(f, "  {}  ({:?}){state}", d.path, d.method)?;
        }
        writeln!(f, "Depended on by:")?;
        if self.depended_on_by.is_empty() {
            writeln!(f, "  (none found)")?;
        }
        for d in &self.depended_on_by {
            writeln!(f, "  {}  ({:?})", d.path, d.method)?;
        }
        writeln!(f)?;
        writeln!(f, "Status: {}", self.status)?;
        if self.project_equivalents.is_empty() {
            writeln!(f, "Project Asset: (none)")?;
        }
        for p in &self.project_equivalents {
            writeln!(f, "Project Asset: {p}")?;
        }
        Ok(())
    }
}
