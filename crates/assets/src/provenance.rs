//! Provenance of project assets.
//!
//! Every file in the distributable `assets/` tree must be listed in
//! `assets/provenance.toml`. The design is in `docs/assets/provenance.md`.

use crate::vfs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

/// Manifest file name inside the assets directory.
pub const MANIFEST_FILE: &str = "provenance.toml";
/// Files in the assets directory that are documentation, not assets.
const NON_ASSET_FILES: &[&str] = &[MANIFEST_FILE, "README.md"];

/// Where an asset came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceClass {
    /// Part of the original game or mod installation. Inspected locally for
    /// research; never committed or distributed.
    #[serde(rename = "REFERENCE")]
    Reference,
    /// Third-party content with a known licence permitting our use.
    #[serde(rename = "LICENSED")]
    Licensed,
    /// Created for this project.
    #[serde(rename = "PROJECT-CREATED", alias = "PROJECT_CREATED")]
    ProjectCreated,
    /// Origin not established. Cannot be distributed until resolved.
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

impl ProvenanceClass {
    pub fn label(self) -> &'static str {
        match self {
            ProvenanceClass::Reference => "REFERENCE",
            ProvenanceClass::Licensed => "LICENSED",
            ProvenanceClass::ProjectCreated => "PROJECT-CREATED",
            ProvenanceClass::Unknown => "UNKNOWN",
        }
    }

    /// Whether a file of this class may live in the distributable tree.
    pub fn distributable(self) -> bool {
        matches!(self, ProvenanceClass::Licensed | ProvenanceClass::ProjectCreated)
    }
}

impl fmt::Display for ProvenanceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One project asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceRecord {
    /// Path relative to the assets directory, `/`-separated.
    pub path: String,
    pub class: ProvenanceClass,
    /// Person or tool that produced the asset.
    pub author: Option<String>,
    pub description: Option<String>,
    /// For LICENSED: where it was obtained.
    pub source: Option<String>,
    /// For LICENSED: licence identifier (SPDX where one exists).
    pub licence: Option<String>,
    /// For LICENSED: path (in the repository) of the licence text.
    pub licence_file: Option<String>,
    /// Hex SHA-256 of the file; if present, it is verified.
    pub sha256: Option<String>,
    /// Reference virtual paths this asset stands in for (for "project
    /// equivalent" queries). Recording an equivalence copies nothing.
    #[serde(default)]
    pub reference_equivalent: Vec<String>,
}

/// Contents of `assets/provenance.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceManifest {
    pub schema: u32,
    #[serde(default, rename = "asset")]
    pub assets: Vec<ProvenanceRecord>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("cannot read {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("cannot parse {path}: {source}")]
    Parse { path: PathBuf, source: Box<toml::de::Error> },
    #[error("unsupported manifest schema {0} (expected 1)")]
    Schema(u32),
}

impl ProvenanceManifest {
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| ManifestError::Io { path: path.to_owned(), source })?;
        Self::parse(&text).map_err(|e| match e {
            ManifestError::Parse { source, .. } => ManifestError::Parse { path: path.to_owned(), source },
            other => other,
        })
    }

    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let manifest: Self =
            toml::from_str(text).map_err(|e| ManifestError::Parse { path: PathBuf::new(), source: Box::new(e) })?;
        if manifest.schema != 1 {
            return Err(ManifestError::Schema(manifest.schema));
        }
        Ok(manifest)
    }

    /// Project assets declared as equivalents of a reference virtual path.
    pub fn equivalents_of(&self, reference_virtual_path: &str) -> Vec<&ProvenanceRecord> {
        let wanted = vfs::normalize(reference_virtual_path);
        self.assets.iter().filter(|r| r.reference_equivalent.iter().any(|e| vfs::normalize(e) == wanted)).collect()
    }
}

/// A violation of the provenance rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProvenanceIssue {
    MissingRecord { path: String },
    MissingFile { path: String },
    DuplicateRecord { path: String },
    NotDistributable { path: String, class: ProvenanceClass },
    IncompleteLicence { path: String, missing: Vec<&'static str> },
    HashMismatch { path: String, expected: String, actual: String },
}

impl fmt::Display for ProvenanceIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvenanceIssue::MissingRecord { path } => {
                write!(f, "{path}: file has no provenance record")
            }
            ProvenanceIssue::MissingFile { path } => {
                write!(f, "{path}: record exists but file is missing")
            }
            ProvenanceIssue::DuplicateRecord { path } => write!(f, "{path}: more than one record"),
            ProvenanceIssue::NotDistributable { path, class } => {
                write!(f, "{path}: class {class} may not be in the distributable tree")
            }
            ProvenanceIssue::IncompleteLicence { path, missing } => {
                write!(f, "{path}: LICENSED asset missing {}", missing.join(", "))
            }
            ProvenanceIssue::HashMismatch { path, expected, actual } => {
                write!(f, "{path}: sha256 {actual} does not match recorded {expected}")
            }
        }
    }
}

/// Check an assets directory against its manifest.
pub fn check_assets_dir(assets_dir: &Path) -> Result<Vec<ProvenanceIssue>, ManifestError> {
    let manifest = ProvenanceManifest::load(&assets_dir.join(MANIFEST_FILE))?;
    let files =
        list_asset_files(assets_dir).map_err(|source| ManifestError::Io { path: assets_dir.to_owned(), source })?;
    Ok(check(&manifest, assets_dir, &files))
}

