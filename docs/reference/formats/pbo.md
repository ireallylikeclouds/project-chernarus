# PBO archives

Status: **ESTIMATED**. Implemented in `crates/formats/src/pbo.rs` and tested against synthetic
archives. Never yet run on a reference PBO.

## Layout

```text
header entries (repeated):
  asciiz  name
  u32     packing method   0 = stored, "Cprs" = LZSS, "Encr" = encrypted, "Vers" = product entry
  u32     original size    (decompressed size for "Cprs")
  u32     reserved
  u32     timestamp        (Unix)
  u32     data size        (bytes stored in the archive)

product entry (optional, first): empty name + "Vers", then asciiz key/value pairs
  ("prefix", "product", "version", …) ended by an empty key
terminator: entry with empty name and packing 0
data blocks: in header order, `data size` bytes each
trailer: 0x00, then SHA-1 (20 bytes) of every byte before the 0x00
```

All integers little-endian; the four-character constants are ASCII read as little-endian u32.

## Engine rules the scanner assumes (ESTIMATED)

- Entries are mounted at `<prefix>\<entry name>`. Lookups are case-insensitive, with backslash
  separators and an optional leading backslash.
- Archives without a `prefix` use their file stem as the prefix.
- When several archives provide the same path, the engine's choice depends on load order (UNKNOWN).
  The catalog records every provider.

## Verification plan

Scan the pinned installation with checksum verification on. Expected: every archive has a product
entry with a prefix, a valid SHA-1 trailer, and only stored or `Cprs` entries. Anything else is
recorded here.
