//! PROJECT-CREATED synthetic fixtures.
//!
//! These builders produce files with the *container structure* of reference
//! formats and invented content, so the discovery pipeline can be tested
//! without any Bohemia or DayZ data. Model/texture/sound stand-ins carry only
//! the leading signature plus arbitrary bytes; they are not valid assets and
//! must never be described as reference data.

use crate::{lzss, pbo};
use sha1::{Digest, Sha1};

/// Builds a PBO archive in memory.
#[derive(Debug, Clone)]
pub struct PboBuilder {
    product_entry: bool,
    extensions: Vec<(String, String)>,
    files: Vec<(String, Vec<u8>, bool)>,
}

impl Default for PboBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PboBuilder {
    pub fn new() -> Self {
        Self { product_entry: true, extensions: Vec::new(), files: Vec::new() }
    }

    /// Omit the `Vers` product entry (older archive style).
    pub fn without_product_entry(mut self) -> Self {
        self.product_entry = false;
        self
    }

    pub fn extension(mut self, key: &str, value: &str) -> Self {
        self.extensions.push((key.to_owned(), value.to_owned()));
        self
    }

    pub fn file(mut self, name: &str, data: Vec<u8>) -> Self {
        self.files.push((name.to_owned(), data, false));
        self
    }

    /// Store the file LZSS-compressed (`Cprs`).
    pub fn compressed_file(mut self, name: &str, data: Vec<u8>) -> Self {
        self.files.push((name.to_owned(), data, true));
        self
    }

    pub fn build(&self) -> Vec<u8> {
        fn entry(out: &mut Vec<u8>, name: &str, fields: [u32; 5]) {
            out.extend_from_slice(name.as_bytes());
            out.push(0);
            for f in fields {
                out.extend_from_slice(&f.to_le_bytes());
            }
        }
        let mut out = Vec::new();
        if self.product_entry {
            entry(&mut out, "", [pbo::PACKING_VERSION, 0, 0, 0, 0]);
            for (k, v) in &self.extensions {
                out.extend_from_slice(k.as_bytes());
                out.push(0);
                out.extend_from_slice(v.as_bytes());
                out.push(0);
            }
            out.push(0);
        }
        let stored: Vec<Vec<u8>> = self
            .files
            .iter()
            .map(|(_, data, compress)| if *compress { lzss::compress(data) } else { data.clone() })
            .collect();
        for ((name, data, compress), stored) in self.files.iter().zip(&stored) {
            let (packing, original) = if *compress { (pbo::PACKING_COMPRESSED, data.len() as u32) } else { (0, 0) };
            entry(&mut out, name, [packing, original, 0, 0, stored.len() as u32]);
        }
        entry(&mut out, "", [0; 5]);
        for s in &stored {
            out.extend_from_slice(s);
        }
        let digest = Sha1::digest(&out);
        out.push(0);
        out.extend_from_slice(&digest);
        out
    }
}

/// Stand-in binarized model: `"ODOL"`, a version word, then the given
/// strings NUL-terminated (roughly how texture and material names appear in
/// real models, which is all the heuristic scanner relies on). The filler
/// before the first string is deliberately printable, as bytes in real
/// binaries can be, so the scanner's handling of glued-on prefixes is tested.
pub fn fake_odol(strings: &[&str]) -> Vec<u8> {
    let mut out = b"ODOL".to_vec();
    out.extend_from_slice(&7u32.to_le_bytes());
    out.extend_from_slice(&[0x5A; 12]);
    for s in strings {
        out.extend_from_slice(s.as_bytes());
        out.push(0);
        out.extend_from_slice(&[0x01, 0x02, 0x03]);
    }
    out
}

/// Stand-in texture: DXT1 type tag, one `"GGAT"` block, filler.
pub fn fake_paa() -> Vec<u8> {
    let mut out = vec![0x01, 0xFF];
    out.extend_from_slice(b"GGATCGVA");
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&[0x80, 0x80, 0x80, 0xFF]);
    out.extend_from_slice(&[0u8; 16]);
    out
}

/// Stand-in sound: `"WSS0"` and filler.
pub fn fake_wss() -> Vec<u8> {
    let mut out = b"WSS0".to_vec();
    out.extend_from_slice(&[0u8; 24]);
    out
}

/// Stand-in Windows executable: `"MZ"` and filler.
pub fn fake_pe() -> Vec<u8> {
    let mut out = b"MZ".to_vec();
    out.extend_from_slice(&[0x90; 62]);
    out
}
