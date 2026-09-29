//! PBO archives (the container every ARMA 2 / DayZ Mod addon ships in).
//!
//! Status: ESTIMATED — written from the public format description and
//! cross-checked against open-source readers (`docs/reference/formats/pbo.md`);
//! exercised only against synthetic archives so far.
//!
//! Layout:
//! ```text
//! entry*            asciiz name; u32 packing; u32 original_size;
//!                   u32 reserved; u32 timestamp; u32 data_size
//!   - first entry may be the product entry: empty name, packing "Vers",
//!     followed by asciiz key/value pairs ended by an empty key
//!   - an entry with an empty name (not "Vers") ends the list
//! data blocks       data_size bytes per entry, in header order
//! 0x00, sha1[20]    SHA-1 of everything before the 0x00 byte
//! ```

use crate::read::StreamReader;
use crate::{FormatError, lzss};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::io::{Read, Seek, SeekFrom};

/// `"Vers"` read as a little-endian u32.
pub const PACKING_VERSION: u32 = u32::from_le_bytes(*b"sreV");
/// `"Cprs"` read as a little-endian u32.
pub const PACKING_COMPRESSED: u32 = u32::from_le_bytes(*b"srpC");
/// `"Encr"` read as a little-endian u32.
pub const PACKING_ENCRYPTED: u32 = u32::from_le_bytes(*b"rcnE");

/// Sanity limits for scanning arbitrary files.
const MAX_ENTRIES: usize = 1_000_000;
const MAX_EXTENSIONS: usize = 4096;

/// How an entry's data is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Packing {
    Uncompressed,
    /// LZSS (`Cprs`); `original_size` is the decompressed size.
    Compressed,
    /// `Encr`; not supported.
    Encrypted,
    Other(u32),
}

impl Packing {
    fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Packing::Uncompressed,
            PACKING_COMPRESSED => Packing::Compressed,
            PACKING_ENCRYPTED => Packing::Encrypted,
            other => Packing::Other(other),
        }
    }
}

/// One file inside a PBO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PboEntry {
    /// Name as stored: relative path with backslash separators.
    pub name: String,
    pub packing: Packing,
    pub original_size: u32,
    pub reserved: u32,
    /// Unix timestamp as stored (often zero).
    pub timestamp: u32,
    /// Bytes stored in the archive (including the LZSS checksum if compressed).
    pub data_size: u32,
    /// Absolute offset of the entry's data in the archive.
    pub offset: u64,
}

impl PboEntry {
    /// Size of the logical (decompressed) content.
    pub fn content_size(&self) -> u64 {
        match self.packing {
            Packing::Compressed => u64::from(self.original_size),
            _ => u64::from(self.data_size),
        }
    }
}

/// Parsed header of a PBO. Entry data is read on demand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PboHeader {
    /// Product-entry key/value pairs in file order (`prefix`, `product`, ...).
    pub extensions: Vec<(String, String)>,
    pub has_product_entry: bool,
    pub entries: Vec<PboEntry>,
    /// Offset where the first data block starts.
    pub data_start: u64,
    /// Offset just past the last data block.
    pub data_end: u64,
}

impl PboHeader {
    /// The `prefix` extension: the virtual path the engine mounts the
    /// archive's contents under.
    pub fn prefix(&self) -> Option<&str> {
        self.extension("prefix")
    }

    /// Case-insensitive extension lookup.
    pub fn extension(&self, key: &str) -> Option<&str> {
        self.extensions.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }

