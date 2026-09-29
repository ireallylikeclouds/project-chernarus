//! Bounds-checked little-endian reading over byte slices and streams.

use crate::FormatError;
use std::io::Read;

/// Longest NUL-terminated string accepted anywhere. Real names are far
/// shorter; the limit stops a scan of a non-matching file from reading
/// megabytes looking for a terminator.
pub const MAX_ASCIIZ: usize = 4096;

/// Cursor over an in-memory byte slice.
#[derive(Debug, Clone)]
pub struct SliceReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> SliceReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn at(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub fn bytes(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], FormatError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.data.len())
            .ok_or(FormatError::UnexpectedEof { what, offset: self.pos as u64 })?;
        let out = &self.data[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    pub fn u8(&mut self, what: &'static str) -> Result<u8, FormatError> {
        Ok(self.bytes(1, what)?[0])
    }

    pub fn u32(&mut self, what: &'static str) -> Result<u32, FormatError> {
        let b = self.bytes(4, what)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self, what: &'static str) -> Result<i32, FormatError> {
        Ok(self.u32(what)? as i32)
    }

    pub fn i64(&mut self, what: &'static str) -> Result<i64, FormatError> {
        let b = self.bytes(8, what)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(i64::from_le_bytes(a))
    }

    pub fn f32(&mut self, what: &'static str) -> Result<f32, FormatError> {
        Ok(f32::from_bits(self.u32(what)?))
    }

    /// NUL-terminated string. Non-UTF-8 bytes (the engine uses the Windows
    /// ANSI code page) are replaced rather than rejected.
    pub fn asciiz(&mut self, what: &'static str) -> Result<String, FormatError> {
        let start = self.pos;
        let window = &self.data[start.min(self.data.len())..];
        let limit = window.len().min(MAX_ASCIIZ + 1);
        match window[..limit].iter().position(|&b| b == 0) {
            Some(len) => {
                self.pos = start + len + 1;
                Ok(String::from_utf8_lossy(&window[..len]).into_owned())
            }
            None if limit > MAX_ASCIIZ => Err(FormatError::LimitExceeded {
                what,
                detail: format!("string at offset {start} longer than {MAX_ASCIIZ} bytes"),
            }),
            None => Err(FormatError::UnexpectedEof { what, offset: start as u64 }),
        }
    }

    /// Variable-length unsigned integer used by rapified configs: 7 bits per
    /// byte, least significant group first, high bit set on all but the last
    /// byte (the same scheme as unsigned LEB128).
    pub fn compressed_u32(&mut self, what: &'static str) -> Result<u32, FormatError> {
        let start = self.pos as u64;
        let mut result: u32 = 0;
        for i in 0..5 {
            let b = self.u8(what)?;
            let bits = u32::from(b & 0x7f);
            if i == 4 && bits > 0x0f {
                return Err(FormatError::invalid(what, start, "compressed integer overflows u32"));
            }
            result |= bits << (7 * i);
            if b & 0x80 == 0 {
                return Ok(result);
            }
        }
        Err(FormatError::invalid(what, start, "compressed integer longer than 5 bytes"))
    }
}

/// Stream reader that tracks its absolute offset, for headers of files too
/// large to load (PBOs can be hundreds of megabytes).
#[derive(Debug)]
pub struct StreamReader<R> {
    inner: R,
    offset: u64,
}

impl<R: Read> StreamReader<R> {
    pub fn new(inner: R, offset: u64) -> Self {
        Self { inner, offset }
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    fn fill(&mut self, buf: &mut [u8], what: &'static str) -> Result<(), FormatError> {
        let at = self.offset;
        self.inner.read_exact(buf).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                FormatError::UnexpectedEof { what, offset: at }
            } else {
                FormatError::Io(e)
            }
        })?;
        self.offset += buf.len() as u64;
        Ok(())
    }

    pub fn u32(&mut self, what: &'static str) -> Result<u32, FormatError> {
        let mut b = [0u8; 4];
        self.fill(&mut b, what)?;
        Ok(u32::from_le_bytes(b))
    }

    pub fn asciiz(&mut self, what: &'static str) -> Result<String, FormatError> {
        let start = self.offset;
        let mut out = Vec::new();
        loop {
            let mut b = [0u8; 1];
            self.fill(&mut b, what)?;
            if b[0] == 0 {
                return Ok(String::from_utf8_lossy(&out).into_owned());
            }
            if out.len() >= MAX_ASCIIZ {
                return Err(FormatError::LimitExceeded {
                    what,
                    detail: format!("string at offset {start} longer than {MAX_ASCIIZ} bytes"),
                });
            }
            out.push(b[0]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compressed_u32_matches_leb128() {
        for (bytes, value) in [
            (&[0x00][..], 0u32),
            (&[0x7f][..], 127),
            (&[0x80, 0x01][..], 128),
            (&[0xe5, 0x8e, 0x26][..], 624_485),
            (&[0xff, 0xff, 0xff, 0xff, 0x0f][..], u32::MAX),
        ] {
            let mut r = SliceReader::new(bytes);
            assert_eq!(r.compressed_u32("t").expect("valid"), value);
            assert_eq!(r.remaining(), 0);
        }
    }

    #[test]
    fn compressed_u32_rejects_overlong() {
        let mut r = SliceReader::new(&[0xff, 0xff, 0xff, 0xff, 0x7f]);
        assert!(r.compressed_u32("t").is_err());
        let mut r = SliceReader::new(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x00]);
        assert!(r.compressed_u32("t").is_err());
    }

    #[test]
    fn asciiz_eof_and_limit() {
        assert!(SliceReader::new(b"abc").asciiz("t").is_err());
        let long = vec![b'a'; MAX_ASCIIZ + 10];
        assert!(matches!(SliceReader::new(&long).asciiz("t"), Err(FormatError::LimitExceeded { .. })));
        let mut r = SliceReader::new(b"ab\0cd\0");
        assert_eq!(r.asciiz("t").expect("valid"), "ab");
        assert_eq!(r.asciiz("t").expect("valid"), "cd");
    }

    #[test]
    fn slice_reader_never_panics_past_end() {
        let mut r = SliceReader::at(b"ab", 10);
        assert!(r.u8("t").is_err());
        assert!(r.asciiz("t").is_err());
    }
}
