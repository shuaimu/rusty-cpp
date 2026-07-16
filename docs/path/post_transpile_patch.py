#!/usr/bin/env python3
"""Post-transpile fixups for the std::path C++ port (applied by build.sh).

Everything here targets code that is DEAD on Unix (HAS_PREFIXES == false) but
must still compile: the Windows Prefix machinery. On Unix `parse_prefix` always
returns None, so `Components.prefix` is permanently None and every branch guarded
by it is unreachable — we only need those branches to type-check.
"""
import re
import sys


def patch(text: str) -> str:
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
        "    Path(const rusty::ffi::OsStr& _o) : inner(_o) {}\n",
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
    return text


def main() -> None:
    path = sys.argv[1]
    src = open(path).read()
    open(path, "w").write(patch(src))


if __name__ == "__main__":
    main()
