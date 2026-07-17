#!/usr/bin/env python3
"""Post-transpile fixups for the std::path C++ port (applied by build.sh).

Everything here targets code that is DEAD on Unix (HAS_PREFIXES == false) but
must still compile: the Windows Prefix machinery. On Unix `parse_prefix` always
returns None, so `Components.prefix` is permanently None and every branch guarded
by it is unreachable — we only need those branches to type-check.
"""
import re
import sys


def _replace_fn_body(text: str, sig: str, new_body: str) -> str:
    """Replace the `{ … }` body of the function whose definition starts with
    `sig` (everything up to but excluding the opening brace)."""
    i = text.find(sig)
    if i < 0:
        sys.stderr.write(f"  WARN: fn sig not found: {sig[:40]}...\n")
        return text
    b = text.index("{", i + len(sig))
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
    text = re.sub(r"return /\* matches!\([^;]*\*/;", "return false;", text)

    # `x.as_ref()` on a bare string literal (`push("")`) can't resolve (const
    # char* has no member as_ref); wrap in an OsStr, which does.
    text = text.replace('this->push("")', 'this->push(rusty::ffi::OsStr::new_(""))')

    # MAIN_SEPARATOR_STR: rusty::to_string_view(MAIN_SEP_STR) isn't constexpr;
    # MAIN_SEP_STR is already a string_view "/", so bind it directly, non-constexpr.
    text = text.replace(
        "export constexpr std::string_view MAIN_SEPARATOR_STR = "
        "rusty::to_string_view(sys::path::MAIN_SEP_STR);",
        "export inline const std::string_view MAIN_SEPARATOR_STR = sys::path::MAIN_SEP_STR;",
    )

    # trim_trailing_sep: `while let Some((&last, init)) = bytes.split_last()`
    # lost its bindings (last/init) and its condition became unreachable(). Patch
    # to a correct OsBytes trailing-separator trim over `bytes`.
    text = text.replace(
        "while (rusty::intrinsics::unreachable() && is_sep_byte("
        "std::move(rusty::detail::deref_if_pointer_like(last)))) {\n"
        "            bytes = std::move(init);\n"
        "        }",
        "while (bytes.len() > 0 && is_sep_byte(bytes[bytes.len() - 1])) {\n"
        "            bytes = bytes.slice_to(bytes.len() - 1);\n"
        "        }",
    )

    # `Iterator::eq(a, b)` (element-wise iterator equality) has no such type in
    # scope; use the runtime free function.
    text = text.replace("Iterator::eq(", "rusty::iter_eq(")

    # A component-boundary scan lambda (`let (extra, comp) = if position() … else`)
    # has its return type mis-inferred as tuple<int, &u8>; both branches actually
    # produce (int, &[u8]) — a byte SPAN, not a byte reference.
    text = text.replace(
        "std::tuple<int32_t, const uint8_t&>",
        "std::tuple<int32_t, std::span<const std::uint8_t>>",
    )

    # Rust-style `{name:?}` interpolation survives into a std::println format
    # string (consteval-invalid in C++). Drop the interpolation placeholders.
    text = re.sub(r'\{[A-Za-z_][A-Za-z0-9_]*:\?\}', "", text)

    # `_ if const { !HAS_PREFIXES } => unreachable!()` lowers to
    # `HAS_PREFIXES && rusty::intrinsics::unreachable()` — but unreachable()
    # returns void, invalid in `&&`. The branch is dead on Unix; make it `false`.
    text = text.replace(
        "rusty::detail::deref_if_pointer_like(HAS_PREFIXES) && rusty::intrinsics::unreachable()",
        "false",
    )

    # Drop emitted `using ::X::Y;` re-exports for std namespaces the Unix port
    # doesn't materialize: their trait impls are prep-stripped and the bare
    # names (Cow/Rc/Arc/OsStr/…) resolve through the transpiler's type mapping.
    text = re.sub(
        r"^using ::(borrow|error|hash|iter|rc|str|sync_mod|collections|ops)::[^;]*;\n",
        "",
        text,
        flags=re.M,
    )
    text = re.sub(r"^using ::ffi::os_str;\n", "", text, flags=re.M)

    # AsRef<Path>: path.rs's generic `P: AsRef<Path>` methods lower `x.as_ref()`
    # to a member call yielding an OsStr& (see os_str.hpp; Path/PathBuf already
    # have their own as_ref from the kept AsRef impls). Make Path implicitly
    # constructible from OsStr so `_push(const Path&)` accepts that OsStr&. Path
    # is never aggregate-initialized here.
    text = text.replace(
        "export struct Path {\n    using Owned = PathBuf;\n    rusty::ffi::OsStr inner;\n",
        "export struct Path {\n    using Owned = PathBuf;\n    rusty::ffi::OsStr inner;\n"
        "    Path() = default;\n"
        "    Path(const rusty::ffi::OsStr& _o) : inner(_o) {}\n"
        "    const rusty::ffi::OsStr& as_ref() const { return inner; }\n",
    )

    # _push does `self.inner.push(path)` where path is &Path; OsString::push wants
    # an OsStr, so route through Path::as_ref (an implicit Path->OsStr would make
    # `Path == OsStr` ambiguous).
    text = text.replace(
        "this->inner.push(std::move(path))", "this->inner.push(path.as_ref())"
    )

    # `cfg!(target_os = "cygwin")` lowers to a comment, leaving an empty ternary
    # condition; it is false on Linux.
    text = text.replace('/* cfg!(target_os = "cygwin") */', "false")

    # `const { … }` blocks are elided to `(void)0`. On Unix:
    #  - `if const { !HAS_PREFIXES }`  ->  if (true)
    #  - Components front init `const { if HAS_PREFIXES {Prefix} else {StartDir} }`
    #    ->  State_StartDir()
    cb = "/* const-block elided (Rust 2024 compile-time fence) */ (void)0"
    text = text.replace(f"if ({cb})", "if (true)")
    text = text.replace(f"{cb}, State_Body()", "State_StartDir(), State_Body()")

    # split_file_at_dot returns (&OsStr, Option<&OsStr>) in Rust, but the value
    # port's from_encoded_bytes_unchecked yields owned OsStr temporaries — a tuple
    # of references would dangle. Make the tuple own its OsStr values.
    text = text.replace(
        "std::tuple<const rusty::ffi::OsStr&, rusty::Option<const rusty::ffi::OsStr&>>",
        "std::tuple<rusty::ffi::OsStr, rusty::Option<rusty::ffi::OsStr>>",
    )
    # rsplit_file_at_dot likewise returns (Option<&OsStr>, Option<&OsStr>) from
    # owned temporaries — own both Options.
    text = text.replace(
        "std::tuple<rusty::Option<const rusty::ffi::OsStr&>, "
        "rusty::Option<const rusty::ffi::OsStr&>>",
        "std::tuple<rusty::Option<rusty::ffi::OsStr>, rusty::Option<rusty::ffi::OsStr>>",
    )

    # Components is a DoubleEndedIterator (has next/next_back). The transpiler
    # emits `x.rev()` as a member call; provide it via the runtime free function.
    text = text.replace(
        "export struct Components {\n    using Item = Component;\n",
        "export struct Components {\n    using Item = Component;\n"
        "    auto rev() { return rusty::rev(std::move(*this)); }\n",
    )

    # Component_Normal holds &OsStr in Rust, but the value port builds it from
    # owned OsStr temporaries (from_encoded_bytes_unchecked) — a reference member
    # would dangle. Store the OsStr by value.
    text = text.replace(
        "export struct Component_Normal {\n    const rusty::ffi::OsStr& _0;\n};",
        "export struct Component_Normal {\n    rusty::ffi::OsStr _0;\n};",
    )

    # parse_single_component matches a &[u8] against b"."/b".."/b"" — the
    # transpiler mis-lowers a byte-slice match to std::visit on the span (all arms
    # unreachable, and wrong at runtime). Replace with a correct byte match.
    text = _replace_fn_body(
        text,
        "rusty::Option<Component> Components::parse_single_component"
        "(std::span<const uint8_t> comp) const ",
        "\n"
        "    auto _eq = [](std::span<const uint8_t> a, const char* b, std::size_t n) {\n"
        "        return a.size() == n && std::equal(a.begin(), a.end(),\n"
        "            reinterpret_cast<const std::uint8_t*>(b));\n"
        "    };\n"
        "    if (_eq(comp, \".\", 1)) { return rusty::None; }\n"
        "    if (_eq(comp, \"..\", 2)) { return rusty::Option<Component>(Component{Component_ParentDir{}}); }\n"
        "    if (comp.empty()) { return rusty::None; }\n"
        "    return rusty::Option<Component>(Component{Component_Normal{\n"
        "        rusty::ffi::OsStr::from_encoded_bytes_unchecked(comp)}});\n",
    )

    # Path::from_u8_slice returns `&Path` into an OWNED OsStr temporary (the
    # value port can't borrow a &[u8] as &Path like Rust). Keep the bytes alive
    # in a thread_local Path (same idiom as rusty::path::as_ref) so the returned
    # reference is valid until the next call — callers consume it immediately.
    text = _replace_fn_body(
        text,
        "const Path& Path::from_u8_slice(std::span<const uint8_t> s) ",
        "\n"
        "    thread_local Path _from_u8_tmp;\n"
        "    _from_u8_tmp = Path{rusty::ffi::OsStr::from_encoded_bytes_unchecked(s)};\n"
        "    return _from_u8_tmp;\n",
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
        "rusty::Option<const rusty::ffi::OsStr&> Path::file_name() const ",
        "\n"
        "    thread_local rusty::ffi::OsStr _file_name_tmp;\n"
        "    auto _comp = this->components().next_back();\n"
        "    if (_comp.is_some()) {\n"
        "        auto _c = _comp.unwrap();\n"
        "        if (_c.index() == 3) {\n"
        "            _file_name_tmp = std::get<3>(_c)._0;\n"
        "            return rusty::Option<const rusty::ffi::OsStr&>(_file_name_tmp);\n"
        "        }\n"
        "    }\n"
        "    return rusty::Option<const rusty::ffi::OsStr&>{rusty::None};\n",
    )

    # Path::is_absolute delegates to sys::path::is_absolute(self), which the
    # transpiler mis-lowered to `(*this).is_absolute()` — infinite recursion. On
    # Unix is_absolute == has_root (a leading '/').
    text = _replace_fn_body(
        text,
        "bool Path::is_absolute() const ",
        "\n    return this->has_root();\n",
    )

    # Component is a data enum whose derived PartialEq compares the underlying
    # std::variant — which needs each alternative to have operator==. The
    # transpiler emits variant member structs (Component_RootDir/…/Normal)
    # WITHOUT one, so inject a defaulted == (empty variants compare equal;
    # Component_Normal's reference member compares its OsStr referent).
    text = re.sub(
        r"export struct (Component_[A-Za-z]+) \{([^}]*)\};",
        lambda m: "export struct {0} {{{1} bool operator==(const {0}&) const = default; }};".format(
            m.group(1), m.group(2)
        ),
        text,
    )

    # The dead `self.prefix.map(|p| p.<method>())` branches lose their closure
    # param `p` in emission, leaving it undeclared. These Prefix methods are only
    # reachable through a prefix (always None on Unix), so the branch never runs.
    text = text.replace("p.has_implicit_root()", "false")
    text = text.replace("p.is_verbatim()", "false")

    # NOTE: the Components::is_sep_byte free-vs-member name collision (the member
    # called the FREE sys::path::is_sep_byte unqualified -> bound to itself ->
    # infinite recursion) is now fixed IN THE TRANSPILER (it qualifies a bare call
    # that collides with an enclosing-Self method), so no patch is needed here.
    return text


def main() -> None:
    path = sys.argv[1]
    src = open(path).read()
    open(path, "w").write(patch(src))


if __name__ == "__main__":
    main()
