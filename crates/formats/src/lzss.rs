//! LZSS variant used for compressed PBO entries (packing method `Cprs`).
//!
//! Status: ESTIMATED. Behaviour follows the public BI community description
//! and was cross-checked against the MIT-licensed `bis-file-formats` reader
//! (see `docs/reference/formats/lzss.md`). It has only been exercised against
//! streams produced by this crate and hand-built vectors, not against a
//! reference PBO.
//!
//! Stream layout:
//! * A flag byte precedes each group of up to eight tokens; bits are consumed
//!   least significant first. A set bit is a literal byte; a clear bit is a
//!   two-byte back-reference.
//! * Back-reference `b1 b2`: distance = `b1 | (b2 & 0xF0) << 4` (12 bits),
//!   length = `(b2 & 0x0F) + 3`. Copying is byte by byte, so a reference may
//!   overlap the bytes it produces.
//! * The window is a 4096-byte ring whose first 4078 bytes start as spaces
//!   and whose last 18 start as zero; writing starts at 4078. A distance that
//!   reaches before the start of the output therefore yields those bytes.
//! * After the data comes a 32-bit little-endian additive checksum of the
//!   output. Readers disagree on whether bytes are summed signed or unsigned,
//!   so both are accepted and the matching one is reported.

use crate::FormatError;
use serde::{Deserialize, Serialize};

const N: usize = 4096;
const F: usize = 18;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 0x0F + MIN_MATCH;

/// Which checksum interpretation matched the stored value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecksumKind {
    /// Sum of bytes as unsigned values.
    Unsigned,
    /// Sum of bytes as signed (`i8`) values.
    Signed,
    /// Every byte was below 0x80, so both interpretations agree.
    Ambiguous,
}

/// Result of a full decompression.
#[derive(Debug, Clone)]
pub struct Decompressed {
    pub data: Vec<u8>,
    /// Bytes of input consumed, including the 4-byte checksum.
    pub consumed: usize,
    pub checksum: ChecksumKind,
}

struct Ring {
    buf: [u8; N],
    r: usize,
}

impl Ring {
    fn new() -> Self {
        let mut buf = [0u8; N];
        buf[..N - F].fill(b' ');
        Self { buf, r: N - F }
    }

    fn push(&mut self, b: u8) {
        self.buf[self.r] = b;
        self.r = (self.r + 1) & (N - 1);
    }
}

/// Decode tokens until `limit` bytes are produced (or `expected_len`, if
/// smaller). Returns the output and the input position after the last token.
fn decode(input: &[u8], expected_len: usize, limit: usize) -> Result<(Vec<u8>, usize), FormatError> {
    let target = expected_len.min(limit);
    let mut out = Vec::with_capacity(target);
    let mut ring = Ring::new();
    let mut pos = 0usize;
    let eof = |pos: usize| FormatError::UnexpectedEof { what: "LZSS stream", offset: pos as u64 };

    while out.len() < target {
        let flags = *input.get(pos).ok_or_else(|| eof(pos))?;
        pos += 1;
        for bit in 0..8 {
            if out.len() >= target {
                break;
            }
            if flags & (1 << bit) != 0 {
                let b = *input.get(pos).ok_or_else(|| eof(pos))?;
                pos += 1;
                out.push(b);
                ring.push(b);
            } else {
                let (b1, b2) = match (input.get(pos), input.get(pos + 1)) {
                    (Some(&b1), Some(&b2)) => (b1, b2),
                    _ => return Err(eof(pos)),
                };
                pos += 2;
                let distance = usize::from(b1) | (usize::from(b2 & 0xF0) << 4);
                let length = usize::from(b2 & 0x0F) + MIN_MATCH;
                if out.len() + length > expected_len {
                    return Err(FormatError::invalid(
                        "LZSS stream",
                        pos as u64,
                        format!("back-reference of {length} bytes overruns expected size {expected_len}"),
                    ));
                }
                let mut src = ring.r.wrapping_sub(distance) & (N - 1);
                for _ in 0..length {
                    let b = ring.buf[src];
                    src = (src + 1) & (N - 1);
                    if out.len() < target {
                        out.push(b);
                    }
                    ring.push(b);
                }
            }
        }
    }
    Ok((out, pos))
}

/// Decompress a complete stream of `expected_len` output bytes and verify its
/// trailing checksum.
pub fn decompress(input: &[u8], expected_len: usize) -> Result<Decompressed, FormatError> {
    let (data, pos) = decode(input, expected_len, expected_len)?;
    let stored = input
        .get(pos..pos + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(FormatError::UnexpectedEof { what: "LZSS checksum", offset: pos as u64 })?;
    let unsigned = data.iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b)));
    let signed = data.iter().fold(0i32, |s, &b| s.wrapping_add(i32::from(b as i8))) as u32;
    let checksum = match (stored == unsigned, stored == signed) {
        (true, true) => ChecksumKind::Ambiguous,
        (true, false) => ChecksumKind::Unsigned,
        (false, true) => ChecksumKind::Signed,
        (false, false) => {
            return Err(FormatError::invalid(
                "LZSS checksum",
                pos as u64,
                format!("stored {stored:#010x}, computed unsigned {unsigned:#010x} / signed {signed:#010x}"),
            ));
        }
    };
    Ok(Decompressed { data, consumed: pos + 4, checksum })
}

/// Decompress only the first `limit` bytes (for format identification).
/// The checksum is not verified because the stream is not read to its end.
pub fn decompress_prefix(input: &[u8], expected_len: usize, limit: usize) -> Result<Vec<u8>, FormatError> {
    decode(input, expected_len, limit).map(|(data, _)| data)
}