    /// Read the header of an archive. `file_len` bounds the data blocks so
    /// a non-PBO file cannot claim more data than exists.
    pub fn read<R: Read>(reader: R, file_len: u64) -> Result<Self, FormatError> {
        let mut r = StreamReader::new(reader, 0);
        let mut extensions = Vec::new();
        let mut has_product_entry = false;
        let mut raw_entries = Vec::new();

        loop {
            let name = r.asciiz("PBO entry name")?;
            let packing = r.u32("PBO packing method")?;
            let original_size = r.u32("PBO original size")?;
            let reserved = r.u32("PBO reserved field")?;
            let timestamp = r.u32("PBO timestamp")?;
            let data_size = r.u32("PBO data size")?;

            if name.is_empty() && packing == PACKING_VERSION {
                if has_product_entry || !raw_entries.is_empty() {
                    return Err(FormatError::invalid(
                        "PBO header",
                        r.offset(),
                        "product entry must be the first entry",
                    ));
                }
                has_product_entry = true;
                loop {
                    let key = r.asciiz("PBO header extension key")?;
                    if key.is_empty() {
                        break;
                    }
                    let value = r.asciiz("PBO header extension value")?;
                    extensions.push((key, value));
                    if extensions.len() > MAX_EXTENSIONS {
                        return Err(FormatError::LimitExceeded {
                            what: "PBO header extensions",
                            detail: format!("more than {MAX_EXTENSIONS}"),
                        });
                    }
                }
                continue;
            }
            if name.is_empty() {
                break;
            }
            raw_entries.push((name, packing, original_size, reserved, timestamp, data_size));
            if raw_entries.len() > MAX_ENTRIES {
                return Err(FormatError::LimitExceeded {
                    what: "PBO entries",
                    detail: format!("more than {MAX_ENTRIES}"),
                });
            }
        }

        let data_start = r.offset();
        let mut offset = data_start;
        let mut entries = Vec::with_capacity(raw_entries.len());
        for (name, packing, original_size, reserved, timestamp, data_size) in raw_entries {
            entries.push(PboEntry {
                name,
                packing: Packing::from_raw(packing),
                original_size,
                reserved,
                timestamp,
                data_size,
                offset,
            });
            offset += u64::from(data_size);
        }
        if offset > file_len {
            return Err(FormatError::invalid(
                "PBO header",
                data_start,
                format!("entries claim {} bytes of data but file is {file_len} bytes", offset - data_start),
            ));
        }
        Ok(PboHeader { extensions, has_product_entry, entries, data_start, data_end: offset })
    }

    /// Read an entry's stored bytes (compressed entries stay compressed).
    pub fn read_raw<R: Read + Seek>(&self, file: &mut R, entry: &PboEntry) -> Result<Vec<u8>, FormatError> {
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut buf = vec![0u8; entry.data_size as usize];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Read an entry's logical content, decompressing and checksum-verifying
    /// compressed entries.
    pub fn read_entry<R: Read + Seek>(&self, file: &mut R, entry: &PboEntry) -> Result<EntryContent, FormatError> {
        let raw = self.read_raw(file, entry)?;
        decode_entry(entry, raw)
    }

    /// Read at most `limit` bytes of an entry's logical content, for format
    /// identification. Compressed entries are decoded only as far as needed.
    pub fn read_prefix<R: Read + Seek>(
        &self,
        file: &mut R,
        entry: &PboEntry,
        limit: usize,
    ) -> Result<Vec<u8>, FormatError> {
        match entry.packing {
            Packing::Uncompressed => {
                file.seek(SeekFrom::Start(entry.offset))?;
                let n = (entry.data_size as usize).min(limit);
                let mut buf = vec![0u8; n];
                file.read_exact(&mut buf)?;
                Ok(buf)
            }
            Packing::Compressed => {
                // Each output byte costs at most 9/8 input bytes, plus a flag byte.
                let need = (limit + limit / 8 + 2).min(entry.data_size as usize);
                file.seek(SeekFrom::Start(entry.offset))?;
                let mut buf = vec![0u8; need];
                file.read_exact(&mut buf)?;
                lzss::decompress_prefix(&buf, entry.original_size as usize, limit)
            }
            Packing::Encrypted | Packing::Other(_) => Err(unsupported_packing(entry)),
        }
    }

    /// Verify the trailing SHA-1. Streams the archive; does not load it.
    pub fn verify_checksum<R: Read + Seek>(&self, file: &mut R) -> Result<Checksum, FormatError> {
        file.seek(SeekFrom::Start(0))?;
        let mut hasher = Sha1::new();
        let mut remaining = self.data_end;
        let mut buf = vec![0u8; 1 << 16];
        while remaining > 0 {
            let n = (buf.len() as u64).min(remaining) as usize;
            file.read_exact(&mut buf[..n])?;
            hasher.update(&buf[..n]);
            remaining -= n as u64;
        }
        let mut trailer = Vec::with_capacity(21);
        file.take(21).read_to_end(&mut trailer)?;
        if trailer.is_empty() {
            return Ok(Checksum::Absent);
        }
        if trailer.len() != 21 || trailer[0] != 0 {
            return Ok(Checksum::Malformed);
        }
        let computed: [u8; 20] = hasher.finalize().into();
        Ok(if trailer[1..] == computed { Checksum::Valid } else { Checksum::Mismatch })
    }
}

/// Logical content of an entry.
#[derive(Debug, Clone)]
pub struct EntryContent {
    pub data: Vec<u8>,
    /// Checksum interpretation that matched, for compressed entries.
    pub lzss_checksum: Option<lzss::ChecksumKind>,
}

fn unsupported_packing(entry: &PboEntry) -> FormatError {
    FormatError::Unsupported {
        what: "PBO packing method",
        detail: format!("{:?} for entry {}", entry.packing, entry.name),
    }
}

fn decode_entry(entry: &PboEntry, raw: Vec<u8>) -> Result<EntryContent, FormatError> {
    match entry.packing {
        Packing::Uncompressed => Ok(EntryContent { data: raw, lzss_checksum: None }),
        Packing::Compressed => {
            let d = lzss::decompress(&raw, entry.original_size as usize)?;
            Ok(EntryContent { data: d.data, lzss_checksum: Some(d.checksum) })
        }
        Packing::Encrypted | Packing::Other(_) => Err(unsupported_packing(entry)),
    }
}

/// Outcome of trailing-checksum verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Checksum {
    Valid,
    Mismatch,
    /// No trailer (older archives, or truncated files).
    Absent,
    /// Trailer present but not `0x00` followed by 20 bytes.
    Malformed,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::PboBuilder;
    use std::io::Cursor;