/// Relative (`/`-separated) paths of every asset file, sorted.
pub fn list_asset_files(assets_dir: &Path) -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(assets_dir).sort_by_file_name() {
        let entry = entry.map_err(std::io::Error::other)?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = relative_slash_path(assets_dir, entry.path());
        if !NON_ASSET_FILES.contains(&rel.as_str()) {
            out.push(rel);
        }
    }
    Ok(out)
}

pub(crate) fn relative_slash_path(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

fn check(manifest: &ProvenanceManifest, assets_dir: &Path, files: &[String]) -> Vec<ProvenanceIssue> {
    let mut issues = Vec::new();
    let mut records: BTreeMap<&str, &ProvenanceRecord> = BTreeMap::new();
    for record in &manifest.assets {
        if records.insert(record.path.as_str(), record).is_some() {
            issues.push(ProvenanceIssue::DuplicateRecord { path: record.path.clone() });
        }
    }
    for file in files {
        if !records.contains_key(file.as_str()) {
            issues.push(ProvenanceIssue::MissingRecord { path: file.clone() });
        }
    }
    for (path, record) in &records {
        let path = (*path).to_owned();
        if !files.contains(&path) {
            issues.push(ProvenanceIssue::MissingFile { path });
            continue;
        }
        if !record.class.distributable() {
            issues.push(ProvenanceIssue::NotDistributable { path: path.clone(), class: record.class });
        }
        if record.class == ProvenanceClass::Licensed {
            let missing: Vec<&'static str> = [
                ("licence", record.licence.is_none()),
                ("source", record.source.is_none()),
                ("licence_file", record.licence_file.is_none()),
            ]
            .into_iter()
            .filter_map(|(name, absent)| absent.then_some(name))
            .collect();
            if !missing.is_empty() {
                issues.push(ProvenanceIssue::IncompleteLicence { path: path.clone(), missing });
            }
        }
        if let Some(expected) = &record.sha256 {
            match sha256_file(&assets_dir.join(&path)) {
                Ok(actual) if actual.eq_ignore_ascii_case(expected) => {}
                Ok(actual) => issues.push(ProvenanceIssue::HashMismatch { path, expected: expected.clone(), actual }),
                Err(e) => issues.push(ProvenanceIssue::HashMismatch {
                    path,
                    expected: expected.clone(),
                    actual: format!("<unreadable: {e}>"),
                }),
            }
        }
    }
    issues
}

/// Hex SHA-256 of a file, streamed.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

/// Hex SHA-256 of bytes.
pub fn sha256_bytes(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(dir: &Path, rel: &str, content: &[u8]) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().expect("has parent")).expect("mkdir");
        fs::write(p, content).expect("write");
    }

    #[test]
    fn clean_tree_passes() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "README.md", b"docs");
        write(dir.path(), "env/grid.txt", b"grid");
        let hash = sha256_bytes(b"grid");
        write(
            dir.path(),
            MANIFEST_FILE,
            format!(
                "schema = 1\n[[asset]]\npath = \"env/grid.txt\"\nclass = \"PROJECT-CREATED\"\nsha256 = \"{hash}\"\nreference_equivalent = [\"\\\\CA\\\\Grid.paa\"]\n"
            )
            .as_bytes(),
        );
        assert_eq!(check_assets_dir(dir.path()).expect("loads"), []);
        let manifest = ProvenanceManifest::load(&dir.path().join(MANIFEST_FILE)).expect("loads");
        assert_eq!(manifest.equivalents_of("ca\\grid.paa").len(), 1);
    }

    #[test]
    fn every_rule_is_enforced() {
        let dir = tempfile::tempdir().expect("tempdir");
        for f in ["unrecorded.bin", "ref.dat", "unk.dat", "lic.dat", "hashed.dat"] {
            write(dir.path(), f, b"x");
        }
        write(
            dir.path(),
            MANIFEST_FILE,
            br#"schema = 1
[[asset]]
path = "ref.dat"
class = "REFERENCE"
[[asset]]
path = "unk.dat"
class = "UNKNOWN"
[[asset]]
path = "lic.dat"
class = "LICENSED"
licence = "CC-BY-4.0"
[[asset]]
path = "hashed.dat"
class = "PROJECT-CREATED"
sha256 = "00"
[[asset]]
path = "gone.dat"
class = "PROJECT-CREATED"
[[asset]]
path = "gone.dat"
class = "PROJECT-CREATED"
"#,
        );
        let issues = check_assets_dir(dir.path()).expect("loads");
        let text: Vec<String> = issues.iter().map(ToString::to_string).collect();
        let has = |needle: &str| text.iter().any(|t| t.contains(needle));
        assert!(has("unrecorded.bin: file has no provenance record"), "{text:?}");
        assert!(has("ref.dat: class REFERENCE may not be"), "{text:?}");
        assert!(has("unk.dat: class UNKNOWN may not be"), "{text:?}");
        assert!(has("lic.dat: LICENSED asset missing source, licence_file"), "{text:?}");
        assert!(has("hashed.dat: sha256"), "{text:?}");
        assert!(has("gone.dat: more than one record"), "{text:?}");
        assert!(has("gone.dat: record exists but file is missing"), "{text:?}");
    }

    #[test]
    fn unknown_fields_and_schema_are_rejected() {
        assert!(ProvenanceManifest::parse("schema = 2").is_err());
        assert!(
            ProvenanceManifest::parse("schema = 1\n[[asset]]\npath=\"a\"\nclass=\"PROJECT-CREATED\"\ntypo=1").is_err()
        );
        assert!(ProvenanceManifest::parse("schema = 1\n[[asset]]\npath=\"a\"\nclass=\"MINE\"").is_err());
    }
}
