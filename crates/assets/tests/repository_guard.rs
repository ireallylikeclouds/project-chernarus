//! Enforces the legal/asset boundary on this repository itself:
//! no proprietary formats anywhere, and a clean provenance manifest.

use chernarus_assets::guard::scan_repository;
use chernarus_assets::provenance::check_assets_dir;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repository root")
}

#[test]
fn repository_contains_no_proprietary_or_untracked_media_files() {
    let findings = scan_repository(&repo_root()).expect("scan repository");
    assert!(
        findings.is_empty(),
        "files that must not be committed (see docs/assets/provenance.md):\n{}",
        findings
            .iter()
            .map(|f| format!("  {} — {:?} ({:?})", f.path, f.format, f.reason))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn project_assets_have_valid_provenance() {
    let issues = check_assets_dir(&repo_root().join("assets")).expect("assets/provenance.toml loads");
    assert!(
        issues.is_empty(),
        "provenance issues:\n{}",
        issues.iter().map(|i| format!("  {i}")).collect::<Vec<_>>().join("\n")
    );
}
