# Rapified configs (raP)

Status: **ESTIMATED**. Implemented in `crates/formats/src/rap.rs`.

```text
header   "\0raP", u32 0, u32 8, u32 enum_offset
body     asciiz parent; compressed-int count; entries
entry    0 class       asciiz name; u32 absolute offset of the class body
         1 value       u8 subtype; asciiz name; value
                        (0 string, 1 f32, 2 i32, 4 "variable" string, 6 i64)
         2 array       asciiz name; array
         3 external    asciiz name        (class Name;)
         4 delete      asciiz name        (delete Name;)
         5 array +=    u32 flags; asciiz name; array
array    compressed-int count; per element u8 type (0 string, 1 f32, 2 i32, 3 nested array, 4 variable, 6 i64) + value
enums    at enum_offset: u32 count; (asciiz name, u32 value) × count
compressed int: 7 bits per byte, least significant group first, high bit = continue (LEB128)
```

Subtypes 4 and 6 are documented but not expected in ARMA 2 data (UNKNOWN until a scan).

## Not implemented (engine semantics)

- **Inheritance** (`class B: A`): values are inherited from the parent class, resolved through
  enclosing scopes. `Class::get` returns only entries declared directly.
- **Cross-addon merging**: addons patch each other's classes in an order derived from
  `CfgPatches.requiredAddons`. Effective values need this before they can be cited as
  reference data.

Both are prerequisites for extracting weapon, ammunition and movement data at PARTIALLY VERIFIED or better.