    fn sample() -> Vec<u8> {
        PboBuilder::new()
            .extension("prefix", "synth\\test")
            .extension("product", "chernarus-fixture")
            .file("config.bin", b"\0raP not really".to_vec())
            .compressed_file("data\\readme.txt", b"hello hello hello hello hello".to_vec())
            .file("empty.txt", Vec::new())
            .build()
    }

    fn header(bytes: &[u8]) -> PboHeader {
        PboHeader::read(Cursor::new(bytes), bytes.len() as u64).expect("valid synthetic PBO")
    }

    #[test]
    fn reads_extensions_and_entries() {
        let bytes = sample();
        let h = header(&bytes);
        assert!(h.has_product_entry);
        assert_eq!(h.prefix(), Some("synth\\test"));
        assert_eq!(h.extension("PRODUCT"), Some("chernarus-fixture"));
        let names: Vec<_> = h.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["config.bin", "data\\readme.txt", "empty.txt"]);
        assert_eq!(h.entries[1].packing, Packing::Compressed);
        assert_eq!(h.entries[1].content_size(), 29);
    }

    #[test]
    fn reads_plain_and_compressed_content() {
        let bytes = sample();
        let h = header(&bytes);
        let mut f = Cursor::new(&bytes);
        let plain = h.read_entry(&mut f, &h.entries[0]).expect("plain");
        assert_eq!(plain.data, b"\0raP not really");
        assert_eq!(plain.lzss_checksum, None);
        let packed = h.read_entry(&mut f, &h.entries[1]).expect("compressed");
        assert_eq!(packed.data, b"hello hello hello hello hello");
        assert!(packed.lzss_checksum.is_some());
        let prefix = h.read_prefix(&mut f, &h.entries[1], 5).expect("prefix");
        assert_eq!(prefix, b"hello");
        assert!(h.read_entry(&mut f, &h.entries[2]).expect("empty").data.is_empty());
    }

    #[test]
    fn checksum_detects_corruption() {
        let mut bytes = sample();
        let h = header(&bytes);
        assert_eq!(h.verify_checksum(&mut Cursor::new(&bytes)).expect("io"), Checksum::Valid);
        let i = h.entries[0].offset as usize;
        bytes[i] ^= 0xFF;
        assert_eq!(h.verify_checksum(&mut Cursor::new(&bytes)).expect("io"), Checksum::Mismatch);
        let cut = h.data_end as usize;
        assert_eq!(h.verify_checksum(&mut Cursor::new(&bytes[..cut])).expect("io"), Checksum::Absent);
    }

    #[test]
    fn archive_without_product_entry() {
        let bytes = PboBuilder::new().without_product_entry().file("a.sqf", b"hint 1;".to_vec()).build();
        let h = header(&bytes);
        assert!(!h.has_product_entry);
        assert_eq!(h.prefix(), None);
        assert_eq!(h.entries.len(), 1);
    }

    #[test]
    fn rejects_non_pbo_input_without_panicking() {
        for junk in [&b""[..], b"MZ\x90\x00", b"\0raP\0\0\0\0\x08\0\0\0", &[0xFFu8; 64][..]] {
            assert!(PboHeader::read(Cursor::new(junk), junk.len() as u64).is_err());
        }
        // Header claiming more data than the file holds.
        let mut bytes = sample();
        bytes.truncate(bytes.len() - 40);
        assert!(PboHeader::read(Cursor::new(&bytes), bytes.len() as u64).is_err());
    }

    #[test]
    fn encrypted_entries_are_reported_unsupported() {
        let entry = PboEntry {
            name: "x".into(),
            packing: Packing::Encrypted,
            original_size: 0,
            reserved: 0,
            timestamp: 0,
            data_size: 0,
            offset: 0,
        };
        assert!(matches!(decode_entry(&entry, Vec::new()), Err(FormatError::Unsupported { .. })));
    }
}
