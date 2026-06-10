#ifndef RUSTY_FFI_HPP
#define RUSTY_FFI_HPP

#include <cstdint>
#include <string>
#include <vector>

namespace rusty::ffi {

// Rust's `OsString::from_vec` is a Unix-only extension on `OsStringExt`
// that adopts a `Vec<u8>` as the OS string's bytes. We map `OsString`
// to `std::basic_string<char>` (no dedicated rusty type), so the
// natural lowering is the range-iterator constructor — copies the
// bytes into a `std::string`. The transpiler emits
// `rusty::ffi::os_string_from_vec` for both the qualified
// `OsString::from_vec` path and the `OsStringExt::from_vec` trait
// method, since both lower to the same `Vec<u8> -> std::string` shape.
//
// Templated on the input container so it works whether the call site
// passes `rusty::Vec<uint8_t>` (from `import rusty;`), `std::vector<
// uint8_t>` (raw STL storage), or any other byte container exposing
// `.data()` + `.size()`. We avoid #including `<rusty/vec.hpp>` here
// because `rusty::Vec` only resolves through the rusty module
// (`import rusty;`), not via headers — see the comment at the top of
// `rusty/vec.hpp`.
template<typename Bytes>
inline std::string os_string_from_vec(const Bytes& bytes) {
    return std::string(
        reinterpret_cast<const char*>(bytes.data()),
        bytes.size()
    );
}

} // namespace rusty::ffi

#endif // RUSTY_FFI_HPP
