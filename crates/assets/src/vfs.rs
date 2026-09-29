//! Engine virtual paths.
//!
//! The engine addresses files inside PBOs as `<prefix>\<entry name>`,
//! case-insensitively, with backslash separators; references in configs and
//! models may or may not start with a backslash. Normalised form used here:
//! lower case, backslashes, no leading separator, no `.`/`..` segments.
//! Status of these rules: ESTIMATED (see `docs/assets/discovery-pipeline.md`).

/// Normalise a reference or mounted path.
pub fn normalize(path: &str) -> String {
    let lowered = path.trim().to_ascii_lowercase().replace('/', "\\");
    let mut parts: Vec<&str> = Vec::new();
    for seg in lowered.split('\\') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("\\")
}

/// Directory part of a normalised path (`""` at the root).
pub fn parent(path: &str) -> &str {
    path.rfind('\\').map_or("", |i| &path[..i])
}

/// Join a relative reference onto a normalised directory.
pub fn join(dir: &str, reference: &str) -> String {
    if dir.is_empty() { normalize(reference) } else { normalize(&format!("{dir}\\{reference}")) }
}

/// Virtual path of a PBO entry.
pub fn mounted(prefix: &str, entry_name: &str) -> String {
    join(&normalize(prefix), entry_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation() {
        assert_eq!(normalize("\\CA\\Data\\Box_CO.paa"), "ca\\data\\box_co.paa");
        assert_eq!(normalize("ca/data//./x.p3d"), "ca\\data\\x.p3d");
        assert_eq!(normalize(" dz\\a\\..\\b.sqf "), "dz\\b.sqf");
        assert_eq!(normalize("..\\..\\x"), "x");
    }

    #[test]
    fn joining_and_mounting() {
        assert_eq!(parent("a\\b\\c.p3d"), "a\\b");
        assert_eq!(parent("c.p3d"), "");
        assert_eq!(join("a\\b", "data\\t.paa"), "a\\b\\data\\t.paa");
        assert_eq!(mounted("\\Synth\\Equip", "models\\box.p3d"), "synth\\equip\\models\\box.p3d");
        assert_eq!(mounted("", "x.sqf"), "x.sqf");
    }
}