/// Greedy compressor producing a stream [`decompress`] accepts, with an
/// unsigned checksum. Used to build synthetic fixtures; it makes no attempt
/// to match the reference packer's output byte for byte.
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 8 + 5);
    let mut pos = 0usize;
    while pos < data.len() {
        let flag_at = out.len();
        out.push(0u8);
        let mut flags = 0u8;
        for bit in 0..8 {
            if pos >= data.len() {
                break;
            }
            let (distance, length) = longest_match(data, pos);
            if length >= MIN_MATCH {
                out.push((distance & 0xFF) as u8);
                out.push((((distance >> 4) & 0xF0) | (length - MIN_MATCH)) as u8);
                pos += length;
            } else {
                flags |= 1 << bit;
                out.push(data[pos]);
                pos += 1;
            }
        }
        out[flag_at] = flags;
    }
    let sum = data.iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b)));
    out.extend_from_slice(&sum.to_le_bytes());
    out
}

/// Longest match for `data[pos..]` within the previous `N - 1` bytes of real
/// output (the space-filled prefix is never used by the compressor).
fn longest_match(data: &[u8], pos: usize) -> (usize, usize) {
    let max_len = MAX_MATCH.min(data.len() - pos);
    let mut best = (0, 0);
    if max_len < MIN_MATCH {
        return best;
    }
    for distance in 1..=pos.min(N - 1) {
        let start = pos - distance;
        let len = (0..max_len).take_while(|&k| data[start + k] == data[pos + k]).count();
        if len > best.1 {
            best = (distance, len);
            if len == max_len {
                break;
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_checksum(mut stream: Vec<u8>, expected: &[u8]) -> Vec<u8> {
        let sum = expected.iter().fold(0u32, |s, &b| s.wrapping_add(u32::from(b)));
        stream.extend_from_slice(&sum.to_le_bytes());
        stream
    }

    #[test]
    fn hand_built_overlapping_back_reference() {
        // Literals "abc", then distance 3 length 6: flags 0b0000_0111.
        let stream = with_checksum(vec![0x07, b'a', b'b', b'c', 0x03, 0x03], b"abcabcabc");
        let d = decompress(&stream, 9).expect("valid stream");
        assert_eq!(d.data, b"abcabcabc");
        assert_eq!(d.consumed, stream.len());
        assert_eq!(d.checksum, ChecksumKind::Ambiguous);
    }

    #[test]
    fn reference_before_start_reads_initial_spaces() {
        // First token is a back-reference (flag bit clear), distance 1, length 3.
        let stream = with_checksum(vec![0x00, 0x01, 0x00], b"   ");
        assert_eq!(decompress(&stream, 3).expect("valid").data, b"   ");
    }

    #[test]
    fn reference_into_uninitialised_tail_reads_zero() {
        // Ring index (4078 - 4095) & 4095 = 4079 lies in the zero-initialised tail.
        let stream = with_checksum(vec![0x00, 0xFF, 0xF0], &[0, 0, 0]);
        assert_eq!(decompress(&stream, 3).expect("valid").data, [0, 0, 0]);
    }

    #[test]
    fn signed_checksum_is_accepted_and_reported() {
        let data = [0xFFu8, 0x80, 0x01];
        let signed = data.iter().fold(0i32, |s, &b| s.wrapping_add(i32::from(b as i8))) as u32;
        let mut stream = vec![0x07, 0xFF, 0x80, 0x01];
        stream.extend_from_slice(&signed.to_le_bytes());
        let d = decompress(&stream, 3).expect("valid");
        assert_eq!(d.checksum, ChecksumKind::Signed);
    }

    #[test]
    fn bad_checksum_is_rejected() {
        let mut stream = compress(b"hello hello hello");
        let last = stream.len() - 1;
        stream[last] ^= 0x55;
        assert!(decompress(&stream, 17).is_err());
    }

    #[test]
    fn overrun_and_truncation_are_errors_not_panics() {
        assert!(decompress(&[0x00, 0x01, 0x0F], 3).is_err());
        assert!(decompress(&[0x01], 1).is_err());
        assert!(decompress(&[], 1).is_err());
        assert!(decompress(&[0x01, b'a'], 1).is_err()); // missing checksum
    }

    #[test]
    fn round_trip_varied_inputs() {
        let mut inputs: Vec<Vec<u8>> = vec![
            Vec::new(),
            b"a".to_vec(),
            b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_vec(),
            b"class CfgPatches { class A { units[] = {}; }; };".repeat(40),
        ];
        // Deterministic pseudo-random bytes (xorshift) with some repetition.
        let mut x: u32 = 0x9E37_79B9;
        let mut noisy = Vec::new();
        for i in 0..20_000 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            noisy.push(if i % 7 < 3 { (x & 0xFF) as u8 } else { b"ODOL"[i % 4] });
        }
        inputs.push(noisy);
        for input in inputs {
            let packed = compress(&input);
            let d = decompress(&packed, input.len()).expect("round trip");
            assert_eq!(d.data, input);
            assert_eq!(d.consumed, packed.len());
        }
    }

    #[test]
    fn prefix_decoding_stops_early() {
        let input = b"0123456789".repeat(100);
        let packed = compress(&input);
        let prefix = decompress_prefix(&packed, input.len(), 16).expect("valid");
        assert_eq!(prefix, &input[..16]);
    }
}
