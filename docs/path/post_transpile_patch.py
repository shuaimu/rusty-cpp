#!/usr/bin/env python3
"""Post-transpile fixups for the std::path C++ port (applied by build.sh).

Everything here targets code that is DEAD on Unix (HAS_PREFIXES == false) but
must still compile: the Windows Prefix machinery. On Unix `parse_prefix` always
returns None, so `Components.prefix` is permanently None and every branch guarded
by it is unreachable — we only need those branches to type-check.
"""
import sys


def patch(text: str) -> str:
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
