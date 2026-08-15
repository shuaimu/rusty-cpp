#!/usr/bin/env python3
"""Post-transpile fixups for the std::path C++ port (applied by build.sh).

Everything here targets code that is DEAD on Unix (HAS_PREFIXES == false) but
must still compile: the Windows Prefix machinery. On Unix `parse_prefix` always
returns None, so `Components.prefix` is permanently None and every branch guarded
by it is unreachable — we only need those branches to type-check.

ANCHORING DISCIPLINE (learned the hard way — see the `path` matrix regression):
an anchor that stops matching patches NOTHING and, without the bookkeeping
below, says NOTHING. This port went red for weeks because the emitter respelled
`rusty::intrinsics::unreachable()` -> `unreachable_panic()` and
`rusty::ffi::OsStr` -> `ffi::OsStr`, silently killing ~7 rewrites at once. So:

  * every anchor goes through `_rep` / `_sub` / `_replace_fn_body` with a LABEL,
    and an anchor that matches nothing is recorded and reported on stderr;
  * `main()` exits non-zero when a REQUIRED anchor missed, so build.sh fails
    loudly instead of handing clang half-patched source;
  * anchors match on SHAPE, not on one spelling — namespace qualification is
    optional (`(?:rusty::)?ffi::OsStr`), as is the `_panic` suffix, and struct
    anchors key on the header plus the field they care about rather than an
    exact run of lines (the emitter interleaves `using` aliases freely).
"""
import re
import sys


# Anchors that matched nothing this run. Populated by the helpers below.
_MISSES: list[str] = []
# Anchors that are allowed to match nothing: the transpiler bug they papered
# over has since been FIXED, so their absence is the good outcome. They stay
# here (rather than being deleted) so a regression re-announces itself.
_OPTIONAL_MISSES: list[str] = []

# `ffi::OsStr` with optional namespace qualification. The emitter has spelled it
# both ways; build.sh injects `namespace ffi = rusty::ffi;` into the global
# fragment, so both resolve to the same entity.
OSSTR = r"(?:rusty::)?ffi::OsStr"
# `unreachable()` / `unreachable_panic()` — renamed by d0e5c088.
UNREACHABLE = r"rusty::intrinsics::unreachable(?:_panic)?\(\)"


def _rep(text: str, old: str, new: str, label: str, required: bool = True) -> str:
    """Literal replace that records a miss instead of silently doing nothing."""
    if old not in text:
        (_MISSES if required else _OPTIONAL_MISSES).append(label)
        return text
    return text.replace(old, new)


def _sub(
    text: str,
    pattern: str,
    repl: str,
    label: str,
    flags: int = 0,
    required: bool = True,
) -> str:
    """Regex replace that records a miss instead of silently doing nothing."""
    new, n = re.subn(pattern, repl, text, flags=flags)
    if n == 0:
        (_MISSES if required else _OPTIONAL_MISSES).append(label)
    return new


def _replace_fn_body(text: str, sig: str, new_body: str, label: str) -> str:
    """Replace the `{ … }` body of the function whose definition starts with
    `sig` — a REGEX matching everything up to but excluding the opening brace."""
    m = re.search(sig, text)
    if m is None:
        _MISSES.append(label)
        return text
    b = text.index("{", m.end())
    depth, j = 0, b
    while j < len(text):
        if text[j] == "{":
            depth += 1
        elif text[j] == "}":
            depth -= 1
            if depth == 0:
                break
        j += 1
    return text[: b + 1] + new_body + text[j:]


