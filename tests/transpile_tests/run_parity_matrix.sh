#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

declare -a MATRIX_CRATES=(
    "either"
    "tap"
    "cfg-if"
    "take_mut"
    "arrayvec"
    "semver"
    "bitflags"
)

declare -A CRATE_REPO=(
    ["either"]="https://github.com/rayon-rs/either.git"
    ["tap"]="https://github.com/myrrlyn/tap.git"
    ["cfg-if"]="https://github.com/alexcrichton/cfg-if.git"
    ["take_mut"]="https://github.com/Sgeo/take_mut.git"
    ["arrayvec"]="https://github.com/bluss/arrayvec.git"
    ["semver"]="https://github.com/dtolnay/semver.git"
    ["bitflags"]="https://github.com/bitflags/bitflags.git"
)

declare -A CRATE_REF=(
    ["either"]="1.13.0"
    ["tap"]="0.4.0"
    ["cfg-if"]="1.0.0"
    ["take_mut"]="v0.2.2"
    ["arrayvec"]="0.7.6"
    ["semver"]="1.0.24"
    ["bitflags"]="2.6.0"
)

TARGET_CRATE=""
WORK_ROOT="${REPO_ROOT}/.rusty-parity-matrix"
DRY_RUN=0
KEEP_WORK_DIRS=0
FIRST_FAIL_CRATE=""
FIRST_FAIL_WORK_DIR=""
FIRST_FAIL_LOG=""

print_usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Run parity matrix across crate set:
  either, tap, cfg-if, take_mut, arrayvec, semver, bitflags

Options:
  --crate <name>      Run only one matrix crate
  --work-root <dir>   Root directory for per-crate parity work dirs
  --keep-work-dirs    Keep/reuse existing per-crate work dirs
  --dry-run           Print planned commands without executing
  --help              Show this help
EOF
}

is_known_crate() {
    local needle="$1"
    local crate
    for crate in "${MATRIX_CRATES[@]}"; do
        if [[ "${crate}" == "${needle}" ]]; then
            return 0
        fi
    done
    return 1
}

join_crates() {
    local IFS=", "
    echo "$*"
}

record_first_failure() {
    local crate="$1"
    local work_dir="$2"
    local matrix_log="$3"

    if [[ -n "${FIRST_FAIL_CRATE}" ]]; then
        return 0
    fi

    FIRST_FAIL_CRATE="${crate}"
    FIRST_FAIL_WORK_DIR="${work_dir}"
    FIRST_FAIL_LOG="${matrix_log}"
}

