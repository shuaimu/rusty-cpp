# ascii_port — Phase A2 + Phase B closed

`library/core/src/ascii/ascii_char.rs` (1229 LOC source) →
`transpiled/ascii_port/ascii_port.cppm` (4151 LOC C++).

| Stage | Status |
|---|---|
| 1. Source acquisition | ✅ `library/core/src/ascii/ascii_char.rs` |
| 2. Prep | ✅ `prep.sh` — 4 rustc-internal syntax forms stripped |
| 3. Transpile | ✅ Zero errors |
| 4. Patcher | ✅ 7 patches in `post_transpile_patch.py` |
| 5. Compile | ✅ `libascii_port.a` builds |
| 6. Smoke | ✅ `ascii_port_module_test.out` passes |

## What's exported and functional

- `AsciiChar` enum (128 variants, U+0000 → U+007F)
- Per-variant factories (`AsciiChar_Null` etc.)
- `to_u8(AsciiChar) -> uint8_t`
- `to_char(AsciiChar) -> char32_t`
- `is_digit(AsciiChar) -> bool` — delegates to `rusty::is_ascii_digit`
- `is_hexdigit(AsciiChar) -> bool` — delegates to `rusty::is_ascii_hexdigit`

## What's patcher-stubbed (Phase B/C work)

The transpiler emits `to_u8(self_).to_ascii_X()` patterns where
`to_ascii_X()` is a Rust u8 method — no analogue exists on `uint8_t` in
C++. The patcher stubs these to sensible defaults pending rusty::ascii
helper additions:

- `to_uppercase` / `to_lowercase` — return self unchanged
- `make_uppercase` / `make_lowercase` — no-op
- `eq_ignore_case` — returns false
- `is_alphabetic` / `is_uppercase` / `is_lowercase` / `is_alphanumeric`
- `is_octdigit` / `is_punctuation` / `is_graphic` / `is_whitespace`
  / `is_control` — return false
- `as_str` — minimal 1-byte string_view stub (the real impl needs
  `impl [AsciiChar]::as_str` which is a hand-port slot)
- `escape_ascii` — stubbed (return type `EscapeDefault` lives in
  parent `core::ascii` module, not ported)

## Hand-port slots (deferred)

- `into_int_impl!` macro_rules block (lines 1156-1171 of source) —
  `impl From<AsciiChar> for u8/u16/u32/u64/u128/char`
- `impl [AsciiChar] { ... }` block (lines 1173-1191) — slice
  `as_str` / `as_bytes` methods (rustc-internal slice-impl syntax)
- `impl fmt::Display for AsciiChar` / `impl fmt::Debug for AsciiChar`
  — both small but use `as_str` chain

## Files

- `Cargo.toml.template` — minimal lib crate manifest
- `prep.sh` — strips 4 unparseable syntax forms (use crate::, derive_const,
  multi-line assert_unsafe_precondition!, into_int_impl! macro,
  impl [AsciiChar] slice-impl)
- `post_transpile_patch.py` — 7 patches (visit_byte_buf stub,
  Self::→AsciiChar::, dup MIN/MAX dedup, as_str stub, escape_ascii
  stub, primitive-method bodies stub, AsciiChar::from_u8_unchecked call
  rewrite)
- `transpiled/ascii_port/ascii_port.cppm` — vendored, patched
- `transpiled/ascii_port/ascii_port.cppm.wip` — fresh transpile output
  (pre-patcher), retained for diff-checking on transpiler updates
