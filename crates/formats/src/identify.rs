//! Format identification by content signature and file extension.
//!
//! Signatures come from public format documentation and are ESTIMATED until
//! a scan of the reference installation confirms them. The scanner reports
//! both the signature-based and the extension-based guess, so any file where
//! they disagree becomes a lead for correcting this table (see
//! `docs/reference/formats/README.md`).

use serde::{Deserialize, Serialize};

/// Bytes of content needed to identify every signature below.
pub const SNIFF_LEN: usize = 64;

/// Known (or suspected) formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    /// Addon archive.
    Pbo,
    /// Texture (`.paa`, `.pac`): u16 type tag followed by `"GGAT"` tagg blocks.
    Paa,
    /// Editable model, `"MLOD"`.
    P3dMlod,
    /// Binarized model, `"ODOL"`.
    P3dOdol,
    /// Binarized terrain, `"OPRW"`.
    WrpOprw,
    /// Editable terrain, `"4WVR"` / `"8WVR"`.
    WrpEditable,
    /// Editable animation, `"RTM_0101"`.
    RtmMlod,
    /// Binarized animation, `"BMTR"`.
    RtmBinarized,
    /// Rapified config, `"\0raP"`.
    RapConfig,
    /// Sound, `"WSS0"`.
    Wss,
    Ogg,
    Wav,
    Jpeg,
    Png,
    /// Windows executable or DLL. Recorded, never parsed or copied.
    PeExecutable,
    /// Addon signing key (`.bikey`). Identified by extension only.
    BiKey,
    /// Addon signature (`.bisign`). Identified by extension only.
    BiSign,
    /// Text config (`.cpp`, `.hpp`, `.h`, text `.rvmat`, `.bisurf`, `.ext`, `.cfg`).
    TextConfig,
    /// Script (`.sqf`, `.sqs`, `.fsm`).
    Script,
    /// Mission (`.sqm`), text form.
    Mission,
    /// Stringtable (`.csv`, `.xml`).
    Stringtable,
    /// Other text (`.txt`, `.html`, `.lip`, `.bikb`, ...).
    Text,
    Unknown,
}

impl Format {
    /// Formats whose content is Bohemia or third-party game data and must
    /// never be committed to this repository.
    pub fn is_proprietary_container_or_asset(self) -> bool {
        matches!(
            self,
            Format::Pbo
                | Format::Paa
                | Format::P3dMlod
                | Format::P3dOdol
                | Format::WrpOprw
                | Format::WrpEditable
                | Format::RtmMlod
                | Format::RtmBinarized
                | Format::RapConfig
                | Format::Wss
                | Format::PeExecutable
                | Format::BiKey
                | Format::BiSign
        )
    }

    /// Whether the content is expected to be text (for dependency scanning).
    pub fn is_text(self) -> bool {
        matches!(self, Format::TextConfig | Format::Script | Format::Mission | Format::Stringtable | Format::Text)
    }
}

/// What the identification was based on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// Content signature.
    Signature,
    /// Extension only (no signature exists or content unavailable).
    Extension,
    /// Content looked like text; extension did not say which kind.
    TextHeuristic,
    None,
}

/// Result of identifying one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identification {
    pub format: Format,
    pub basis: Basis,
    /// Format implied by the extension, if the extension is known.
    pub by_extension: Option<Format>,
    /// Signature and extension disagree.
    pub conflict: bool,
}

/// Identify from a file name (or path) and the first bytes of its content.
pub fn identify(name: &str, head: &[u8]) -> Identification {
    let by_extension = format_for_extension(extension(name));
    let by_signature = format_for_signature(head);
    let (format, basis) = match (by_signature, by_extension) {
        (Some(f), _) => (f, Basis::Signature),
        (None, Some(f)) => (f, Basis::Extension),
        (None, None) if looks_like_text(head) && !head.is_empty() => (Format::Text, Basis::TextHeuristic),
        (None, None) => (Format::Unknown, Basis::None),
    };
    let conflict = match (by_signature, by_extension) {
        (Some(s), Some(e)) => s != e && !compatible(s, e),
        // A binary-only extension on content with no known signature.
        (None, Some(e)) => has_signature(e) && !head.is_empty(),
        _ => false,
    };
    Identification { format, basis, by_extension, conflict }
}