print_failure_diagnostics() {
    local crate="$1"
    local work_dir="$2"
    local matrix_log="$3"

    record_first_failure "${crate}" "${work_dir}" "${matrix_log}"

    echo "  FAIL: ${crate}" >&2
    echo "  first failing crate: ${crate}" >&2
    if [[ -n "${matrix_log}" ]]; then
        echo "  matrix log: ${matrix_log}" >&2
    fi
    echo "  baseline artifact: ${work_dir}/baseline.txt" >&2
    echo "  build artifact: ${work_dir}/build.log" >&2
    echo "  run artifact: ${work_dir}/run.log" >&2
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --crate)
            if [[ $# -lt 2 ]]; then
                echo "error: --crate requires a value" >&2
                exit 2
            fi
            if ! is_known_crate "$2"; then
                echo "error: unknown matrix crate '$2'" >&2
                echo "known crates: $(join_crates "${MATRIX_CRATES[@]}")" >&2
                exit 2
            fi
            TARGET_CRATE="$2"
            shift 2
            ;;
        --work-root)
            if [[ $# -lt 2 ]]; then
                echo "error: --work-root requires a value" >&2
                exit 2
            fi
            WORK_ROOT="$2"
            shift 2
            ;;
        --keep-work-dirs)
            KEEP_WORK_DIRS=1
            shift
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

cd "${REPO_ROOT}"
mkdir -p "${WORK_ROOT}"

if [[ -n "${TARGET_CRATE}" ]]; then
    CRATES_TO_RUN=("${TARGET_CRATE}")
else
    CRATES_TO_RUN=("${MATRIX_CRATES[@]}")
fi

ensure_crate_checkout() {
    local crate="$1"
    local crate_dir="${SCRIPT_DIR}/${crate}"
    local repo="${CRATE_REPO[${crate}]}"
    local ref="${CRATE_REF[${crate}]}"
    local work_dir="${WORK_ROOT}/${crate}"

    if [[ -f "${crate_dir}/Cargo.toml" ]]; then
        return 0
    fi

    if [[ "${DRY_RUN}" -eq 1 ]]; then
        echo "  [dry-run] git clone --depth 1 --branch ${ref} ${repo} ${crate_dir}"
        return 0
    fi

    if [[ -d "${crate_dir}" ]]; then
        rm -r "${crate_dir}"
    fi

    echo "  Cloning ${crate} (${ref})..."
    if ! git clone --depth 1 --branch "${ref}" "${repo}" "${crate_dir}"; then
        print_failure_diagnostics "${crate}" "${work_dir}" ""
        return 1
    fi
}

run_parity_for_crate() {
    local crate="$1"
    local crate_dir="${SCRIPT_DIR}/${crate}"
    local manifest="${crate_dir}/Cargo.toml"
    local work_dir="${WORK_ROOT}/${crate}"
    local matrix_log="${work_dir}/matrix.log"

    if [[ "${DRY_RUN}" -eq 1 ]]; then
        echo "crate: ${crate}"
        echo "  manifest: ${manifest}"
        echo "  command: cargo run -p rusty-cpp-transpiler -- parity-test --manifest-path ${manifest} --stop-after run --work-dir ${work_dir}"
        return 0
    fi

    if [[ ! -f "${manifest}" ]]; then
        print_failure_diagnostics "${crate}" "${work_dir}" "${matrix_log}"
        echo "  missing manifest: ${manifest}" >&2
        return 1
    fi

    if [[ -d "${work_dir}" && "${KEEP_WORK_DIRS}" -eq 0 ]]; then
        rm -r "${work_dir}"
    fi
    mkdir -p "${work_dir}"

    local -a cmd=(
        cargo
        run
        -p
        rusty-cpp-transpiler
        --
        parity-test
        --manifest-path
        "${manifest}"
        --stop-after
        run
        --work-dir
        "${work_dir}"
    )
    if [[ "${KEEP_WORK_DIRS}" -eq 1 ]]; then
        cmd+=(--keep-work-dir)
    fi

    echo "crate: ${crate}"
    echo "  manifest: ${manifest}"
    echo "  command: ${cmd[*]}"

    if "${cmd[@]}" >"${matrix_log}" 2>&1; then
        echo "  PASS: ${crate}"
        return 0
    fi

    print_failure_diagnostics "${crate}" "${work_dir}" "${matrix_log}"
    echo "  tail of matrix log:" >&2
    tail -n 50 "${matrix_log}" >&2 || true
    return 1
}

TOTAL=0
PASS=0
FAIL=0

echo "═══════════════════════════════════════════════════════════════════════"
echo "Parity Matrix"
echo "  crates: $(join_crates "${CRATES_TO_RUN[@]}")"
echo "  work root: ${WORK_ROOT}"
if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "  mode: dry-run"
fi
echo "═══════════════════════════════════════════════════════════════════════"

for crate in "${CRATES_TO_RUN[@]}"; do
    TOTAL=$((TOTAL + 1))
    echo ""
    echo "━━ ${crate} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if ! ensure_crate_checkout "${crate}"; then
        FAIL=$((FAIL + 1))
        break
    fi
    if run_parity_for_crate "${crate}"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        break
    fi
done

echo ""
echo "═══════════════════════════════════════════════════════════════════════"
printf "Summary: total=%d pass=%d fail=%d\n" "${TOTAL}" "${PASS}" "${FAIL}"
echo "═══════════════════════════════════════════════════════════════════════"

if [[ "${FAIL}" -gt 0 ]]; then
    if [[ -n "${FIRST_FAIL_CRATE}" ]]; then
        echo "First failing crate: ${FIRST_FAIL_CRATE}" >&2
        if [[ -n "${FIRST_FAIL_LOG}" ]]; then
            echo "Failure log: ${FIRST_FAIL_LOG}" >&2
        fi
        echo "baseline.txt: ${FIRST_FAIL_WORK_DIR}/baseline.txt" >&2
        echo "build.log: ${FIRST_FAIL_WORK_DIR}/build.log" >&2
        echo "run.log: ${FIRST_FAIL_WORK_DIR}/run.log" >&2
    fi
    exit 1
fi
