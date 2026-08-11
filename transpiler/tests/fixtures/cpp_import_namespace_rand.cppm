module;

#include <cstdint>

export module rrr.rand;

export namespace rrr {

std::uint64_t randgen_rand_raw() {
    return 7;
}

double randgen_rand_max() {
    return 10.0;
}

} // export namespace rrr
