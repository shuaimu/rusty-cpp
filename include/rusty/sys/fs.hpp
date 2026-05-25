#ifndef RUSTY_SYS_FS_HPP
#define RUSTY_SYS_FS_HPP

// rusty::sys::fs — Rust-like filesystem helpers for C++
//
// A minimal subset of Rust's std::fs aimed at making @safe callers
// possible without leaking std::ifstream / FILE* / char* into safe
// code:
//
//   read_to_string(path) -> Result<std::string, io::Error>
//
// The function body wraps libstdc++ I/O in a single @unsafe block so
// the function itself can be called from @safe code.

#include <fstream>
#include <sstream>
#include <string>
#include <string_view>

#include "rusty/io.hpp"     // for rusty::io::Error
#include "rusty/result.hpp"

namespace rusty {
namespace sys {
namespace fs {

// @safe - read the entire file at `path` into a string. Returns
// `io::Error` (Kind::NotFound on open failure, Kind::Other on read
// failure) — no FILE* / ifstream handle escapes the call.
inline rusty::Result<std::string, rusty::io::Error>
read_to_string(std::string_view path) {
    // @unsafe { std::ifstream / std::stringstream / rdbuf are libstdc++
    //           types not reachable by the borrow checker; the function
    //           returns by value so no raw handle escapes. }
    {
        std::ifstream in(std::string{path},
                         std::ios::in | std::ios::binary);
        if (!in.is_open()) {
            return rusty::Err<std::string, rusty::io::Error>(
                rusty::io::Error(rusty::io::Error::Kind::NotFound,
                                 std::string{path}));
        }
        std::stringstream buf;
        buf << in.rdbuf();
        if (in.bad()) {
            return rusty::Err<std::string, rusty::io::Error>(
                rusty::io::Error(rusty::io::Error::Kind::Other,
                                 std::string{path}));
        }
        return rusty::Ok<std::string, rusty::io::Error>(buf.str());
    }
}

}  // namespace fs
}  // namespace sys
}  // namespace rusty

#endif  // RUSTY_SYS_FS_HPP
