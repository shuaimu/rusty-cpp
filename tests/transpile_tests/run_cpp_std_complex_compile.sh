#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

WORK_DIR="${REPO_ROOT}/.rusty-cpp-std-complex"
DRY_RUN=0

print_usage() {
    cat <<USAGE
Usage: $(basename "$0") [options]

Run compile-stage cpp::std complex interop coverage for the committed fixture crate.

Options:
  --work-dir <dir>  Working directory for transpile + compile artifacts
  --dry-run         Print planned commands without executing
  --help            Show this help
USAGE
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

MANIFEST_PATH="${REPO_ROOT}/tests/transpile_tests/cpp_std_complex/Cargo.toml"
INDEX_PATH="${REPO_ROOT}/tests/transpile_tests/cpp_std_complex/cpp_module_index.toml"
RUSTY_RUNTIME_MODULE_PATH="${REPO_ROOT}/include/rusty/rusty.cppm"
TARGET_CPPM_PATH="${WORK_DIR}/targets/cpp_std_complex/cpp_std_complex.cppm"
TRANSPILE_LOG="${WORK_DIR}/transpile.log"
BUILD_LOG="${WORK_DIR}/build.log"
STATUS_LOG="${WORK_DIR}/status.log"

report_failure() {
    local stage="$1"
    echo "cpp-std-complex compile check failed at stage: ${stage}" >&2
    echo "manifest: ${MANIFEST_PATH}" >&2
    echo "index: ${INDEX_PATH}" >&2
    echo "transpile.log: ${TRANSPILE_LOG}" >&2
    echo "build.log: ${BUILD_LOG}" >&2
    echo "transpiled.cppm: ${TARGET_CPPM_PATH}" >&2
    echo "rusty-runtime-module.cppm: ${RUSTY_RUNTIME_MODULE_PATH}" >&2
}

echo "═══════════════════════════════════════════════════════════════════════"
echo "cpp::std complex compile check"
echo "  manifest: ${MANIFEST_PATH}"
echo "  index: ${INDEX_PATH}"
echo "  work dir: ${WORK_DIR}"
if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "  mode: dry-run"
fi
echo "═══════════════════════════════════════════════════════════════════════"

PARITY_CMD=(
    cargo
    run
    -p
    rusty-cpp-transpiler
    --
    parity-test
    --manifest-path
    "${MANIFEST_PATH}"
    --no-baseline
    --stop-after
    transpile
    --cpp-module-index
    "${INDEX_PATH}"
    --work-dir
    "${WORK_DIR}"
)

if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "[dry-run] ${PARITY_CMD[*]}"
    echo "[dry-run] g++ -std=c++23 -fmodules-ts -I ${REPO_ROOT}/include -x c++ -c ${RUSTY_RUNTIME_MODULE_PATH} -o rusty.runtime.o"
    echo "[dry-run] g++ -std=c++23 -fmodules-ts -I ${REPO_ROOT}/include -x c++ -c ${TARGET_CPPM_PATH} -o cpp_std_complex.o"
    exit 0
fi

rm -rf "${WORK_DIR}"
mkdir -p "${WORK_DIR}"
cd "${REPO_ROOT}"

if ! "${PARITY_CMD[@]}" >"${TRANSPILE_LOG}" 2>&1; then
    report_failure "transpile"
    echo "tail of transpile log:" >&2
    tail -n 80 "${TRANSPILE_LOG}" >&2 || true
    exit 1
fi

if [[ ! -f "${TARGET_CPPM_PATH}" ]]; then
    report_failure "transpile-output"
    echo "missing transpiled output: ${TARGET_CPPM_PATH}" >&2
    exit 1
fi

: >"${BUILD_LOG}"
echo "compiler probe for import std support" >>"${BUILD_LOG}"

SUPPORTED_COMPILER=""
SUPPORTED_FLAGS=""
PROBE_SOURCE="${WORK_DIR}/import_std_probe.cppm"
cat >"${PROBE_SOURCE}" <<'PROBE'
export module import_std_probe;
import std;
export int probe_value() { return 1; }
PROBE

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
    echo "SKIP: no detected compiler with working 'import std' module support" | tee -a "${STATUS_LOG}"
    echo "Details in ${BUILD_LOG}"
    exit 0
fi

echo "using compiler: ${SUPPORTED_COMPILER}" | tee -a "${STATUS_LOG}"
echo "compiler flags: ${SUPPORTED_FLAGS}" | tee -a "${STATUS_LOG}"

IFS=' ' read -r -a ACTIVE_FLAGS <<<"${SUPPORTED_FLAGS}"

if ! (
    cd "${WORK_DIR}" &&
    "${SUPPORTED_COMPILER}" "${ACTIVE_FLAGS[@]}" -I "${REPO_ROOT}/include" -x c++ -c "${RUSTY_RUNTIME_MODULE_PATH}" -o rusty.runtime.o
) >>"${BUILD_LOG}" 2>&1; then
    report_failure "compile-rusty-runtime-module"
    tail -n 80 "${BUILD_LOG}" >&2 || true
    exit 1
fi

if ! (
    cd "${WORK_DIR}" &&
    "${SUPPORTED_COMPILER}" "${ACTIVE_FLAGS[@]}" -I "${REPO_ROOT}/include" -x c++ -c "${TARGET_CPPM_PATH}" -o cpp_std_complex.o
) >>"${BUILD_LOG}" 2>&1; then
    report_failure "compile-transpiled-module"
    tail -n 80 "${BUILD_LOG}" >&2 || true
    exit 1
fi

echo "PASS: cpp::std complex module compile check succeeded"
echo "build.log: ${BUILD_LOG}"
