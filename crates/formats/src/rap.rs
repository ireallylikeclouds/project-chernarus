//! Rapified (binarized) configs: `config.bin`, binarized `.rvmat`, and other
//! files starting with `"\0raP"`.
//!
//! Status: ESTIMATED — written from the public format description and
//! cross-checked against open-source readers (`docs/reference/formats/rap.md`);
//! exercised only against configs written by [`write`].
//!
//! This is the gateway to most reference *data* (`CfgWeapons`, `CfgAmmo`,
//! `CfgVehicles`, `CfgMoves*`, loot tables). The reader produces a plain tree.
//! Inheritance and cross-addon merging (`class B: A`, `CfgPatches` load order)
//! are engine semantics and are deliberately not resolved here.
//!
//! Layout:
//! ```text
//! header   "\0raP", u32 0, u32 8, u32 enum_offset
//! body     asciiz parent_name; compressed_u32 count; entry[count]
//! entry    u8 0: asciiz name; u32 body_offset          (class)
//!          u8 1: u8 subtype; asciiz name; value        (scalar)
//!          u8 2: asciiz name; array                    (array)
//!          u8 3: asciiz name                           (external class)
//!          u8 4: asciiz name                           (delete class)
//!          u8 5: u32 flags; asciiz name; array         (array "+=")
//! scalar   subtype 0 asciiz | 1 f32 | 2 i32 | 4 asciiz (variable) | 6 i64
//! array    compressed_u32 count; (u8 type; value)[count], type 3 = nested array
//! enums    u32 count; (asciiz name; u32 value)[count] at enum_offset
//! ```

use crate::FormatError;
use crate::read::SliceReader;
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8; 4] = b"\0raP";
const MAX_DEPTH: usize = 128;
const MAX_ENTRIES: u32 = 1_000_000;

/// Scalar value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    String(String),
    Float(f32),
    Int(i32),
    /// Subtype 4. Documented as "variable"; not expected in binarized game
    /// data. ESTIMATED.
    Variable(String),
    /// Subtype 6 (introduced after ARMA 2, per documentation). ESTIMATED.
    Int64(i64),
}

/// Array element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    String(String),
    Float(f32),
    Int(i32),
    Array(Vec<Element>),
    Variable(String),
    Int64(i64),
}

/// A class body: optional parent name and ordered entries.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Class {
    /// Name after the colon in `class Name: Parent`, if any.
    pub parent: Option<String>,
    pub entries: Vec<Entry>,
}

/// A named entry in a class body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entry {
    Class {
        name: String,
        body: Class,
    },
    Value {
        name: String,
        value: Value,
    },
    Array {
        name: String,
        elements: Vec<Element>,
        append: bool,
    },
    /// `class Name;` — forward declaration of a class defined elsewhere.
    External {
        name: String,
    },
    /// `delete Name;`
    Delete {
        name: String,
    },
}

impl Entry {
    pub fn name(&self) -> &str {
        match self {
            Entry::Class { name, .. }
            | Entry::Value { name, .. }
            | Entry::Array { name, .. }
            | Entry::External { name }
            | Entry::Delete { name } => name,
        }
    }
}

impl Class {
    /// Case-insensitive lookup of an entry declared directly in this class
    /// (config names are case-insensitive in the engine). Inherited entries
    /// are not searched.
    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name().eq_ignore_ascii_case(name))
    }

    /// Case-insensitive lookup of a direct subclass body.
    pub fn class(&self, name: &str) -> Option<&Class> {
        match self.get(name)? {
            Entry::Class { body, .. } => Some(body),
            _ => None,
        }
    }

    /// Follow `path` through subclasses and return the final entry.
    pub fn lookup(&self, path: &[&str]) -> Option<&Entry> {
        let (last, classes) = path.split_last()?;
        let mut class = self;
        for name in classes {
            class = class.class(name)?;
        }
        class.get(last)
    }

    /// Visit every string (scalar or array element) with its entry path.
    pub fn visit_strings<F: FnMut(&[String], &str)>(&self, f: &mut F) {
        let mut path = Vec::new();
        self.visit_inner(&mut path, f);
    }

    fn visit_inner<F: FnMut(&[String], &str)>(&self, path: &mut Vec<String>, f: &mut F) {
        for entry in &self.entries {
            path.push(entry.name().to_owned());
            match entry {
                Entry::Class { body, .. } => body.visit_inner(path, f),
                Entry::Value { value: Value::String(s) | Value::Variable(s), .. } => f(path, s),
                Entry::Array { elements, .. } => visit_elements(elements, path, f),
                _ => {}
            }
            path.pop();
        }
    }
}