def patch(text: str) -> str:
    # Some `matches!(...)` invocations on the dead Prefix machinery lower to a
    # comment (unresolved), leaving `return /* … */;` in a bool function — void.
    # These are unreachable on Unix (no prefix is ever built); make them `false`.
    text = _sub(
        text,
        r"return /\* matches!\([^;]*\*/;",
        "return false;",
        "matches!-comment -> false",
        required=False,
    )

    # `x.as_ref()` on a bare string literal (`push("")`) can't resolve (const
    # char* has no member as_ref); wrap in an OsStr, which does.
    text = _rep(
        text,
        'this->push("")',
        'this->push(ffi::OsStr::new_(""))',
        'push("") -> OsStr',
    )

    # MAIN_SEPARATOR_STR: rusty::to_string_view(MAIN_SEP_STR) isn't constexpr;
    # MAIN_SEP_STR is already a string_view "/", so bind it directly, non-constexpr.
    text = _rep(
        text,
        "export constexpr std::string_view MAIN_SEPARATOR_STR = "
        "rusty::to_string_view(sys::path::MAIN_SEP_STR);",
        "export inline const std::string_view MAIN_SEPARATOR_STR = sys::path::MAIN_SEP_STR;",
        "MAIN_SEPARATOR_STR non-constexpr",
    )

    # trim_trailing_sep: `while let Some((&last, init)) = bytes.split_last()`
    # lost its bindings (last/init) and its condition became unreachable(). Patch
    # to a correct OsBytes trailing-separator trim over `bytes`.
    text = _sub(
        text,
        UNREACHABLE + r" && is_sep_byte\(\s*std::move\([^;]*?\)\)\) \{\n"
        r"\s*bytes = std::move\(init\);\n\s*\}",
        "bytes.len() > 0 && is_sep_byte(bytes[bytes.len() - 1])) {\n"
        "            bytes = bytes.slice_to(bytes.len() - 1);\n"
        "        }",
        "trim_trailing_sep while-loop",
    )

    # `Iterator::eq(a, b)` (element-wise iterator equality) has no such type in
    # scope; use the runtime free function.
    text = _rep(text, "Iterator::eq(", "rusty::iter_eq(", "Iterator::eq -> iter_eq")

    # A component-boundary scan lambda (`let (extra, comp) = if position() … else`)
    # has its return type mis-inferred as tuple<int, &u8>; both branches actually
    # produce (int, &[u8]) — a byte SPAN, not a byte reference.
    text = _rep(
        text,
        "std::tuple<int32_t, const uint8_t&>",
        "std::tuple<int32_t, std::span<const std::uint8_t>>",
        "component-scan tuple<int,&u8> -> span",
    )

    # Rust-style `{name:?}` interpolation survives into a std::println format
    # string (consteval-invalid in C++). Drop the interpolation placeholders.
    text = _sub(
        text,
        r"\{[A-Za-z_][A-Za-z0-9_]*:\?\}",
        "",
        "{name:?} placeholder strip",
        required=False,
    )

    # `_ if const { !HAS_PREFIXES } => unreachable!()` used to lower to
    # `HAS_PREFIXES && rusty::intrinsics::unreachable()` — unreachable() returns
    # void, invalid in `&&`. The transpiler now lowers this arm as a statement
    # (`if (rusty::detail::rust_not(HAS_PREFIXES))`), so the rewrite is obsolete.
    text = _sub(
        text,
        r"rusty::detail::deref_if_pointer_like\(HAS_PREFIXES\) && " + UNREACHABLE,
        "false",
        "HAS_PREFIXES && unreachable -> false (obsolete)",
        required=False,
    )

    # Drop emitted `using ::X::Y;` re-exports for std namespaces the Unix port
    # doesn't materialize: their trait impls are prep-stripped and the bare
    # names (Cow/Rc/Arc/OsStr/…) resolve through the transpiler's type mapping.
    text = _sub(
        text,
        r"^using ::(borrow|error|hash|iter|rc|str|sync_mod|collections|ops)::[^;]*;\n",
        "",
        "drop unmaterialized std re-exports",
        flags=re.M,
    )
    text = _sub(
        text, r"^using ::ffi::os_str;\n", "", "drop ::ffi::os_str re-export", flags=re.M
    )

    # AsRef<Path>: path.rs's generic `P: AsRef<Path>` methods lower `x.as_ref()`
    # to a member call yielding an OsStr& (see os_str.hpp; Path/PathBuf already
    # have their own as_ref from the kept AsRef impls). Make Path implicitly
    # constructible from OsStr so `_push(const Path&)` accepts that OsStr&. Path
    # is never aggregate-initialized here.
    #
    # Anchored on the struct header + the `inner` field, NOT on an exact line
    # run: the emitter interleaves `using Item = …;` / `using IntoIter = …;`
    # aliases between them and adds more over time.
    text = _sub(
        text,
        r"(export struct Path \{\n(?:    using [^\n]*\n)*    " + OSSTR + r" inner;\n)",
        r"\1"
        "    Path() = default;\n"
        "    Path(const ffi::OsStr& _o) : inner(_o) {}\n"
        "    const ffi::OsStr& as_ref() const { return inner; }\n",
        "struct Path ctor/as_ref injection",
    )

    # _push does `self.inner.push(path)` where path is &Path; OsString::push wants
    # an OsStr, so route through Path::as_ref (an implicit Path->OsStr would make
    # `Path == OsStr` ambiguous).
    text = _rep(
        text,
        "this->inner.push(std::move(path))",
        "this->inner.push(path.as_ref())",
        "_push -> path.as_ref()",
    )

    # `cfg!(target_os = "cygwin")` lowers to a comment, leaving an empty ternary
    # condition; it is false on Linux.
    text = _rep(
        text,
        '/* cfg!(target_os = "cygwin") */',
        "false",
        "cfg!(cygwin) -> false",
        required=False,
    )

    # `const { … }` blocks are elided to `(void)0`. On Unix:
    #  - `if const { !HAS_PREFIXES }`  ->  if (true)
    #  - Components front init `const { if HAS_PREFIXES {Prefix} else {StartDir} }`
    #    ->  State_StartDir()
    cb = "/* const-block elided (Rust 2024 compile-time fence) */ (void)0"
    text = _rep(text, f"if ({cb})", "if (true)", "const-block if -> true", required=False)
    text = _rep(
        text,
        f"{cb}, State_Body()",
        "State_StartDir(), State_Body()",
        "const-block Components front init",
        required=False,
    )

    # split_file_at_dot returns (&OsStr, Option<&OsStr>) in Rust, but the value
    # port's from_encoded_bytes_unchecked yields owned OsStr temporaries — a tuple
    # of references would dangle. Make the tuple own its OsStr values.
    text = _sub(
        text,
        r"std::tuple<const " + OSSTR + r"&, rusty::Option<const " + OSSTR + r"&>>",
        "std::tuple<ffi::OsStr, rusty::Option<ffi::OsStr>>",
        "split_file_at_dot owning tuple",
    )
    # rsplit_file_at_dot likewise returns (Option<&OsStr>, Option<&OsStr>) from
    # owned temporaries — own both Options.
    text = _sub(
        text,
        r"std::tuple<rusty::Option<const " + OSSTR + r"&>, "
        r"rusty::Option<const " + OSSTR + r"&>>",
        "std::tuple<rusty::Option<ffi::OsStr>, rusty::Option<ffi::OsStr>>",
        "rsplit_file_at_dot owning tuple",
    )

    # Components is a DoubleEndedIterator (has next/next_back). The transpiler
    # emits `x.rev()` as a member call; provide it via the runtime free function.
    text = _sub(
        text,
        r"(export struct Components \{\n    using Item = Component;\n)",
        r"\1    auto rev() { return rusty::rev(std::move(*this)); }\n",
        "Components::rev injection",
    )

    # Component_Normal holds &OsStr in Rust, but the value port builds it from
    # owned OsStr temporaries (from_encoded_bytes_unchecked) — a reference member
    # would dangle. Store the OsStr by value. (Replace just the field line — the
    # struct now also carries a transpiler-emitted `operator== = default` after
    # it, from derive(PartialEq); the defaulted == then compares the OsStr value.)
    text = _sub(
        text,
        r"(export struct Component_Normal \{\n)    const " + OSSTR + r"& _0;\n",
        r"\1    ffi::OsStr _0;\n",
        "Component_Normal by-value field",
    )

    # parse_single_component matches a &[u8] against b"."/b".."/b"" — the
    # transpiler mis-lowers a byte-slice match to std::visit on the span (all arms
    # unreachable, and wrong at runtime). Replace with a correct byte match.
    text = _replace_fn_body(
        text,
        r"rusty::Option<Component> Components::parse_single_component"
        r"\(std::span<const uint8_t> comp\) const ",
        "\n"
        "    auto _eq = [](std::span<const uint8_t> a, const char* b, std::size_t n) {\n"
        "        return a.size() == n && std::equal(a.begin(), a.end(),\n"
        "            reinterpret_cast<const std::uint8_t*>(b));\n"
        "    };\n"
        "    if (_eq(comp, \".\", 1)) { return rusty::None; }\n"
        "    if (_eq(comp, \"..\", 2)) { return rusty::Option<Component>(Component{Component_ParentDir{}}); }\n"
        "    if (comp.empty()) { return rusty::None; }\n"
        "    return rusty::Option<Component>(Component{Component_Normal{\n"
        "        ffi::OsStr::from_encoded_bytes_unchecked(comp)}});\n",
        "parse_single_component byte match",
    )

    # Path::from_u8_slice returns `&Path` into an OWNED OsStr temporary (the
    # value port can't borrow a &[u8] as &Path like Rust). Keep the bytes alive
    # in a thread_local Path (same idiom as rusty::path::as_ref) so the returned
    # reference is valid until the next call — callers consume it immediately.
    text = _replace_fn_body(
        text,
        r"const Path& Path::from_u8_slice\(std::span<const uint8_t> s\) ",
        "\n"
        "    thread_local Path _from_u8_tmp;\n"
        "    _from_u8_tmp = Path{ffi::OsStr::from_encoded_bytes_unchecked(s)};\n"
        "    return _from_u8_tmp;\n",
        "from_u8_slice thread_local",
    )

    # Path::file_name = `next_back().and_then(|p| match p { Normal(p) => Some(p),
    # _ => None })` returns `&OsStr` borrowed out of next_back()'s OWNED Component
    # temporary (Component_Normal holds an OsStr BY VALUE in the value port), so
    # the reference dangles at return (ASan: stack-use-after-return). Materialize
    # the found component's bytes into a thread_local OsStr — same idiom as
    # from_u8_slice; callers consume the result immediately. Normal is variant
    # index 3 (Prefix stripped on Unix: RootDir=0, CurDir=1, ParentDir=2, Normal=3).
    text = _replace_fn_body(
        text,
        r"rusty::Option<const " + OSSTR + r"&> Path::file_name\(\) const ",
        "\n"
        "    thread_local ffi::OsStr _file_name_tmp;\n"
        "    auto _comp = this->components().next_back();\n"
        "    if (_comp.is_some()) {\n"
        "        auto _c = _comp.unwrap();\n"
        "        if (_c.index() == 3) {\n"
        "            _file_name_tmp = std::get<3>(_c)._0;\n"
        "            return rusty::Option<const ffi::OsStr&>(_file_name_tmp);\n"
        "        }\n"
        "    }\n"
        "    return rusty::Option<const ffi::OsStr&>{rusty::None};\n",
        "file_name thread_local",
    )

    # file_stem/extension/file_prefix are `file_name().map(r?split_file_at_dot)
    # .and_then(...)` in Rust — returning `&OsStr` slices of file_name()'s result.
    # In the value port both file_name()'s thread_local AND split's owned tuple are
    # temporaries the returned ref would dangle into, and the emitted
    # Option<OsStr>-value chain doesn't even convert to Option<const OsStr&>.
    # Reimplement each directly over file_name()'s bytes (byte-find at '.') and
    # materialize the result into a per-method thread_local OsStr. Mirrors std's
    # rsplit_file_at_dot (stem=before-or-after, ext=before-and-after; last dot)
    # and split_file_at_dot (prefix=before; FIRST dot at index >= 1); the leading
    # dot (index 0) and ".." are treated as an extension-less whole, per std.
    def _osstr_thread_local_body(tmp, compute):
        return (
            "\n"
            "    thread_local ffi::OsStr {tmp};\n"
            "    auto _fn = this->file_name();\n"
            "    if (_fn.is_none()) {{ return rusty::Option<const ffi::OsStr&>{{rusty::None}}; }}\n"
            "    std::string_view _s = _fn.unwrap().as_str_view();\n"
            "    constexpr auto _npos = std::string_view::npos;\n"
            "{compute}"
        ).format(tmp=tmp, compute=compute)

    text = _replace_fn_body(
        text,
        r"rusty::Option<const " + OSSTR + r"&> Path::file_stem\(\) const ",
        _osstr_thread_local_body(
            "_stem_tmp",
            "    std::string_view _r;\n"
            "    auto _dot = _s.rfind('.');\n"
            "    if (_s == \"..\" || _dot == _npos || _dot == 0) { _r = _s; }\n"
            "    else { _r = _s.substr(0, _dot); }\n"
            "    _stem_tmp = ffi::OsStr(_r);\n"
            "    return rusty::Option<const ffi::OsStr&>(_stem_tmp);\n",
        ),
        "file_stem thread_local",
    )

    text = _replace_fn_body(
        text,
        r"rusty::Option<const " + OSSTR + r"&> Path::extension\(\) const ",
        _osstr_thread_local_body(
            "_ext_tmp",
            "    auto _dot = _s.rfind('.');\n"
            "    if (_s == \"..\" || _dot == _npos || _dot == 0) {\n"
            "        return rusty::Option<const ffi::OsStr&>{rusty::None};\n"
            "    }\n"
            "    _ext_tmp = ffi::OsStr(_s.substr(_dot + 1));\n"
            "    return rusty::Option<const ffi::OsStr&>(_ext_tmp);\n",
        ),
        "extension thread_local",
    )

    text = _replace_fn_body(
        text,
        r"rusty::Option<const " + OSSTR + r"&> Path::file_prefix\(\) const ",
        _osstr_thread_local_body(
            "_prefix_tmp",
            "    std::string_view _r;\n"
            "    auto _dot = _s.find('.', 1);\n"
            "    if (_s == \"..\" || _dot == _npos) { _r = _s; }\n"
            "    else { _r = _s.substr(0, _dot); }\n"
            "    _prefix_tmp = ffi::OsStr(_r);\n"
            "    return rusty::Option<const ffi::OsStr&>(_prefix_tmp);\n",
        ),
        "file_prefix thread_local",
    )

    # Path::is_absolute delegates to sys::path::is_absolute(self), which the
    # transpiler mis-lowered to `(*this).is_absolute()` — infinite recursion. On
    # Unix is_absolute == has_root (a leading '/').
    text = _replace_fn_body(
        text,
        r"bool Path::is_absolute\(\) const ",
        "\n    return this->has_root();\n",
        "is_absolute -> has_root",
    )

    # impl Prefix's methods are DEAD on Unix (Prefix is never constructed) but
    # must still type-check — they're referenced by the never-taken
    # `self.prefix.as_ref().map(|p| p.len()/is_verbatim())` closures in
    # Components. Their real bodies `match *self { … }` / `matches!(*self, …)`
    # now lower to a Prefix std::visit (matches! became a real expression) that
    # can't resolve. Replace each body with its Unix-constant value.
    for sig, body, label in (
        (r"size_t Prefix::len\(\) const ", "\n    return 0;\n", "Prefix::len"),
        (r"bool Prefix::is_verbatim\(\) const ", "\n    return false;\n", "Prefix::is_verbatim"),
        (r"bool Prefix::is_drive\(\) const ", "\n    return false;\n", "Prefix::is_drive"),
        (
            r"bool Prefix::has_implicit_root\(\) const ",
            "\n    return true;\n",
            "Prefix::has_implicit_root",
        ),
    ):
        text = _replace_fn_body(text, sig, body, label)

    # NOTE: Component's variant member structs (Component_RootDir/…/Normal) now
    # get their defaulted `operator==` FROM THE TRANSPILER — a data enum deriving
    # PartialEq emits one on each variant struct (needed for the std::variant
    # comparison). No injection needed here.

    # The dead `self.prefix.map(|p| p.<method>())` branches lose their closure
    # param `p` in emission, leaving it undeclared. These Prefix methods are only
    # reachable through a prefix (always None on Unix), so the branch never runs.
    text = _rep(text, "p.has_implicit_root()", "false", "dead p.has_implicit_root")
    text = _rep(text, "p.is_verbatim()", "false", "dead p.is_verbatim")

    # NOTE: the Components::is_sep_byte free-vs-member name collision (the member
    # called the FREE sys::path::is_sep_byte unqualified -> bound to itself ->
    # infinite recursion) is now fixed IN THE TRANSPILER (it qualifies a bare call
    # that collides with an enclosing-Self method), so no patch is needed here.
    return text


def main() -> None:
    path = sys.argv[1]
    src = open(path).read()
    out = patch(src)
    open(path, "w").write(out)

    for label in _OPTIONAL_MISSES:
        sys.stderr.write(f"  note: obsolete anchor did not match (fine): {label}\n")
    if _MISSES:
        sys.stderr.write(
            f"ERROR: {len(_MISSES)} post-transpile anchor(s) matched NOTHING — the\n"
            "emitted C++ has changed shape and these rewrites were skipped:\n"
        )
        for label in _MISSES:
            sys.stderr.write(f"  - {label}\n")
        sys.exit(1)


if __name__ == "__main__":
    main()
