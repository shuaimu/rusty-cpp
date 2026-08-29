#!/usr/bin/env bash
# End-to-end ASAN check for the transpiled vec_port::Vec::extract_if path.
#
# This intentionally builds a fresh vec_port module from rustc's alloc sources
# instead of relying on stale /tmp artifacts.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

WORK_DIR="${TMPDIR:-/tmp}/rusty-vec-port-extract-if"
KEEP_WORK_DIR=0
CXX="${CXX:-clang++}"

print_usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --work-dir <dir>  Work directory for generated vec_port artifacts
                    (default: ${WORK_DIR})
  --keep-work-dir   Keep generated artifacts after a successful run
  --help            Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --work-dir)
            if [[ $# -lt 2 ]]; then
                echo "error: --work-dir requires a value" >&2
                exit 2
            fi
            WORK_DIR="$2"
            shift 2
            ;;
        --keep-work-dir)
            KEEP_WORK_DIR=1
            shift
            ;;
        --help|-h)
            print_usage
            exit 0
            ;;
        *)
            echo "error: unknown option '$1'" >&2
            print_usage >&2
            exit 2
            ;;
    esac
done

if ! command -v "${CXX}" >/dev/null 2>&1; then
    echo "error: CXX compiler '${CXX}' not found; clang++ with C++20 modules is required" >&2
    exit 1
fi

SYSROOT="$(rustc --print sysroot)"
RUST_SRC="${SYSROOT}/lib/rustlib/src/rust/library/alloc/src"
if [[ ! -d "${RUST_SRC}/vec" || ! -d "${RUST_SRC}/raw_vec" ]]; then
    echo "error: rust-src component not found under ${RUST_SRC}" >&2
    echo "hint: rustup component add rust-src" >&2
    exit 1
fi

VEC_CRATE="${WORK_DIR}/vec_crate"
CPP_OUT="${WORK_DIR}/cpp_out"
BUILD_DIR="${CPP_OUT}/build"
LOG_DIR="${WORK_DIR}/logs"

run_logged() {
    local name="$1"
    shift
    local log="${LOG_DIR}/${name}.log"
    echo "[vec_port] ${name}"
    if ! "$@" >"${log}" 2>&1; then
        echo "error: ${name} failed; showing last 120 log lines" >&2
        tail -n 120 "${log}" >&2 || true
        exit 1
    fi
}

rm -rf "${WORK_DIR}"
mkdir -p "${VEC_CRATE}/src" "${LOG_DIR}"

cp -r "${RUST_SRC}/vec" "${VEC_CRATE}/src/vec"
cp -r "${RUST_SRC}/raw_vec" "${VEC_CRATE}/src/raw_vec"

run_logged prep \
    bash "${REPO_ROOT}/docs/vec_port/prep.sh" \
    "${VEC_CRATE}/src/vec" \
    "${VEC_CRATE}/src/raw_vec"

cat > "${VEC_CRATE}/Cargo.toml" <<'EOF'
[package]
name = "vec_port"
version = "0.0.1"
edition = "2021"

[lib]
path = "src/lib.rs"
EOF

cat > "${VEC_CRATE}/src/lib.rs" <<'EOF'
#![allow(unused)]
pub mod vec;
pub mod raw_vec;
EOF

run_logged transpile \
    cargo run --manifest-path "${REPO_ROOT}/Cargo.toml" -p rusty-cpp-transpiler -- \
    --crate "${VEC_CRATE}/Cargo.toml" \
    --output-dir "${CPP_OUT}"

run_logged post_transpile_patch \
    python3 "${REPO_ROOT}/docs/vec_port/post_transpile_patch.py" "${CPP_OUT}"

if ! grep -q "vec_port: extract_if content merged" "${CPP_OUT}/vec_port.vec.cppm"; then
    echo "error: extract_if content was not merged into vec_port.vec.cppm" >&2
    exit 1
fi
if ! awk '
    /struct ExtractIf \{/ { in_extract_if = 1 }
    in_extract_if && /Vec<T, A>& vec;/ { found_local_vec = 1 }
    in_extract_if && /^};/ { done = 1; exit }
    END { exit (done && found_local_vec) ? 0 : 1 }
' "${CPP_OUT}/vec_port.vec.cppm"; then
    echo "error: ExtractIf does not hold the local transpiled Vec<T, A>&" >&2
    exit 1
fi

ASAN_FLAGS="-I${REPO_ROOT}/include -std=c++23 -fsanitize=address -fno-omit-frame-pointer"
run_logged cmake_configure \
    cmake -B "${BUILD_DIR}" -S "${CPP_OUT}" -G Ninja \
    -DCMAKE_CXX_COMPILER="${CXX}" \
    -DCMAKE_CXX_FLAGS="${ASAN_FLAGS}" \
    -DCMAKE_CXX_STANDARD=23

run_logged cmake_build \
    cmake --build "${BUILD_DIR}" -- -j2

cp "${SCRIPT_DIR}/vec_extract_if_test.cpp" "${CPP_OUT}/vec_extract_if_test.cpp"
run_logged compile_extract_if_test \
    "${CXX}" -std=c++23 -fsanitize=address -fno-omit-frame-pointer \
    -I"${REPO_ROOT}/include" \
    -fprebuilt-module-path="${BUILD_DIR}/CMakeFiles/vec_port.dir" \
    -x c++ "${CPP_OUT}/vec_extract_if_test.cpp" \
    -x none "${BUILD_DIR}/libvec_port.a" \
    -o "${CPP_OUT}/vec_extract_if_test_asan"

run_logged run_extract_if_test \
    env ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 \
    "${CPP_OUT}/vec_extract_if_test_asan"

cat "${LOG_DIR}/run_extract_if_test.log"

if [[ "${KEEP_WORK_DIR}" == "0" ]]; then
    rm -rf "${WORK_DIR}"
fi

echo "PASS: vec_port extract_if is ASAN-clean"