fn visit_elements<F: FnMut(&[String], &str)>(elements: &[Element], path: &[String], f: &mut F) {
    for e in elements {
        match e {
            Element::String(s) | Element::Variable(s) => f(path, s),
            Element::Array(inner) => visit_elements(inner, path, f),
            _ => {}
        }
    }
}

/// A parsed rapified config.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Config {
    pub root: Class,
    pub enums: Vec<(String, u32)>,
}

/// Parse a complete rapified config.
pub fn read(data: &[u8]) -> Result<Config, FormatError> {
    let mut r = SliceReader::new(data);
    if r.bytes(4, "raP magic")? != MAGIC {
        return Err(FormatError::invalid("raP magic", 0, "missing \\0raP"));
    }
    let always0 = r.u32("raP header")?;
    let always8 = r.u32("raP header")?;
    if always0 != 0 || always8 != 8 {
        return Err(FormatError::invalid("raP header", 4, format!("expected 0 and 8, found {always0} and {always8}")));
    }
    let enum_offset = r.u32("raP enum offset")?;
    let mut active = Vec::new();
    let root = read_body(data, r.pos(), 0, &mut active)?;
    let enums = read_enums(data, enum_offset as usize)?;
    Ok(Config { root, enums })
}

fn read_body(data: &[u8], offset: usize, depth: usize, active: &mut Vec<usize>) -> Result<Class, FormatError> {
    if depth > MAX_DEPTH {
        return Err(FormatError::LimitExceeded {
            what: "raP class nesting",
            detail: format!("deeper than {MAX_DEPTH}"),
        });
    }
    if active.contains(&offset) {
        return Err(FormatError::invalid("raP class body", offset as u64, "class body offset cycle"));
    }
    active.push(offset);
    let mut r = SliceReader::at(data, offset);
    let parent = r.asciiz("raP parent name")?;
    let count = r.compressed_u32("raP entry count")?;
    if count > MAX_ENTRIES || count as usize > r.remaining() {
        return Err(FormatError::invalid(
            "raP entry count",
            offset as u64,
            format!("{count} entries cannot fit in remaining data"),
        ));
    }
    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let at = r.pos() as u64;
        let entry = match r.u8("raP entry type")? {
            0 => {
                let name = r.asciiz("raP class name")?;
                let body_offset = r.u32("raP class offset")? as usize;
                let body = read_body(data, body_offset, depth + 1, active)?;
                Entry::Class { name, body }
            }
            1 => {
                let subtype = r.u8("raP value subtype")?;
                let name = r.asciiz("raP value name")?;
                let value = match subtype {
                    0 => Value::String(r.asciiz("raP string value")?),
                    1 => Value::Float(r.f32("raP float value")?),
                    2 => Value::Int(r.i32("raP int value")?),
                    4 => Value::Variable(r.asciiz("raP variable value")?),
                    6 => Value::Int64(r.i64("raP int64 value")?),
                    other => {
                        return Err(FormatError::invalid("raP value subtype", at, format!("unknown subtype {other}")));
                    }
                };
                Entry::Value { name, value }
            }
            2 => {
                let name = r.asciiz("raP array name")?;
                let elements = read_array(&mut r, depth)?;
                Entry::Array { name, elements, append: false }
            }
            3 => Entry::External { name: r.asciiz("raP external class name")? },
            4 => Entry::Delete { name: r.asciiz("raP delete class name")? },
            5 => {
                let _flags = r.u32("raP array flags")?;
                let name = r.asciiz("raP array name")?;
                let elements = read_array(&mut r, depth)?;
                Entry::Array { name, elements, append: true }
            }
            other => {
                return Err(FormatError::invalid("raP entry type", at, format!("unknown entry type {other}")));
            }
        };
        entries.push(entry);
    }
    active.pop();
    Ok(Class { parent: (!parent.is_empty()).then_some(parent), entries })
}