/// Lower-case extension without the dot (`""` if none).
pub fn extension(name: &str) -> String {
    let file = name.rsplit(['\\', '/']).next().unwrap_or(name);
    match file.rfind('.') {
        Some(i) if i > 0 || file.len() > 1 => file[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

fn format_for_extension(ext: String) -> Option<Format> {
    Some(match ext.as_str() {
        "pbo" => Format::Pbo,
        "paa" | "pac" => Format::Paa,
        "p3d" => Format::P3dOdol,
        "wrp" => Format::WrpOprw,
        "rtm" => Format::RtmBinarized,
        "bin" => Format::RapConfig,
        "wss" => Format::Wss,
        "ogg" => Format::Ogg,
        "wav" => Format::Wav,
        "jpg" | "jpeg" => Format::Jpeg,
        "png" => Format::Png,
        "exe" | "dll" => Format::PeExecutable,
        "bikey" => Format::BiKey,
        "bisign" => Format::BiSign,
        "cpp" | "hpp" | "h" | "rvmat" | "bisurf" | "ext" | "cfg" => Format::TextConfig,
        "sqf" | "sqs" | "fsm" => Format::Script,
        "sqm" => Format::Mission,
        "csv" | "xml" => Format::Stringtable,
        "txt" | "html" | "htm" | "lip" | "bikb" | "log" => Format::Text,
        _ => return None,
    })
}

fn format_for_signature(head: &[u8]) -> Option<Format> {
    let starts = |sig: &[u8]| head.starts_with(sig);
    Some(if starts(b"\0sreV") {
        Format::Pbo
    } else if head.len() >= 6 && &head[2..6] == b"GGAT" {
        Format::Paa
    } else if starts(b"MLOD") {
        Format::P3dMlod
    } else if starts(b"ODOL") {
        Format::P3dOdol
    } else if starts(b"OPRW") {
        Format::WrpOprw
    } else if starts(b"4WVR") || starts(b"8WVR") {
        Format::WrpEditable
    } else if starts(b"RTM_") {
        Format::RtmMlod
    } else if starts(b"BMTR") {
        Format::RtmBinarized
    } else if starts(crate::rap::MAGIC) {
        Format::RapConfig
    } else if starts(b"WSS0") {
        Format::Wss
    } else if starts(b"OggS") {
        Format::Ogg
    } else if starts(b"RIFF") && head.len() >= 12 && &head[8..12] == b"WAVE" {
        Format::Wav
    } else if starts(&[0xFF, 0xD8, 0xFF]) {
        Format::Jpeg
    } else if starts(b"\x89PNG") {
        Format::Png
    } else if starts(b"MZ") {
        Format::PeExecutable
    } else {
        return None;
    })
}

/// Extension-implied formats that a matching signature legitimately refines.
fn compatible(signature: Format, extension: Format) -> bool {
    matches!(
        (signature, extension),
        (Format::P3dMlod, Format::P3dOdol)
            | (Format::WrpEditable, Format::WrpOprw)
            | (Format::RtmMlod, Format::RtmBinarized)
            // A binarized .rvmat / .bisurf / .cpp is a rapified config.
            | (Format::RapConfig, Format::TextConfig)
            | (Format::RapConfig, Format::Mission)
    )
}

/// Formats that always carry a signature, so its absence is suspicious.
fn has_signature(format: Format) -> bool {
    matches!(
        format,
        Format::Paa
            | Format::P3dMlod
            | Format::P3dOdol
            | Format::WrpOprw
            | Format::RtmBinarized
            | Format::RapConfig
            | Format::Wss
            | Format::Ogg
            | Format::Wav
            | Format::Jpeg
            | Format::Png
            | Format::PeExecutable
    )
}

/// Heuristic: no NUL bytes and mostly printable (allowing Windows-1252 high bytes).
pub fn looks_like_text(head: &[u8]) -> bool {
    if head.contains(&0) {
        return false;
    }
    let printable = head.iter().filter(|&&b| b >= 0x20 || matches!(b, b'\t' | b'\n' | b'\r')).count();
    printable * 100 >= head.len() * 95
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_win_over_extensions() {
        let paa = [0x01, 0xFF, b'G', b'G', b'A', b'T', b'C', b'G', b'V', b'A'];
        let id = identify("tex_co.paa", &paa);
        assert_eq!((id.format, id.basis, id.conflict), (Format::Paa, Basis::Signature, false));
        assert_eq!(identify("model.p3d", b"MLOD\x01\x01").format, Format::P3dMlod);
        assert!(!identify("model.p3d", b"MLOD\x01\x01").conflict);
        assert_eq!(identify("x.wrp", b"OPRW\x12\0\0\0").format, Format::WrpOprw);
        assert_eq!(identify("a.rtm", b"BMTR\x03\0\0\0").format, Format::RtmBinarized);
        assert_eq!(identify("config.bin", b"\0raP\0\0\0\0").format, Format::RapConfig);
        assert!(!identify("m.rvmat", b"\0raP\0\0\0\0").conflict);
        assert_eq!(identify("s.wss", b"WSS0\x01\0").format, Format::Wss);
        assert_eq!(identify("dayz.pbo", b"\0sreV\0\0\0\0").format, Format::Pbo);
    }

    #[test]
    fn mismatches_are_flagged() {
        let id = identify("fake.paa", b"OggS\0\x02");
        assert_eq!(id.format, Format::Ogg);
        assert!(id.conflict);
        // Binary extension with no recognisable signature.
        let id = identify("broken.p3d", b"\x00\x01\x02\x03");
        assert_eq!(id.format, Format::P3dOdol);
        assert_eq!(id.basis, Basis::Extension);
        assert!(id.conflict);
    }

    #[test]
    fn text_and_unknown() {
        assert_eq!(identify("init.sqf", b"private [\"_a\"];").format, Format::Script);
        let id = identify("README", b"plain words here");
        assert_eq!((id.format, id.basis), (Format::Text, Basis::TextHeuristic));
        assert_eq!(identify("blob.xyz", &[0, 1, 2, 3, 0]).format, Format::Unknown);
        assert_eq!(identify("empty.xyz", b"").format, Format::Unknown);
    }

    #[test]
    fn extension_parsing() {
        assert_eq!(extension("a\\b\\C.PAA"), "paa");
        assert_eq!(extension("dir.d/noext"), "");
        assert_eq!(extension("archive.tar.gz"), "gz");
        assert_eq!(extension(".hidden"), "hidden");
    }
}
