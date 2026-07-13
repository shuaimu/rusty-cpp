#!/usr/bin/env bash
# Regenerate + compile the transpiled Rust **std** port as C++20 module
# `rusty` (std collides with C++'s std namespace — see the std-family roadmap
# in docs/port_regen/STATUS.md).
#
# First slice: std::collections::hash — HashMap/HashSet over a vendored
# hashbrown 0.16.1 (std's own pin) — plus std::hash (RandomState with
# fixed-seed stub, DefaultHasher over rusty::hash::SipHasher). The hashbrown
# dependency is a LOCAL PATH dep so the transpiler recursively transpiles it
# to a sibling `hashbrown` module (registry deps are not transpiled).
#
# Usage: build.sh <work_dir>
set -uo pipefail
W="${1:?usage: build.sh <work_dir>}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
SRC="$(rustc --print sysroot)/lib/rustlib/src/rust/library/std/src"
CORE="$(rustc --print sysroot)/lib/rustlib/src/rust/library/core/src"
rm -rf "$W"; mkdir -p "$W/src/collections/hash" "$W/src/hash"

# --- std sources (raw; prep.sh rewrites in place) ---
cp "$SRC/collections/hash/map.rs" "$W/src/collections/hash/map.rs"
cp "$SRC/collections/hash/set.rs" "$W/src/collections/hash/set.rs"
cp "$SRC/hash/random.rs" "$W/src/hash/random.rs"
printf 'pub mod map;\npub mod set;\n' > "$W/src/collections/hash/mod.rs"
printf 'pub mod hash;\npub use hash::map::HashMap;\npub use hash::set::HashSet;\n' > "$W/src/collections/mod.rs"
printf 'pub mod random;\npub use random::{DefaultHasher, RandomState};\n' > "$W/src/hash/mod.rs"
printf '#![allow(unused)]\n#![allow(deprecated)]\npub mod collections;\npub mod error;\npub mod hash;\npub mod io;\n' > "$W/src/lib.rs"

# --- std::error slice: std's error.rs is only Report+Indented over a
#     `pub use core::error::*` — the Error TRAIT lives in core::error.
#     Merge both files into ONE src/error.rs so trait/Report/Source stay
#     intra-module (no self-referential `pub use std::error::Error` after
#     the core::->std:: prep rewrite). Inner attrs (#![doc=include_str!],
#     #![stable]) stripped — invalid outside the crate root. ---
{ grep -v '^#!\[' "$CORE/error.rs"; printf '\n// ==== std::error (Report) — appended by build.sh ====\n'; grep -v '^#!\[' "$SRC/error.rs"; } > "$W/src/error.rs"

# --- io-cursor slice: cursor.rs + trimmed impls.rs from std, hand facades ---
mkdir -p "$W/src/io"
cp "$SRC/io/cursor.rs" "$W/src/io/cursor.rs"
cp "$SRC/io/impls.rs" "$W/src/io/impls.rs"

# error.rs: PREP STUB. The real std/src/io/error.rs is sys-tangled
# (crate::sys::decode_error_kind, RawOsError, Box<dyn error::Error> customs,
# repr_bitpacked tagged-pointer packing) — architectural OS boundary; the
# pure Cursor surface only needs kind+static-message consts.
cat > "$W/src/io/error.rs" <<'EOF'
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    InvalidInput,
    InvalidData,
    WriteZero,
    UnexpectedEof,
    OutOfMemory,
    Other,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: &'static str,
}

impl Error {
    pub(crate) const READ_EXACT_EOF: Error =
        Error::new_const(ErrorKind::UnexpectedEof, "failed to fill whole buffer");
    pub(crate) const WRITE_ALL_EOF: Error =
        Error::new_const(ErrorKind::WriteZero, "failed to write whole buffer");
    pub(crate) const INVALID_UTF8: Error =
        Error::new_const(ErrorKind::InvalidData, "stream did not contain valid UTF-8");

