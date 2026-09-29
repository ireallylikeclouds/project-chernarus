//! Repository guard: no Bohemia/DayZ container or asset formats in git, and
//! no media outside the provenance-tracked `assets/` tree.
//!
//! Complements `.gitignore` by checking file *content*, so a renamed PBO or
//! texture is caught as well. A texture converted to PNG cannot be told apart
//! from original work by content, which is why media files are only allowed
//! under `assets/`, where every file needs a provenance record. Local-only
//! directories documented in `docs/assets/provenance.md` are skipped because
//! they are ignored by git.

use crate::provenance::relative_slash_path;
use chernarus_formats::identify::{SNIFF_LEN, identify};
use chernarus_formats::{Format, Identification};
use serde::Serialize;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

/// Top-level directories never scanned: VCS data, build output and the
/// git-ignored local reference directories.
pub const SKIPPED_DIRS: &[&str] = &[".git", "target", "reference", "reference-data", "local"];

/// Directory whose files are covered by the provenance manifest.
pub const ASSETS_DIR: &str = "assets";

/// Why a file was flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Bohemia container/asset format or a Windows binary.
    ProprietaryFormat,
    /// Image or audio outside `assets/`, so it has no provenance record.
    UntrackedMedia,
}

/// A flagged file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub path: String,
    pub format: Format,
    pub reason: Reason,
    pub identification: Identification,
}

fn is_media(format: Format) -> bool {
    matches!(format, Format::Png | Format::Jpeg | Format::Ogg | Format::Wav)
}

/// Scan a repository working tree.
pub fn scan_repository(root: &Path) -> std::io::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let walker = WalkDir::new(root).sort_by_file_name().into_iter().filter_entry(|e| {
        let top_level_skip = e.depth() == 1 && SKIPPED_DIRS.iter().any(|d| e.file_name() == *d);
        !top_level_skip
    });
    for entry in walker {
        let entry = entry.map_err(std::io::Error::other)?;
        if !entry.file_type().is_file() {
            continue;
        }
        let mut head = Vec::with_capacity(SNIFF_LEN);
        std::fs::File::open(entry.path())?.take(SNIFF_LEN as u64).read_to_end(&mut head)?;
        let rel = relative_slash_path(root, entry.path());
        let id = identify(&rel, &head);
        let reason = if id.format.is_proprietary_container_or_asset() {
            Reason::ProprietaryFormat
        } else if is_media(id.format) && !rel.starts_with(&format!("{ASSETS_DIR}/")) {
            Reason::UntrackedMedia
        } else {
            continue;
        };
        findings.push(Finding { path: rel, format: id.format, reason, identification: id });
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chernarus_formats::synth;
    use std::fs;

    #[test]
    fn detects_renamed_and_extension_only_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join("docs")).expect("mkdir");
        fs::write(root.join("docs/notes.md"), "# notes").expect("write");
        fs::write(root.join("docs/innocent.txt"), synth::PboBuilder::new().build()).expect("write");
        fs::write(root.join("key.bikey"), b"anything").expect("write");
        fs::create_dir_all(root.join("reference")).expect("mkdir");
        fs::write(root.join("reference/local.pbo"), synth::PboBuilder::new().build()).expect("write");
        fs::create_dir_all(root.join("target")).expect("mkdir");
        fs::write(root.join("target/x.p3d"), synth::fake_odol(&[])).expect("write");
        fs::write(root.join("docs/diagram.png"), b"\x89PNG\r\n").expect("write");
        fs::create_dir_all(root.join("assets/ui")).expect("mkdir");
        fs::write(root.join("assets/ui/icon.png"), b"\x89PNG\r\n").expect("write");

        let findings = scan_repository(root).expect("scan");
        let paths: Vec<_> = findings.iter().map(|f| (f.path.as_str(), f.format, f.reason)).collect();
        assert_eq!(
            paths,
            [
                ("docs/diagram.png", Format::Png, Reason::UntrackedMedia),
                ("docs/innocent.txt", Format::Pbo, Reason::ProprietaryFormat),
                ("key.bikey", Format::BiKey, Reason::ProprietaryFormat),
            ]
        );
    }
}