fn read_array(r: &mut SliceReader<'_>, depth: usize) -> Result<Vec<Element>, FormatError> {
    if depth > MAX_DEPTH {
        return Err(FormatError::LimitExceeded {
            what: "raP array nesting",
            detail: format!("deeper than {MAX_DEPTH}"),
        });
    }
    let count = r.compressed_u32("raP array count")?;
    if count as usize > r.remaining() {
        return Err(FormatError::invalid(
            "raP array count",
            r.pos() as u64,
            format!("{count} elements cannot fit in remaining data"),
        ));
    }
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let at = r.pos() as u64;
        out.push(match r.u8("raP element type")? {
            0 => Element::String(r.asciiz("raP string element")?),
            1 => Element::Float(r.f32("raP float element")?),
            2 => Element::Int(r.i32("raP int element")?),
            3 => Element::Array(read_array(r, depth + 1)?),
            4 => Element::Variable(r.asciiz("raP variable element")?),
            6 => Element::Int64(r.i64("raP int64 element")?),
            other => {
                return Err(FormatError::invalid("raP element type", at, format!("unknown element type {other}")));
            }
        });
    }
    Ok(out)
}

fn read_enums(data: &[u8], offset: usize) -> Result<Vec<(String, u32)>, FormatError> {
    if offset == 0 || offset >= data.len() {
        return Ok(Vec::new());
    }
    let mut r = SliceReader::at(data, offset);
    let count = r.u32("raP enum count")?;
    if count as usize > r.remaining() {
        return Err(FormatError::invalid("raP enum count", offset as u64, format!("{count} enums cannot fit")));
    }
    (0..count).map(|_| Ok((r.asciiz("raP enum name")?, r.u32("raP enum value")?))).collect()
}