    pub const fn new_const(kind: ErrorKind, message: &'static str) -> Error {
        Error { kind, message }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &'static str {
        self.message
    }
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! const_error {
    ($kind:expr, $message:expr $(,)?) => {
        crate::io::error::Error::new_const($kind, $message)
    };
}
pub(crate) use const_error;
EOF

# mod.rs: PREP FACADE. SeekFrom + the trimmed Read/Write/Seek/BufRead trait
# surface Cursor needs (all methods required — every kept impl provides
# them, so no trait-default machinery). BorrowedCursor/IoSlice{,Mut} are
# sys::io-backed and out of the pure slice.
cat > "$W/src/io/mod.rs" <<'EOF'
mod cursor;
pub mod error;
mod impls;

pub use cursor::Cursor;
pub use error::{Error, ErrorKind, Result};
pub(crate) use error::const_error;

pub mod prelude {
    pub use super::{BufRead, Read, Seek, Write};
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
    fn stream_len(&mut self) -> Result<u64>;
    fn stream_position(&mut self) -> Result<u64>;
}

pub trait BufRead: Read {
    fn fill_buf(&mut self) -> Result<&[u8]>;
    fn consume(&mut self, amt: usize);
}
EOF

# --- vendored hashbrown 0.16.1 (registry cache, fallback: git tag) ---
HB_CACHE="$(ls -d "$HOME"/.cargo/registry/src/*/hashbrown-0.16.1 2>/dev/null | head -1)"
if [[ -n "$HB_CACHE" ]]; then
  cp -r "$HB_CACHE" "$W/hashbrown"
else
  git clone --depth 1 --branch v0.16.1 https://github.com/rust-lang/hashbrown.git "$W/hashbrown" \
    || { echo "hashbrown 0.16.1 unavailable (no registry cache, clone failed)"; exit 1; }
fi
rm -rf "$W/hashbrown/tests" "$W/hashbrown/benches" "$W/hashbrown/.git" 2>/dev/null || true
# Trimmed manifest: no foldhash/allocator deps; rustc-internal-api gives std's
# RustcEntry surface; [workspace] keeps cargo-expand standalone.
cat > "$W/hashbrown/Cargo.toml" <<EOF
[package]
edition = "2021"
name = "hashbrown"
version = "0.16.1"
[lib]
name = "hashbrown"
path = "src/lib.rs"
[features]
default = ["rustc-internal-api"]
inline-more = []
raw-entry = []
rustc-internal-api = []
[workspace]
EOF

cat > "$W/Cargo.toml" <<EOF
[package]
name = "rusty"
version = "0.0.1"
edition = "2021"
[lib]
path = "src/lib.rs"
[dependencies]
hashbrown = { path = "./hashbrown", default-features = false, features = ["rustc-internal-api"] }
# Empty [workspace] (+ exclude the vendored dep) so cargo-expand treats this
# as standalone even inside the repo tree (same gotcha as docs/alloc).
[workspace]
exclude = ["hashbrown"]
EOF

bash "$HERE/prep.sh" "$W/src" >/dev/null
TRANSPILER="${RUSTY_CPP_TRANSPILER_BIN:-$REPO/target/release/rusty-cpp-transpiler}"
"$TRANSPILER" --crate "$W/Cargo.toml" --expand --output-dir "$W/out" > "$W/transpile.log" 2>&1
echo "transpile exit=$? ($(tail -1 "$W/transpile.log"))"
[[ -f "$W/out/rusty.cppm" ]] || { echo "no rusty.cppm — see $W/transpile.log"; exit 1; }
[[ -f "$W/out/hashbrown/hashbrown.cppm" ]] || { echo "no hashbrown.cppm (dep not transpiled?)"; exit 1; }

python3 "$HERE/post_transpile_patch.py" "$W/out" || exit 1

FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include -x c++-module"
clang++ $FLAGS --precompile -o "$W/out/hashbrown/hashbrown.pcm" \
  "$W/out/hashbrown/hashbrown.cppm" -ferror-limit=0 2> "$W/hb_compile.err"
echo "hashbrown compile: $(grep -c ' error: ' "$W/hb_compile.err") errors"
clang++ $FLAGS --precompile -fmodule-file=hashbrown="$W/out/hashbrown/hashbrown.pcm" \
  -o "$W/out/rusty.pcm" "$W/out/rusty.cppm" -ferror-limit=0 2> "$W/compile.err"
# NOTE: match ' error: ' — the io slice's type paths contain the substring
# "error:" (io::error::Error) which inflates a bare 'error:' count via notes.
echo "rusty compile: $(grep -c ' error: ' "$W/compile.err") errors"
grep -hoE " error: .*" "$W/compile.err" "$W/hb_compile.err" 2>/dev/null \
  | sed -E "s/'[^']*'/'X'/g; s/[0-9]+/N/g" | sort | uniq -c | sort -rn | head -8
