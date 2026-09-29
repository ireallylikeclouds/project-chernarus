# LZSS (compressed PBO entries)

Status: **ESTIMATED**. Implemented in `crates/formats/src/lzss.rs`.

- A flag byte precedes each group of eight tokens, consumed least-significant bit first. A set bit is
  a literal byte; a clear bit is a two-byte back-reference.
- Back-reference `b1 b2`: distance `b1 | (b2 & 0xF0) << 4` (12 bits), length `(b2 & 0x0F) + 3`
  (3–18 bytes). Copying goes byte by byte, so overlapping references repeat data.
- Window: a 4096-byte ring. Bytes 0–4077 start as spaces and 4078–4095 as zero; writing starts
  at 4078. A distance reaching before the start of the output reads those initial bytes.
- A 32-bit little-endian additive checksum follows the data. Open-source readers differ on whether
  bytes are summed as signed or unsigned; the reader accepts both and records which matched
  (`lzss_checksum` in the catalog). Which one the reference archives use is **UNKNOWN**, and the
  first real scan will answer it.

Tests: hand-built streams (overlapping copy, reference before the start of output, reference into
the zero-initialised tail, signed checksum), corruption and truncation errors, and round trips
through the project's own encoder.
