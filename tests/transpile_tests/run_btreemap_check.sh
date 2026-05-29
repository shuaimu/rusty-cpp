#!/usr/bin/env bash
# End-to-end check for tests/transpile_tests/btreemap.
#
# Asserts that:
#   1. The crate transpiles cleanly to a single .cppm module.
#   2. After re-annotating the three exported fixture functions as `// @safe`,
#      rusty-cpp-checker exits cleanly with no violations.
#
# Usage: ./run_btreemap_check.sh [--work-dir <dir>] [--dry-run]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

WORK_DIR="${REPO_ROOT}/.rusty-btreemap-check"
DRY_RUN=0

print_usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --work-dir <dir>  Working directory for transpile + check artifacts
                    (default: ${REPO_ROOT}/.rusty-btreemap-check)
  --dry-run         Print planned commands without executing
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

MANIFEST_PATH="${SCRIPT_DIR}/btreemap/Cargo.toml"
OUT_DIR="${WORK_DIR}/btreemap_cpp_out"
CPPM_PATH="${OUT_DIR}/btreemap_fixture.cppm"
CHECK_INPUT="${OUT_DIR}/btreemap_fixture_safe.cpp"
CHECK_LOG="${WORK_DIR}/check.log"

run() {
    if [[ "${DRY_RUN}" == "1" ]]; then
        printf '+ %s\n' "$*"
    else
        "$@"
    fi
}

mkdir -p "${WORK_DIR}"
rm -rf "${OUT_DIR}"

echo "[1/4] Building binaries"
run cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" -p rusty-cpp-transpiler
run cargo build --manifest-path "${REPO_ROOT}/Cargo.toml" --bin rusty-cpp-checker

TRANSPILER="${REPO_ROOT}/target/debug/rusty-cpp-transpiler"
CHECKER="${REPO_ROOT}/target/debug/rusty-cpp-checker"

echo "[2/4] Transpiling tests/transpile_tests/btreemap"
run "${TRANSPILER}" --crate "${MANIFEST_PATH}" --output-dir "${OUT_DIR}"

if [[ "${DRY_RUN}" == "0" && ! -f "${CPPM_PATH}" ]]; then
    echo "error: expected ${CPPM_PATH} to be generated" >&2
    exit 1
fi

echo "[3/4] Injecting // @safe annotations on the fixture functions"
if [[ "${DRY_RUN}" == "0" ]]; then
    sed -E \
        's|^export int32_t (insert_then_get_present\|insert_then_get_missing\|insert_returns_old)\(\) \{|// @safe\nexport int32_t \1() {|' \
        "${CPPM_PATH}" > "${CHECK_INPUT}"
fi

echo "[4/4] Running rusty-cpp-checker on the annotated fixture"
if [[ "${DRY_RUN}" == "0" ]]; then
    set +e
    "${CHECKER}" "${CHECK_INPUT}" -I "${REPO_ROOT}/include" > "${CHECK_LOG}" 2>&1
    CHECK_STATUS=$?
    set -e
else
    run "${CHECKER}" "${CHECK_INPUT}" -I "${REPO_ROOT}/include"
fi

if [[ "${DRY_RUN}" == "1" ]]; then
    echo "Dry run complete."
    exit 0
fi

if grep -q '@unsafe' "${CHECK_INPUT}"; then
    echo "FAIL: annotated fixture unexpectedly contains @unsafe" >&2
    echo "Input: ${CHECK_INPUT}" >&2
    exit 1
fi

if [[ "${CHECK_STATUS}" -ne 0 ]]; then
    echo "FAIL: rusty-cpp-checker reported violations:" >&2
    tail -n 80 "${CHECK_LOG}" >&2 || true
    echo "" >&2
    echo "Full log: ${CHECK_LOG}" >&2
    exit 1
fi

# Sanity-check: assert the suppressed wrong-return-type error from the
# pre-fix transpiler is gone. That warning was emitted by clang when the
# fixture lowered `m.get(&1)` to the slice-style `rusty::get(m, 1)` and
# tried to return a `BTreeMap<int,int>` value from an `int32_t` function.
SUPPRESSED=$(grep -E 'Warning \(suppressed error\):.*BTreeMap' "${CHECK_LOG}" || true)
if [[ -n "${SUPPRESSED}" ]]; then
    echo "FAIL: suppressed return-type error reappeared:" >&2
    echo "${SUPPRESSED}" >&2
    exit 1
fi

echo "PASS: BTreeMap insert/get fixture is checker-clean under // @safe"
