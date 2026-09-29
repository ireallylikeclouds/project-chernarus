//! A PROJECT-CREATED synthetic "reference installation" for tests and demos.
//!
//! It mimics only the *shape* of an ARMA 2 OA + mod installation (folders,
//! PBO containers, config/model/texture signatures, cross-addon references).
//! Every name and byte is invented; nothing is derived from Bohemia or DayZ
//! files. It exercises each discovery code path, including the failure ones.

use chernarus_formats::rap::{Class, Config, Element, Entry, Value};
use chernarus_formats::{rap, synth};
use std::fs;
use std::io;
use std::path::Path;

fn class(name: &str, parent: Option<&str>, entries: Vec<Entry>) -> Entry {
    Entry::Class { name: name.to_owned(), body: Class { parent: parent.map(str::to_owned), entries } }
}

fn string(name: &str, value: &str) -> Entry {
    Entry::Value { name: name.to_owned(), value: Value::String(value.to_owned()) }
}

fn array(name: &str, elements: Vec<Element>) -> Entry {
    Entry::Array { name: name.to_owned(), elements, append: false }
}

fn config(entries: Vec<Entry>) -> Vec<u8> {
    rap::write(&Config { root: Class { parent: None, entries }, enums: Vec::new() })
}

fn put(root: &Path, rel: &str, data: &[u8]) -> io::Result<()> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, data)
}

/// Write the synthetic installation under `root`.
pub fn write_synthetic_installation(root: &Path) -> io::Result<()> {
    put(root, "SynthGame.exe", &synth::fake_pe())?;

    let core_config = config(vec![
        class("CfgPatches", None, vec![class("synth_core", None, vec![array("units", vec![])])]),
        class(
            "CfgVehicles",
            None,
            vec![class(
                "SynthThing",
                None,
                vec![string("model", "\\synth\\core\\models\\thing.p3d"), string("displayName", "Synthetic thing")],
            )],
        ),
    ]);
    let core = synth::PboBuilder::new()
        .extension("prefix", "synth\\core")
        .extension("product", "synthetic-fixture")
        .file("config.bin", core_config)
        .file(
            "models\\thing.p3d",
            synth::fake_odol(&["synth\\core\\data\\thing_co.paa", "synth\\core\\data\\thing.rvmat"]),
        )
        .file("data\\thing_co.paa", synth::fake_paa())
        .compressed_file(
            "data\\thing.rvmat",
            b"class Stage1\n{\n\ttexture = \"synth\\core\\data\\thing_nohq.paa\";\n};\n".to_vec(),
        )
        .file("data\\mislabeled.paa", b"OggS\x00\x02synthetic".to_vec())
        .build();
    put(root, "Expansion/Addons/synth_core.pbo", &core)?;

    let mod_config = config(vec![
        class("CfgPatches", None, vec![class("synth_mod", None, vec![array("units", vec![])])]),
        class(
            "CfgVehicles",
            None,
            vec![
                Entry::External { name: "SynthThing".to_owned() },
                class(
                    "SynthZombie",
                    Some("SynthThing"),
                    vec![
                        string("model", "\\synth\\mod\\zombie.p3d"),
                        array(
                            "soundGrowl",
                            vec![
                                Element::String("\\synth\\mod\\sounds\\growl".to_owned()),
                                Element::Float(1.0),
                                Element::Float(1.0),
                            ],
                        ),
                        array(
                            "hiddenSelectionsTextures",
                            vec![Element::String("\\synth\\core\\data\\thing_co.paa".to_owned())],
                        ),
                    ],
                ),
            ],
        ),
    ]);
    let mod_pbo = synth::PboBuilder::new()
        .extension("prefix", "synth\\mod")
        .file("config.bin", mod_config)
        .file("zombie.p3d", synth::fake_odol(&["synth\\mod\\data\\zombie_co.paa", "data\\zombie_hair.paa"]))
        .file("data\\zombie_co.paa", synth::fake_paa())
        .file("data\\zombie_hair.paa", synth::fake_paa())
        .file("sounds\\growl.wss", synth::fake_wss())
        .file("init.sqf", b"// synthetic\n[] execVM \"\\synth\\mod\\scripts\\spawn.sqf\";\n".to_vec())
        .file("scripts\\spawn.sqf", b"// synthetic\nhint \"spawn\";\n".to_vec())
        .build();
    put(root, "@SynthMod/Addons/synth_mod.pbo", &mod_pbo)?;

    let old_style = synth::PboBuilder::new()
        .without_product_entry()
        .file("readme.txt", b"synthetic archive without a product entry".to_vec())
        .build();
    put(root, "@SynthMod/Addons/old_style.pbo", &old_style)?;
    put(root, "@SynthMod/Addons/broken.pbo", b"\0sreV\0\0\0\0truncated")?;
    put(root, "@SynthMod/Keys/synth.bikey", b"synthetic key placeholder")?;
    Ok(())
}
