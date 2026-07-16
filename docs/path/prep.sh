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

# Extension SETTERS use the raw-pointer `.addr()` truncate-to-offset trick
# (`slice[len..].as_ptr().addr() - self.as_ptr().addr()`), which the value port
# can't express (pointers into copied buffers are unrelated). Secondary to the
# core path API — strip for the first RUNTIME PASS. (extension() getter stays.)
for a in ("pub fn set_extension<S: AsRef<OsStr>>", "fn _set_extension(&mut self",
          "pub fn add_extension<S: AsRef<OsStr>>", "fn _add_extension(&mut self",
          "pub fn with_extension<S: AsRef<OsStr>>", "fn _with_extension(&self"):
    drop(a, a)
# normalize_lexically builds a Vec<Component> (emitted as std::vector, no
# push/truncate) and is a lexical-normalization extra — strip for now.
drop("pub fn normalize_lexically(&self)", "normalize_lexically")

# Foreign / multi-target AsRef impls: their <Trait>Adapter<U> specializations
# collide (keyed on impl type, not target) and inherit an undeclared AsRef base.
# The path-manipulation core needs only AsRef for Path/PathBuf.
for a in ("impl AsRef<Path> for Cow<'_, OsStr>", "impl AsRef<Path> for OsString",
          "impl AsRef<Path> for str", "impl AsRef<Path> for String",
          "impl AsRef<OsStr> for Component<'_>", "impl AsRef<Path> for Component<'_>",
          "impl AsRef<OsStr> for Components<'_>", "impl AsRef<Path> for Components<'_>",
          "impl AsRef<OsStr> for Iter<'_>", "impl AsRef<Path> for Iter<'_>"):
    drop(a, a)

# --- Prefix cascade strip (Unix-dead) --------------------------------------
# On Unix parse_prefix is always None, so the Component::Prefix variant is never
# constructed and State::Prefix never entered. Keeping the variant forces the
# emitted Component `operator==` to compare PrefixComponent (which itself
# compares Prefix), and the Prefix data-enum `==`/`<` need per-variant operators
# that don't exist. Drop the derives that emit those operators, remove the
# Component::Prefix variant + its construction/match sites, and drop
# PrefixComponent's comparison impls (they depend on the removed Prefix ==/ord).
def rep(old, new, label):
    global s
    if old not in s:
        sys.stderr.write(f"  WARN: replace anchor not found: {label}\n")
        return
    s = s.replace(old, new)

# Prefix / State derives → drop PartialEq/Eq/PartialOrd/Ord/Hash emission.
rep("#[derive(Copy, Clone, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]\n"
    "#[stable(feature = \"rust1\", since = \"1.0.0\")]\npub enum Prefix<'a> {",
    "#[derive(Copy, Clone, Debug)]\n"
    "#[stable(feature = \"rust1\", since = \"1.0.0\")]\npub enum Prefix<'a> {",
    "Prefix derive")

# Remove the Component::Prefix variant.
rep("    #[stable(feature = \"rust1\", since = \"1.0.0\")]\n"
    "    Prefix(#[stable(feature = \"rust1\", since = \"1.0.0\")] PrefixComponent<'a>),\n\n",
    "", "Component::Prefix variant")

# Component::as_os_str arm.
rep("            Component::Prefix(p) => p.as_os_str(),\n", "", "as_os_str Prefix arm")

# Components::next State::Prefix arms.
rep("""                State::Prefix if self.prefix_len() == 0 => {
                    self.front = State::StartDir;
                }
                State::Prefix => {
                    self.front = State::StartDir;
                    debug_assert!(self.prefix_len() <= self.path.len());
                    let raw = &self.path[..self.prefix_len()];
                    self.path = &self.path[self.prefix_len()..];
                    return Some(Component::Prefix(PrefixComponent {
                        raw: unsafe { OsStr::from_encoded_bytes_unchecked(raw) },
                        parsed: self.prefix.unwrap(),
                    }));
                }
""", "", "next State::Prefix arms")

# Components::next_back State::Prefix arms.
rep("""                State::Prefix if self.prefix_len() > 0 => {
                    self.back = State::Done;
                    return Some(Component::Prefix(PrefixComponent {
                        raw: unsafe { OsStr::from_encoded_bytes_unchecked(self.path) },
                        parsed: self.prefix.unwrap(),
                    }));
                }
                State::Prefix => {
                    self.back = State::Done;
                    return None;
                }
""", "", "next_back State::Prefix arms")

