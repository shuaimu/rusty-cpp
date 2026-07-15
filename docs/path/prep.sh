#!/usr/bin/env bash
# Prep std's path.rs for a Unix-only value-semantics C++ port: strip the
# fs/io accessors, the make-absolute helper, try_reserve (TryReserveError), and
# the foreign/multi-target AsRef impls whose adapter emission collides. What
# remains is the lexical path-manipulation core (Path/PathBuf/Components/
# Ancestors/Display + the Prefix machinery, which is dead but compilable on
# Unix). Usage: prep.sh <path.rs>
set -uo pipefail
F="${1:?usage: prep.sh <path.rs>}"
python3 - "$F" <<'PYS'
import sys
p = sys.argv[1]
s = open(p).read()

def matching_close(text, b):
    """Index just past the `}` matching the `{` at index b, skipping Rust
    strings, raw strings, char/byte literals, and comments."""
    i, n, depth = b, len(text), 0
    while i < n:
        c = text[i]
        if c == '{':
            depth += 1; i += 1
        elif c == '}':
            depth -= 1; i += 1
            if depth == 0:
                return i
        elif c == '/' and i + 1 < n and text[i+1] == '/':
            j = text.find('\n', i);  i = n if j < 0 else j + 1
        elif c == '/' and i + 1 < n and text[i+1] == '*':
            j = text.find('*/', i+2); i = n if j < 0 else j + 2
        elif c == 'r' and i + 1 < n and text[i+1] in '"#':
            # raw string r"..." / r#"..."#
            k = i + 1; hashes = 0
            while k < n and text[k] == '#': hashes += 1; k += 1
            if k < n and text[k] == '"':
                close = '"' + '#' * hashes
                j = text.find(close, k+1); i = n if j < 0 else j + len(close)
            else:
                i += 1
        elif c == '"':
            j = i + 1
            while j < n:
                if text[j] == '\\': j += 2; continue
                if text[j] == '"': break
                j += 1
            i = j + 1
        elif c == "'":
            # char/byte literal 'x' or '\n' — but NOT a lifetime ('a). A char
            # literal has a closing quote within a few chars.
            j = i + 1
            if j < n and text[j] == '\\':
                j += 2
                while j < n and text[j] != "'": j += 1
                i = j + 1
            elif j + 1 < n and text[j+1] == "'":
                i = j + 2               # 'x'
            else:
                i += 1                  # lifetime tick
        else:
            i += 1
    return n

def cut_from(text, anchor):
    i = text.find(anchor)
    if i < 0:
        return text, False
    b = text.find('{', i)
    if b < 0:
        return text, False
    j = matching_close(text, b)
    start = text.rfind('\n', 0, i) + 1
    # Also consume the contiguous doc-comment / attribute / blank block directly
    # above the item, so it doesn't dangle onto the next item (or EOF).
    lines = text[:start].split('\n')
    while lines:
        stripped = lines[-1].lstrip()
        if stripped == '' or stripped.startswith('///') or stripped.startswith('//!') \
           or stripped.startswith('#[') or stripped.startswith('#![') \
           or stripped.startswith('//'):
            lines.pop()
        else:
            break
    head = '\n'.join(lines)
    if head:
        head += '\n'
    return head + text[j:], True

def drop(anchor, label):
    global s
    s, ok = cut_from(s, anchor)
    if not ok:
        sys.stderr.write(f"  WARN: anchor not found: {label}\n")

# fs/io accessors on `impl Path` — pure filesystem, not lexical.
for a in ("pub fn metadata(&self)", "pub fn symlink_metadata(&self)",
          "pub fn canonicalize(&self)", "pub fn read_link(&self)",
          "pub fn read_dir(&self)", "pub fn exists(&self)",
          "pub fn try_exists(&self)", "pub fn is_file(&self)",
          "pub fn is_dir(&self)", "pub fn is_symlink(&self)"):
    drop(a, a)

# try_reserve family pulls collections::TryReserveError.
drop("pub fn try_reserve(&mut self", "try_reserve")
drop("pub fn try_reserve_exact(&mut self", "try_reserve_exact")

# make-absolute pulls env::current_dir + io.
drop("pub fn absolute<P: AsRef<Path>>", "absolute")

# Foreign / multi-target AsRef impls: their <Trait>Adapter<U> specializations
# collide (keyed on impl type, not target) and inherit an undeclared AsRef base.
# The path-manipulation core needs only AsRef for Path/PathBuf.
for a in ("impl AsRef<Path> for Cow<'_, OsStr>", "impl AsRef<Path> for OsString",
          "impl AsRef<Path> for str", "impl AsRef<Path> for String",
          "impl AsRef<OsStr> for Component<'_>", "impl AsRef<Path> for Component<'_>",
          "impl AsRef<OsStr> for Components<'_>", "impl AsRef<Path> for Components<'_>",
          "impl AsRef<OsStr> for Iter<'_>", "impl AsRef<Path> for Iter<'_>"):
    drop(a, a)

open(p, "w").write(s)
print("  path.rs prep complete")
PYS
