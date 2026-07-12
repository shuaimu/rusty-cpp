#!/usr/bin/env python3
"""io-cursor slice surgery (prep step).

cursor.rs: drop the sys-adjacent surface (BorrowedCursor read_buf*, IoSlice
*vectored*), the Vec-backed Write impls (rusty::Vec is module-only, not
reachable from the standalone `rusty` std-port module), Box<[u8]> impl, and
the unsafe vec write helpers.

impls.rs: EXTRACT only `impl Read for &[u8]`, `impl BufRead for &[u8]`,
`impl Write for &mut [u8]` (the impls Cursor delegates to), then drop the
same sys-adjacent methods within them.
"""
import re
import sys
from pathlib import Path


def find_block_end(lines, i):
    depth, seen, j = 0, False, i
    while j < len(lines):
        for ch in lines[j]:
            if ch == "{":
                depth += 1
                seen = True
            elif ch == "}":
                depth -= 1
        j += 1
        if seen and depth == 0:
            return j
    raise SystemExit(f"unbalanced block from line {i}")


def walkback(lines, i):
    while i > 0:
        s = lines[i - 1].strip()
        if s.startswith("#[") or s.startswith("///") or s.startswith("//"):
            i -= 1
        else:
            break
    return i


def drop_blocks(text, starters):
    lines = text.splitlines(keepends=True)
    drops = []
    i = 0
    while i < len(lines):
        hit = None
        for pat in starters:
            if re.search(pat, lines[i]):
                hit = pat
                break
        if hit:
            start = walkback(lines, i)
            end = find_block_end(lines, i)
            drops.append((start, end, hit))
            i = end
        else:
            i += 1
    keep = []
    di = 0
    i = 0
    while i < len(lines):
        if di < len(drops) and i == drops[di][0]:
            i = drops[di][1]
            di += 1
        else:
            keep.append(lines[i])
            i += 1
    return "".join(keep), [d[2] for d in drops]


def extract_blocks(text, starters):
    lines = text.splitlines(keepends=True)
    out = []
    for pat in starters:
        for i, ln in enumerate(lines):
            if re.search(pat, ln):
                end = find_block_end(lines, i)
                out.append("".join(lines[i:end]))
                break
        else:
            raise SystemExit(f"extract: no match for {pat}")
    return "\n".join(out)


METHOD_DROPS = [
    r"^\s*fn read_buf\(",
    r"^\s*fn read_buf_exact\(",
    r"^\s*fn read_vectored\(",
    r"^\s*fn is_read_vectored\(",
    r"^\s*fn write_vectored\(",
    r"^\s*fn is_write_vectored\(",
    r"^\s*fn write_all_vectored\(",
    r"^\s*fn read_to_end\(",
    r"^\s*fn read_to_string\(",
]

CURSOR_DROPS = [
    # whole impls first (their inner fns are skipped wholesale)
    r"^impl<A> Write for Cursor<&mut Vec<u8, A>>",
    r"^impl<A> Write for Cursor<Vec<u8, A>>",
    r"^impl<A> Write for Cursor<Box<\[u8\], A>>",
    # unsafe Vec write helpers (dead once the Vec impls are gone)
    r"^\s*fn reserve_and_pad",
    r"^\s*unsafe fn vec_write_all_unchecked",
    r"^\s*fn vec_write_all\b",
    r"^\s*fn vec_write_all_vectored",
    r"^\s*fn slice_write_vectored",
    r"^\s*fn slice_write_all_vectored",
] + METHOD_DROPS

IMPLS_KEEP = [
    r"^impl Read for &\[u8\] \{",
    r"^impl BufRead for &\[u8\] \{",
    r"^impl Write for &mut \[u8\] \{",
]

IMPLS_HEADER = """\
// Trimmed extraction of std/src/io/impls.rs: only the in-memory slice
// impls Cursor delegates to. Generic forwarding impls (&mut R, Box<R>),
// VecDeque, String and Vec impls skipped (Vec is module-only in the
// runtime; forwarding impls need read_buf/IoSlice surface).
use crate::io::{self, BufRead, Read, Write};
use std::cmp;
use std::mem;

"""


SEEK_MATCH_OLD = """        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.inner.as_ref().len() as u64, n),
            SeekFrom::Current(n) => (self.pos, n),
        };
"""

# The transpiler lowers `match` used as an rvalue into an IIFE; a match arm
# that early-RETURNS from the enclosing fn becomes a return from the tuple
# lambda (type mismatch). Hoist the diverging arm into `if let`.
SEEK_MATCH_NEW = """        if let SeekFrom::Start(n) = style {
            self.pos = n;
            return Ok(n);
        }
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => (n, 0),
            SeekFrom::End(n) => (self.inner.as_ref().len() as u64, n),
            SeekFrom::Current(n) => (self.pos, n),
        };
"""

# checked_add_signed / unsigned_abs have no rusty runtime lowering; spell the
# same arithmetic with checked_add / checked_sub (which do).
SEEK_ARITH_OLD = "        match base_pos.checked_add_signed(offset) {\n"
SEEK_ARITH_NEW = """        let __new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((u64::MAX - (offset as u64)) + 1)
        };
        match __new_pos {
"""


def main(io_dir: Path) -> None:
    cur = io_dir / "cursor.rs"
    t, dropped = drop_blocks(cur.read_text(), CURSOR_DROPS)
    assert SEEK_MATCH_OLD in t, "seek match shape drifted"
    t = t.replace(SEEK_MATCH_OLD, SEEK_MATCH_NEW)
    assert SEEK_ARITH_OLD in t, "seek arith shape drifted"
    t = t.replace(SEEK_ARITH_OLD, SEEK_ARITH_NEW)
    cur.write_text(t)
    print(f"cursor.rs: dropped {len(dropped)} blocks + rewrote seek (2 spots)")

    imp = io_dir / "impls.rs"
    t = extract_blocks(imp.read_text(), IMPLS_KEEP)
    t, dropped = drop_blocks(t, METHOD_DROPS)
    imp.write_text(IMPLS_HEADER + t)
    print(f"impls.rs: extracted {len(IMPLS_KEEP)} impls, dropped {len(dropped)} methods")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