# PathBuf::_push need_sep Component::Prefix arm.
rep("""                    Component::Prefix(prefix) => {
                        !prefix.parsed.is_drive() && prefix.parsed.len() > 0
                    }
""", "", "_push Prefix arm")

# (normalize_lexically's Component::Prefix arms are removed with the whole fn,
# which is stripped above.)

# PrefixComponent comparison impls depend on the removed Prefix ==/ord/hash.
for a in ("impl<'a> PartialEq for PrefixComponent<'a>",
          "impl<'a> PartialOrd for PrefixComponent<'a>",
          "impl Ord for PrefixComponent<'_>",
          "impl Hash for PrefixComponent<'_>"):
    drop(a, a)

# --- Auxiliary trait impls not needed for RUNTIME-PASS of path manipulation.
# These pull unresolved std namespaces (::hash::, ::borrow::Borrow,
# ::iter::FusedIterator, ::error::Error) and the Ord/PartialOrd impls return
# cmp::Ordering. Keep PartialEq (bool `==`, no Ordering, used by tests).
for a in ("impl FusedIterator for Iter<'_>",
          "impl FusedIterator for Components<'_>",
          "impl FusedIterator for Ancestors<'_>",
          "impl Eq for Components<'_>",
          "impl Ord for Components<'_>",
          "impl Borrow<Path> for PathBuf",
          "impl Hash for PathBuf",
          "impl Eq for PathBuf",
          "impl PartialOrd for PathBuf",
          "impl Ord for PathBuf",
          "impl Hash for Path",
          "impl Eq for Path",
          "impl PartialOrd for Path",
          "impl Ord for Path",
          "impl Error for StripPrefixError",
          "impl Error for NormalizeError",
          # compare_components (byte ordering → cmp::Ordering) is now called only
          # by Components' PartialOrd (Path/PathBuf Ord/PartialOrd stripped above).
          "impl<'a> PartialOrd for Components<'a>",
          "fn compare_components(mut left"):
    drop(a, a)
# Path::display + the Display wrapper depend on os_str::Display (no Unix runtime).
# display() is presentation-only; strip for the first RUNTIME PASS.
for a in ("pub fn display(&self)", "pub struct Display<'a>",
          "impl fmt::Debug for Display<'_>", "impl fmt::Display for Display<'_>"):
    drop(a, a)

# with_added_extension calls the stripped add_extension. Debug impls each emit a
# namespace-scope DebugHelper struct that collides (redefinition); Debug is not
# needed for RUNTIME PASS.
for a in ("pub fn with_added_extension<S: AsRef<OsStr>>",
          "impl fmt::Debug for Components<'_>", "impl fmt::Debug for Iter<'_>",
          "impl fmt::Debug for PathBuf", "impl fmt::Debug for Path"):
    drop(a, a)

# Box/Rc/Arc conversions + FromIterator/Extend use raw-pointer casts / trait
# machinery not needed for the RUNTIME-PASS core (join/parent/file_name/…).
for a in ("pub fn into_boxed_path(self)",
          "impl<P: AsRef<Path>> FromIterator<P> for PathBuf",
          "impl<P: AsRef<Path>> Extend<P> for PathBuf",
          "impl From<Box<Path>> for PathBuf", "impl From<PathBuf> for Box<Path>",
          "impl From<PathBuf> for Arc<Path>", "impl From<&Path> for Arc<Path>",
          "impl From<&mut Path> for Arc<Path>",
          "impl From<PathBuf> for Rc<Path>", "impl From<&Path> for Rc<Path>",
          "impl From<&mut Path> for Rc<Path>",
          "impl From<&Path> for Box<Path>", "impl From<&mut Path> for Box<Path>",
          "impl From<Cow<'_, Path>> for Box<Path>",
          "impl Clone for Box<Path>", "pub fn into_path_buf(self: Box<Self>)",
          "impl FromStr for PathBuf",
          "impl<'a> From<Cow<'a, Path>> for PathBuf"):
    drop(a, a)

# Dead verbatim-normalization branch in PathBuf::_push (prefix_verbatim() is
# always false on Unix): it builds a Vec<Component> that emits as std::vector
# without push/truncate. Neutralize the whole `else if` branch.
_i = s.find("} else if comps.prefix_verbatim() && !path.inner.is_empty() {")
if _i >= 0:
    _b = s.find("{", _i)
    s = s[:_i] + "} else if false {}" + s[matching_close(s, _b):]
else:
    sys.stderr.write("  WARN: verbatim _push branch not found\n")

open(p, "w").write(s)
print("  path.rs prep complete")
PYS