/// Render a config as config-source text (for local inspection only; output
/// derived from reference files must not be committed).
pub fn to_text(config: &Config) -> String {
    let mut out = String::new();
    for (name, value) in &config.enums {
        out.push_str(&format!("// enum {name} = {value}\n"));
    }
    text_entries(&mut out, &config.root.entries, 0);
    out
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn float_text(f: f32) -> String {
    let s = format!("{f}");
    if s.contains(['.', 'e', 'i', 'N']) { s } else { format!("{s}.0") }
}

fn element_text(e: &Element) -> String {
    match e {
        Element::String(s) => quote(s),
        Element::Float(f) => float_text(*f),
        Element::Int(i) => i.to_string(),
        Element::Int64(i) => i.to_string(),
        Element::Variable(v) => v.clone(),
        Element::Array(items) => format!("{{{}}}", items.iter().map(element_text).collect::<Vec<_>>().join(", ")),
    }
}

fn text_entries(out: &mut String, entries: &[Entry], depth: usize) {
    let pad = "\t".repeat(depth);
    for entry in entries {
        match entry {
            Entry::Class { name, body } => {
                match &body.parent {
                    Some(p) => out.push_str(&format!("{pad}class {name}: {p}\n{pad}{{\n")),
                    None => out.push_str(&format!("{pad}class {name}\n{pad}{{\n")),
                }
                text_entries(out, &body.entries, depth + 1);
                out.push_str(&format!("{pad}}};\n"));
            }
            Entry::Value { name, value } => {
                let v = match value {
                    Value::String(s) => quote(s),
                    Value::Float(f) => float_text(*f),
                    Value::Int(i) => i.to_string(),
                    Value::Int64(i) => i.to_string(),
                    Value::Variable(v) => v.clone(),
                };
                out.push_str(&format!("{pad}{name} = {v};\n"));
            }
            Entry::Array { name, elements, append } => {
                let op = if *append { "+=" } else { "=" };
                let items = elements.iter().map(element_text).collect::<Vec<_>>().join(", ");
                out.push_str(&format!("{pad}{name}[] {op} {{{items}}};\n"));
            }
            Entry::External { name } => out.push_str(&format!("{pad}class {name};\n")),
            Entry::Delete { name } => out.push_str(&format!("{pad}delete {name};\n")),
        }
    }
}

/// Write a config in rapified form. Class bodies are laid out depth-first
/// after their parent's entry list. Used for synthetic fixtures.
pub fn write(config: &Config) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // enum offset, patched below
    write_body(&mut out, &config.root);
    let enum_offset = out.len() as u32;
    out[12..16].copy_from_slice(&enum_offset.to_le_bytes());
    out.extend_from_slice(&(config.enums.len() as u32).to_le_bytes());
    for (name, value) in &config.enums {
        put_asciiz(&mut out, name);
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

fn put_asciiz(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

fn put_compressed(out: &mut Vec<u8>, mut v: u32) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn write_body(out: &mut Vec<u8>, class: &Class) {
    put_asciiz(out, class.parent.as_deref().unwrap_or(""));
    put_compressed(out, class.entries.len() as u32);
    let mut pending = Vec::new();
    for entry in &class.entries {
        match entry {
            Entry::Class { name, body } => {
                out.push(0);
                put_asciiz(out, name);
                pending.push((out.len(), body));
                out.extend_from_slice(&0u32.to_le_bytes());
            }
            Entry::Value { name, value } => {
                out.push(1);
                let subtype = match value {
                    Value::String(_) => 0,
                    Value::Float(_) => 1,
                    Value::Int(_) => 2,
                    Value::Variable(_) => 4,
                    Value::Int64(_) => 6,
                };
                out.push(subtype);
                put_asciiz(out, name);
                match value {
                    Value::String(s) | Value::Variable(s) => put_asciiz(out, s),
                    Value::Float(f) => out.extend_from_slice(&f.to_le_bytes()),
                    Value::Int(i) => out.extend_from_slice(&i.to_le_bytes()),
                    Value::Int64(i) => out.extend_from_slice(&i.to_le_bytes()),
                }
            }
            Entry::Array { name, elements, append } => {
                if *append {
                    out.push(5);
                    out.extend_from_slice(&1u32.to_le_bytes());
                } else {
                    out.push(2);
                }
                put_asciiz(out, name);
                write_array(out, elements);
            }
            Entry::External { name } => {
                out.push(3);
                put_asciiz(out, name);
            }
            Entry::Delete { name } => {
                out.push(4);
                put_asciiz(out, name);
            }
        }
    }
    for (slot, body) in pending {
        let offset = out.len() as u32;
        out[slot..slot + 4].copy_from_slice(&offset.to_le_bytes());
        write_body(out, body);
    }
}

fn write_array(out: &mut Vec<u8>, elements: &[Element]) {
    put_compressed(out, elements.len() as u32);
    for e in elements {
        match e {
            Element::String(s) => {
                out.push(0);
                put_asciiz(out, s);
            }
            Element::Float(f) => {
                out.push(1);
                out.extend_from_slice(&f.to_le_bytes());
            }
            Element::Int(i) => {
                out.push(2);
                out.extend_from_slice(&i.to_le_bytes());
            }
            Element::Array(inner) => {
                out.push(3);
                write_array(out, inner);
            }
            Element::Variable(s) => {
                out.push(4);
                put_asciiz(out, s);
            }
            Element::Int64(i) => {
                out.push(6);
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> String {
        v.to_owned()
    }

    fn sample() -> Config {
        Config {
            root: Class {
                parent: None,
                entries: vec![
                    Entry::Class {
                        name: s("CfgPatches"),
                        body: Class {
                            parent: None,
                            entries: vec![Entry::Class {
                                name: s("synth_test"),
                                body: Class {
                                    parent: None,
                                    entries: vec![
                                        Entry::Array { name: s("units"), elements: vec![], append: false },
                                        Entry::Value { name: s("requiredVersion"), value: Value::Float(1.62) },
                                    ],
                                },
                            }],
                        },
                    },
                    Entry::Class {
                        name: s("CfgVehicles"),
                        body: Class {
                            parent: None,
                            entries: vec![
                                Entry::External { name: s("ThingBase") },
                                Entry::Class {
                                    name: s("SynthBox"),
                                    body: Class {
                                        parent: Some(s("ThingBase")),
                                        entries: vec![
                                            Entry::Value {
                                                name: s("model"),
                                                value: Value::String(s("\\synth\\test\\box.p3d")),
                                            },
                                            Entry::Value { name: s("scope"), value: Value::Int(2) },
                                            Entry::Array {
                                                name: s("sound"),
                                                elements: vec![
                                                    Element::String(s("\\synth\\test\\open")),
                                                    Element::Float(0.5),
                                                    Element::Array(vec![Element::Int(1), Element::String(s("x.paa"))]),
                                                ],
                                                append: true,
                                            },
                                        ],
                                    },
                                },
                                Entry::Delete { name: s("OldBox") },
                            ],
                        },
                    },
                ],
            },
            enums: vec![(s("destructengine"), 2), (s("destructno"), 0)],
        }
    }

    #[test]
    fn round_trip() {
        let cfg = sample();
        let bytes = write(&cfg);
        assert_eq!(&bytes[..4], MAGIC);
        assert_eq!(read(&bytes).expect("valid"), cfg);
    }

    #[test]
    fn lookup_is_case_insensitive() {
        let cfg = sample();
        match cfg.root.lookup(&["cfgvehicles", "SYNTHBOX", "Model"]) {
            Some(Entry::Value { value: Value::String(m), .. }) => assert_eq!(m, "\\synth\\test\\box.p3d"),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            cfg.root.class("CfgVehicles").and_then(|c| c.class("SynthBox")).and_then(|c| c.parent.as_deref()),
            Some("ThingBase")
        );
        assert!(cfg.root.lookup(&["CfgVehicles", "Missing", "model"]).is_none());
    }

    #[test]
    fn visits_all_strings_with_paths() {
        let mut seen = Vec::new();
        sample().root.visit_strings(&mut |path, v| seen.push((path.join("/"), v.to_owned())));
        assert_eq!(
            seen,
            [
                (s("CfgVehicles/SynthBox/model"), s("\\synth\\test\\box.p3d")),
                (s("CfgVehicles/SynthBox/sound"), s("\\synth\\test\\open")),
                (s("CfgVehicles/SynthBox/sound"), s("x.paa")),
            ]
        );
    }

    #[test]
    fn renders_config_text() {
        let text = to_text(&sample());
        for expected in [
            "// enum destructengine = 2",
            "class CfgVehicles\n{",
            "\tclass ThingBase;",
            "\tclass SynthBox: ThingBase\n\t{",
            "\t\tmodel = \"\\synth\\test\\box.p3d\";",
            "\t\tsound[] += {\"\\synth\\test\\open\", 0.5, {1, \"x.paa\"}};",
            "\t\trequiredVersion = 1.62;",
            "\t\tunits[] = {};",
            "\tdelete OldBox;",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
        }
    }

    #[test]
    fn malformed_input_is_an_error() {
        assert!(read(b"").is_err());
        assert!(read(b"\0raP\0\0\0\0\x09\0\0\0\0\0\0\0").is_err());
        let bytes = write(&sample());
        for cut in [5, 17, 30, bytes.len() / 2] {
            assert!(read(&bytes[..cut]).is_err(), "truncated at {cut}");
        }
    }

    #[test]
    fn class_offset_cycle_is_detected() {
        // Root body: no parent, one class entry "A" whose body offset points back at the root body.
        let mut bytes = Vec::from(&MAGIC[..]);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&8u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&[0, 1, 0, b'A', 0]);
        bytes.extend_from_slice(&16u32.to_le_bytes());
        assert!(read(&bytes).is_err());
    }
}
