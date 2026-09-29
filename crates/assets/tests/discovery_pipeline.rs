//! End-to-end discovery over the synthetic installation: identification,
//! PBO mounting, dependency extraction and resolution, queries, determinism.

use chernarus_assets::catalog::{Catalog, EdgeMethod, PrefixSource};
use chernarus_assets::discovery::{ScanOptions, scan};
use chernarus_assets::fixture::write_synthetic_installation;
use chernarus_assets::provenance::{ProvenanceClass, ProvenanceManifest};
use chernarus_assets::query;
use chernarus_formats::Format;
use chernarus_formats::lzss::ChecksumKind;
use chernarus_formats::pbo::Checksum;

fn scanned() -> (tempfile::TempDir, Catalog) {
    let dir = tempfile::tempdir().expect("tempdir");
    write_synthetic_installation(dir.path()).expect("fixture");
    let catalog = scan(dir.path(), &ScanOptions::default()).expect("scan");
    (dir, catalog)
}

fn edge_targets(catalog: &Catalog, from: &str) -> Vec<(String, EdgeMethod, bool)> {
    catalog.edges.iter().filter(|e| e.from == from).map(|e| (e.target.clone(), e.method, e.resolved())).collect()
}

#[test]
fn identifies_files_and_entries() {
    let (_dir, catalog) = scanned();
    let index = catalog.index();
    let get = |id: &str| *index.by_id.get(id).unwrap_or_else(|| panic!("missing {id}"));

    assert_eq!(get("SynthGame.exe").format, Format::PeExecutable);
    assert_eq!(get("@SynthMod/Keys/synth.bikey").format, Format::BiKey);
    let core = get("Expansion/Addons/synth_core.pbo");
    assert_eq!(core.format, Format::Pbo);
    let details = core.pbo.as_ref().expect("pbo details");
    assert_eq!(details.prefix.as_deref(), Some("synth\\core"));
    assert_eq!(details.checksum, Some(Checksum::Valid));
    assert_eq!(details.entry_count, 5);

    let model = get("Expansion/Addons/synth_core.pbo::models\\thing.p3d");
    assert_eq!(model.virtual_path.as_deref(), Some("synth\\core\\models\\thing.p3d"));
    assert_eq!(model.format, Format::P3dOdol);
    assert!(!model.redistributable);
    assert_eq!(model.provenance, ProvenanceClass::Reference);

    let rvmat = get("Expansion/Addons/synth_core.pbo::data\\thing.rvmat");
    assert_eq!(rvmat.format, Format::TextConfig);
    assert_eq!(rvmat.lzss_checksum, Some(ChecksumKind::Ambiguous));

    let mislabeled = get("Expansion/Addons/synth_core.pbo::data\\mislabeled.paa");
    assert_eq!(mislabeled.format, Format::Ogg);
    assert!(mislabeled.format_conflict);

    let old = get("@SynthMod/Addons/old_style.pbo");
    let old_details = old.pbo.as_ref().expect("pbo details");
    assert_eq!(old_details.prefix_source, PrefixSource::FileName);
    assert_eq!(
        get("@SynthMod/Addons/old_style.pbo::readme.txt").virtual_path.as_deref(),
        Some("old_style\\readme.txt")
    );

    assert!(catalog.assets.iter().all(|a| a.sha256.is_some()));
    assert!(
        catalog.issues.iter().any(|i| i.asset == "@SynthMod/Addons/broken.pbo"),
        "broken archive is reported, not fatal: {:?}",
        catalog.issues
    );
}

