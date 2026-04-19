#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

RUSTYCPP_DIR="${EXAMPLE_ROOT}/.."
WORK_DIR="${EXAMPLE_ROOT}/build/cpp_rust_member_interop"
DRY_RUN=0

print_usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Compile and run the C++ <-> Rust member-call interop example.

Options:
  --rustycpp-dir <dir>  Path to rusty-cpp repository/root
  --work-dir <dir>      Working directory for generated artifacts
  --dry-run             Print planned commands without executing
  --help                Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --rustycpp-dir)
            if [[ $# -lt 2 ]]; then
                echo "error: --rustycpp-dir requires a value" >&2
                exit 2
            fi
            RUSTYCPP_DIR="$2"
            shift 2
            ;;
        --work-dir)
            if [[ $# -lt 2 ]]; then
                echo "error: --work-dir requires a value" >&2
                exit 2
            fi
            WORK_DIR="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
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

RUST_SOURCE="${EXAMPLE_ROOT}/interop/rust/member_bridge.rs"
CPP_MODULE_INDEX="${EXAMPLE_ROOT}/interop/cpp_module_index.toml"
CPP_HOST_MODULE="${EXAMPLE_ROOT}/interop/cpp/interop.host.cppm"
CPP_MAIN_SOURCE="${EXAMPLE_ROOT}/interop/cpp/interop_main.cpp"
TRANSPILED_CPPM="${WORK_DIR}/interop.bridge.cppm"
PROGRAM_PATH="${WORK_DIR}/interop_member_demo"
TRANSPILE_LOG="${WORK_DIR}/transpile.log"
BUILD_LOG="${WORK_DIR}/build.log"

report_failure() {
    local stage="$1"
    echo "interop test failed at stage: ${stage}" >&2
    echo "rustycpp dir: ${RUSTYCPP_DIR}" >&2
    echo "work dir: ${WORK_DIR}" >&2
    echo "transpile.log: ${TRANSPILE_LOG}" >&2
    echo "build.log: ${BUILD_LOG}" >&2
}

echo "═══════════════════════════════════════════════════════════════════════"
echo "C++ <-> Rust member interop test"
echo "  rustycpp dir: ${RUSTYCPP_DIR}"
echo "  work dir: ${WORK_DIR}"
if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "  mode: dry-run"
fi
echo "═══════════════════════════════════════════════════════════════════════"

if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "[dry-run] cargo run -p rusty-cpp-transpiler -- ${RUST_SOURCE} --output ${TRANSPILED_CPPM} --module-name interop.bridge --cpp-module-index ${CPP_MODULE_INDEX}"
    echo "[dry-run] g++ -std=c++23 -fmodules-ts -I ${RUSTYCPP_DIR}/include -x c++ -c ${CPP_HOST_MODULE} -o ${WORK_DIR}/interop.host.o"
    echo "[dry-run] g++ -std=c++23 -fmodules-ts -I ${RUSTYCPP_DIR}/include -x c++ -c ${TRANSPILED_CPPM} -o ${WORK_DIR}/interop.bridge.o"
    echo "[dry-run] g++ -std=c++23 -fmodules-ts -I ${RUSTYCPP_DIR}/include ${CPP_MAIN_SOURCE} ${WORK_DIR}/interop.host.o ${WORK_DIR}/interop.bridge.o -o ${PROGRAM_PATH}"
    echo "[dry-run] ${PROGRAM_PATH}"
    exit 0
fi

rm -rf "${WORK_DIR}"
mkdir -p "${WORK_DIR}"

: >"${TRANSPILE_LOG}"
: >"${BUILD_LOG}"

PARITY_CMD=(
    cargo
    run
    -p
    rusty-cpp-transpiler
    --
    "${RUST_SOURCE}"
    --output
    "${TRANSPILED_CPPM}"
    --module-name
    "interop.bridge"
    --cpp-module-index
    "${CPP_MODULE_INDEX}"
)

if ! (
    cd "${RUSTYCPP_DIR}" &&
    "${PARITY_CMD[@]}"
) >"${TRANSPILE_LOG}" 2>&1; then
    report_failure "transpile"
    tail -n 80 "${TRANSPILE_LOG}" >&2 || true
    exit 1
fi

if [[ ! -f "${TRANSPILED_CPPM}" ]]; then
    report_failure "transpile-output"
    echo "missing transpiled output: ${TRANSPILED_CPPM}" >&2
    exit 1
fi

SUPPORTED_COMPILER=""
SUPPORTED_FLAGS=""

PROBE_SOURCE="${WORK_DIR}/import_std_probe.cppm"
cat >"${PROBE_SOURCE}" <<'EOF'
export module import_std_probe;
import std;
export int probe_value() { return 1; }
EOF

try_probe() {
    local compiler="$1"
    shift
    local flags=("$@")
    if ! command -v "${compiler}" >/dev/null 2>&1; then
        return 2
    fi
    if (
        cd "${WORK_DIR}" &&
        "${compiler}" "${flags[@]}" -x c++ -c "${PROBE_SOURCE}" -o import_std_probe.o
    ) >>"${BUILD_LOG}" 2>&1; then
        SUPPORTED_COMPILER="${compiler}"
        SUPPORTED_FLAGS="${flags[*]}"
        return 0
    fi
    return 1
}

if ! try_probe "g++" "-std=c++23" "-fmodules-ts"; then
    echo "g++ probe failed or unavailable" >>"${BUILD_LOG}"
fi
if [[ -z "${SUPPORTED_COMPILER}" ]]; then
    if ! try_probe "clang++" "-std=c++20" "-stdlib=libc++"; then
        echo "clang++ probe failed or unavailable" >>"${BUILD_LOG}"
    fi
fi

if [[ -z "${SUPPORTED_COMPILER}" ]]; then
    echo "SKIP: no compiler with working C++20 module support (import std)" | tee -a "${BUILD_LOG}"
    exit 0
fi

IFS=' ' read -r -a ACTIVE_FLAGS <<<"${SUPPORTED_FLAGS}"
echo "using compiler: ${SUPPORTED_COMPILER} ${SUPPORTED_FLAGS}" | tee -a "${BUILD_LOG}"

if ! (
    cd "${WORK_DIR}" &&
    "${SUPPORTED_COMPILER}" "${ACTIVE_FLAGS[@]}" -I "${RUSTYCPP_DIR}/include" -x c++ -c "${CPP_HOST_MODULE}" -o interop.host.o
) >>"${BUILD_LOG}" 2>&1; then
    report_failure "compile-host-module"
    tail -n 80 "${BUILD_LOG}" >&2 || true
    exit 1
fi

if ! (
    cd "${WORK_DIR}" &&
    "${SUPPORTED_COMPILER}" "${ACTIVE_FLAGS[@]}" -I "${RUSTYCPP_DIR}/include" -x c++ -c "${TRANSPILED_CPPM}" -o interop.bridge.o
) >>"${BUILD_LOG}" 2>&1; then
    report_failure "compile-rust-module"
    tail -n 80 "${BUILD_LOG}" >&2 || true
    exit 1
fi

if ! (
    cd "${WORK_DIR}" &&
    "${SUPPORTED_COMPILER}" "${ACTIVE_FLAGS[@]}" -I "${RUSTYCPP_DIR}/include" "${CPP_MAIN_SOURCE}" interop.host.o interop.bridge.o -o "${PROGRAM_PATH}"
) >>"${BUILD_LOG}" 2>&1; then
    report_failure "link-main"
    tail -n 80 "${BUILD_LOG}" >&2 || true
    exit 1
fi

if ! "${PROGRAM_PATH}" >>"${BUILD_LOG}" 2>&1; then
    report_failure "run-program"
    tail -n 80 "${BUILD_LOG}" >&2 || true
    exit 1
fi

echo "PASS: C++ <-> Rust member interop test"
echo "build.log: ${BUILD_LOG}"
