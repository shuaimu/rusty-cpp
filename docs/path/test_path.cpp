// Runtime assertions for the std::path port (Unix). Exercises the lexical
// path-manipulation core: construction, as_os_str, is_absolute/has_root, parent,
// push/pop, components, starts_with/strip_prefix.
#include <cassert>
#include <cstdio>
#include <string_view>

#include <rusty/os_str.hpp>

import pathmod;

static std::string_view sv(const rusty::ffi::OsStr& s) { return s.as_str_view(); }

static rusty::ffi::OsString os(std::string_view s) {
    return rusty::ffi::OsString::from(s);
}

#define STEP(msg) std::fprintf(stderr, "[step] %s\n", msg)

int main() {
    STEP("construct");
    // Construction + as_os_str round-trip.
    PathBuf pb = PathBuf::from(os("/foo/bar"));
    STEP("as_os_str");
    assert(sv(pb.as_path().as_os_str()) == "/foo/bar");

    STEP("is_absolute");
    // Absolute / rooted.
    assert(pb.as_path().is_absolute());
    assert(pb.as_path().has_root());
    {
        PathBuf rel = PathBuf::from(os("foo/bar"));
        assert(!rel.as_path().is_absolute());
        assert(!rel.as_path().has_root());
    }

    STEP("parent");
    // parent().
    {
        STEP("parent:call");
        auto par = pb.as_path().parent();
        STEP("parent:is_some");
        assert(par.is_some());
        STEP("parent:unwrap");
        const Path& pp = par.unwrap();
        STEP("parent:as_os_str");
        auto s = sv(pp.as_os_str());
        std::fprintf(stderr, "[parent] = '%.*s'\n", (int)s.size(), s.data());
        assert(s == "/foo");
    }

    STEP("file_name");
    // file_name(): "/foo/bar" -> Some("bar"); "/" -> None.
    {
        auto fnopt = pb.as_path().file_name();
        assert(fnopt.is_some());
        assert(sv(fnopt.unwrap()) == "bar");
        PathBuf root = PathBuf::from(os("/"));
        assert(root.as_path().file_name().is_none());
    }

    STEP("push/pop");
    // push / pop.
    {
        PathBuf p = PathBuf::from(os("/a/b"));
        p.push(rusty::ffi::OsStr(std::string_view("c")));
        assert(sv(p.as_path().as_os_str()) == "/a/b/c");
        assert(p.pop());
        assert(sv(p.as_path().as_os_str()) == "/a/b");
    }

    STEP("components");
    // components(): "/foo/bar" -> RootDir, Normal("foo"), Normal("bar").
    {
        auto comps = pb.as_path().components();
        int normals = 0;
        for (;;) {
            auto c = comps.next();
            if (c.is_none()) break;
            // Only Normal components carry bytes we can inspect here.
            ++normals;
        }
        assert(normals == 3);  // RootDir + foo + bar
    }

    STEP("starts_with/strip");
    // starts_with / strip_prefix.
    {
        PathBuf base = PathBuf::from(os("/foo"));
        assert(pb.as_path().starts_with(rusty::ffi::OsStr(std::string_view("/foo"))));
        auto rest = pb.as_path().strip_prefix(rusty::ffi::OsStr(std::string_view("/foo")));
        assert(rest.is_ok());
        assert(sv(rest.unwrap().as_os_str()) == "bar");
    }

    std::puts("path runtime: all assertions passed");
    return 0;
}