#[test]
fn extracts_and_resolves_dependencies() {
    let (_dir, catalog) = scanned();
    let mod_cfg = "@SynthMod/Addons/synth_mod.pbo::config.bin";
    assert_eq!(
        edge_targets(&catalog, mod_cfg),
        [
            ("synth\\core\\data\\thing_co.paa".to_owned(), EdgeMethod::ConfigParse, true),
            ("synth\\mod\\sounds\\growl.wss".to_owned(), EdgeMethod::ConfigImplicitExtension, true),
            ("synth\\mod\\zombie.p3d".to_owned(), EdgeMethod::ConfigParse, true),
        ]
    );
    // Model references: absolute and model-relative.
    assert_eq!(
        edge_targets(&catalog, "@SynthMod/Addons/synth_mod.pbo::zombie.p3d"),
        [
            ("synth\\mod\\data\\zombie_co.paa".to_owned(), EdgeMethod::BinaryStringScan, true),
            ("synth\\mod\\data\\zombie_hair.paa".to_owned(), EdgeMethod::BinaryStringScan, true),
        ]
    );
    // A reference to a file the installation does not contain stays unresolved.
    assert_eq!(
        edge_targets(&catalog, "Expansion/Addons/synth_core.pbo::data\\thing.rvmat"),
        [("synth\\core\\data\\thing_nohq.paa".to_owned(), EdgeMethod::TextScan, false)]
    );
    assert_eq!(
        edge_targets(&catalog, "@SynthMod/Addons/synth_mod.pbo::init.sqf"),
        [("synth\\mod\\scripts\\spawn.sqf".to_owned(), EdgeMethod::TextScan, true)]
    );
}

#[test]
fn answers_asset_questions() {
    let (_dir, catalog) = scanned();
    let index = catalog.index();
    let hits = query::find(&index, "\\SYNTH\\core\\data\\thing_co.paa");
    assert_eq!(hits.len(), 1);
    let manifest = ProvenanceManifest::parse(
        "schema = 1\n[[asset]]\npath = \"props/thing_albedo.png\"\nclass = \"PROJECT-CREATED\"\nreference_equivalent = [\"synth\\\\core\\\\data\\\\thing_co.paa\"]\n",
    )
    .expect("manifest");
    let card = query::describe(&catalog, &index, hits[0], Some(&manifest));
    let dependents: Vec<_> = card.depended_on_by.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(
        dependents,
        ["@SynthMod/Addons/synth_mod.pbo::config.bin", "Expansion/Addons/synth_core.pbo::models\\thing.p3d"]
    );
    assert_eq!(card.project_equivalents, ["assets/props/thing_albedo.png"]);
    assert_eq!(card.status, "HAS_PROJECT_EQUIVALENT");

    let model = query::find(&index, "synth/core/models/thing.p3d");
    let card = query::describe(&catalog, &index, model[0], None);
    assert_eq!(card.status, "REFERENCE_ONLY");
    let text = card.to_string();
    assert!(text.contains("Asset: synth\\core\\models\\thing.p3d"), "{text}");
    assert!(text.contains("Provenance: REFERENCE"), "{text}");
    assert!(text.contains("synth\\core\\data\\thing.rvmat"), "{text}");
}

#[test]
fn scans_are_deterministic_and_round_trip() {
    let (dir, first) = scanned();
    let second = scan(dir.path(), &ScanOptions::default()).expect("rescan");
    assert_eq!(first, second);
    let path = dir.path().join("catalog.json");
    first.save(&path).expect("save");
    assert_eq!(Catalog::load(&path).expect("load"), first);
}

#[test]
fn metadata_only_scan_skips_content_work() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_synthetic_installation(dir.path()).expect("fixture");
    let options =
        ScanOptions { hash: false, dependencies: false, verify_pbo_checksums: false, ..ScanOptions::default() };
    let catalog = scan(dir.path(), &options).expect("scan");
    assert!(catalog.edges.is_empty());
    assert!(catalog.assets.iter().all(|a| a.sha256.is_none()));
    // Identification still works for compressed entries (prefix decoding).
    let index = catalog.index();
    let rvmat = index.by_id["Expansion/Addons/synth_core.pbo::data\\thing.rvmat"];
    assert_eq!(rvmat.format, Format::TextConfig);
}
