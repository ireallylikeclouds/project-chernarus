//! Extraction of asset-path-like strings from text or binary content.
//!
//! Used as a *heuristic* dependency source for formats this project cannot
//! parse yet (ODOL models, terrains) and for text configs. Every edge found
//! this way is labelled with the method that produced it, so it is never
//! confused with an edge from a real parser.

/// Extensions that mark a string as a reference to another asset.
pub const ASSET_EXTENSIONS: &[&str] = &[
    "p3d", "paa", "pac", "rvmat", "rtm", "wss", "ogg", "wav", "lip", "sqf", "sqs", "fsm", "bisurf", "jpg", "hpp", "h",
    "bikb", "wrp",
];

fn is_path_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'\\' | b'/' | b'.' | b'-' | b'$' | b'@')
}

/// Whether `s` ends in one of [`ASSET_EXTENSIONS`] (case-insensitive) and has
/// something before the dot.
pub fn has_asset_extension(s: &str) -> bool {
    match s.rfind('.') {
        Some(i) if i > 0 => {
            let ext = &s[i + 1..];
            ASSET_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext))
        }
        _ => false,
    }
}

/// Every maximal run of path characters that ends in an asset extension.
/// Results keep their original spelling and are de-duplicated in order.
pub fn asset_paths(data: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut start = None;
    for (i, &b) in data.iter().chain(std::iter::once(&0u8)).enumerate() {
        match (is_path_byte(b), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                // Only ASCII bytes are accepted above, so this is valid UTF-8.
                let run = String::from_utf8_lossy(&data[s..i]);
                let run = run.trim_end_matches('.');
                if run.len() >= 5 && has_asset_extension(run) && !out.iter().any(|o| o == run) {
                    out.push(run.to_owned());
                }
                start = None;
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_paths_in_binary_and_text() {
        let mut bin = b"ODOL\x07\0\0\0junk\0".to_vec();
        bin.extend_from_slice(b"ca\\data\\box_co.paa\0\x01\x02ca\\data\\box.rvmat\0ca\\data\\box_co.paa\0");
        assert_eq!(asset_paths(&bin), ["ca\\data\\box_co.paa", "ca\\data\\box.rvmat"]);

        let text = b"texture = \"\\synth\\x\\t_co.paa\";\n#include \"defs.hpp\"\nmodel = \"not a path\";";
        assert_eq!(asset_paths(text), ["\\synth\\x\\t_co.paa", "defs.hpp"]);
    }

    #[test]
    fn ignores_non_asset_words() {
        assert!(asset_paths(b"version 1.63 build.exe readme.txt").is_empty());
        assert!(!has_asset_extension(".paa"));
        assert!(has_asset_extension("A.PAA"));
    }
}
