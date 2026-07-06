#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# The transpiler binary the per-crate runs invoke. Defaults to the release
# build; overridable via RUSTY_CPP_TRANSPILER_BIN so callers (e.g. the harness
# test) can inject a stub. When overridden, the pre-build is skipped — the
# caller supplies a ready binary.
TRANSPILER_BIN="${RUSTY_CPP_TRANSPILER_BIN:-${REPO_ROOT}/target/release/rusty-cpp-transpiler}"

declare -a MATRIX_CRATES=(
    "either"
    "tap"
    "cfg-if"
    "take_mut"
    "arrayvec"
    "semver"
    "bitflags"
    "smallvec"
    # vec: a committed local (non-cloned) crate — hand-written focused Vec
    # parity tests. Its source lives in tests/transpile_tests/vec/ (tracked via
    # .gitignore exceptions), so ensure_crate_checkout skips the clone.
    "vec"
    # btree: committed local crate like `vec` — focused BTreeMap/BTreeSet
    # parity tests (ordered iteration, ranges, entry API, set algebra).
    "btree"
    # itertools: temporarily disabled — iterator-item engine + FoldWhile_Ok done
    # (quick leak-free), remaining = tree_reduce alias-hoist (UFCS path, collision-
    # safe) + projected-push transpile leaks. Re-enable when those land.
    # "itertools"
    "once_cell"
    "serde_bytes"
    "serde_repr"
    "serde_core"
    "serde"
    "pollster"
    # indexmap + hashbrown: serde_yaml's transitive deps, added as standalone
    # matrix crates so their codegen is exercised in isolation (fast, focused
    # feedback vs. only surfacing buried in the serde_yaml build). Versions
    # match what serde_yaml resolves so fixes transfer. KNOWN_FAIL until clear.
    "hashbrown"
    "indexmap"
    # serde_yaml: disabled from the default matrix — its compile gate is the
    # unsafe_libyaml raw-pointer C-port long tail, not the serde layer (see
    # memory serde-yaml-translation-coverage). Run manually with
    # `--crate serde_yaml`; the clone-URL/version tables below stay for that.
    # "serde_yaml"
)

# Crates known to FAIL for reasons unrelated to (and predating) the UFCS
# migration — they fail flag-OFF too, so they cannot be "regressed" by the
# default flip. A failure here is reported but does NOT fail the matrix (it is
# tallied as KNOWN-FAIL, not FAIL). itertools: a flag-independent transpile
# panic on the Vec<_>::new() element-inference gap + a multi-session
# Itertools-default-body long tail (see memory itertools-codegen-remaining).
declare -a KNOWN_FAIL_CRATES=(
    "itertools"
    "serde_yaml"
    "hashbrown"
    "indexmap"
)

is_known_fail() {
    local needle="$1"
    local crate
    for crate in "${KNOWN_FAIL_CRATES[@]}"; do
        if [[ "${crate}" == "${needle}" ]]; then
            return 0
        fi
    done
    return 1
}

declare -A CRATE_REPO=(
    ["either"]="https://github.com/rayon-rs/either.git"
    ["tap"]="https://github.com/myrrlyn/tap.git"
    ["cfg-if"]="https://github.com/alexcrichton/cfg-if.git"
    ["take_mut"]="https://github.com/Sgeo/take_mut.git"
    ["arrayvec"]="https://github.com/bluss/arrayvec.git"
    ["semver"]="https://github.com/dtolnay/semver.git"
    ["bitflags"]="https://github.com/bitflags/bitflags.git"
    ["smallvec"]="https://github.com/servo/rust-smallvec.git"
    ["itertools"]="https://github.com/rust-itertools/itertools.git"
    ["once_cell"]="https://github.com/matklad/once_cell.git"
    ["serde_bytes"]="https://github.com/serde-rs/bytes.git"
    ["serde_repr"]="https://github.com/dtolnay/serde-repr.git"
    ["serde_core"]="https://github.com/serde-rs/serde.git"
    ["serde"]="https://github.com/serde-rs/serde.git"
    ["pollster"]="https://github.com/zesterer/pollster.git"
    ["serde_yaml"]="https://github.com/dtolnay/serde-yaml.git"
    ["hashbrown"]="https://github.com/rust-lang/hashbrown.git"
    ["indexmap"]="https://github.com/indexmap-rs/indexmap.git"
)

declare -A CRATE_REF=(
    ["either"]="1.13.0"
    ["tap"]="0.4.0"
    ["cfg-if"]="1.0.0"
    ["take_mut"]="v0.2.2"
    ["arrayvec"]="0.7.6"
    ["semver"]="1.0.24"
    ["bitflags"]="2.6.0"
    ["smallvec"]="v1.15.1"
    ["itertools"]="v0.14.0"
    ["once_cell"]="v1.21.4"
    ["serde_bytes"]="0.11.19"
    ["serde_repr"]="0.1.20"
    ["serde_core"]="v1.0.228"
    ["serde"]="v1.0.228"
    ["pollster"]="master"
    ["serde_yaml"]="0.9.34"
    # Match the versions serde_yaml resolves (indexmap 2.14.0 -> hashbrown
    # 0.17.1 + equivalent 1.0.2) so standalone fixes transfer to serde_yaml.
    ["hashbrown"]="v0.17.1"
    ["indexmap"]="2.14.0"
)

declare -A CRATE_MANIFEST_REL=(
    ["serde_core"]="serde_core/Cargo.toml"
    ["serde"]="serde/Cargo.toml"
)

TARGET_CRATE=""
WORK_ROOT="${REPO_ROOT}/.rusty-parity-matrix"
DRY_RUN=0
KEEP_WORK_DIRS=0
IMPORT_STD=0
PREFER_RUSTY_UNIT=0
PREFER_RUSTY_VIEWS=0
CONTINUE_ON_FAIL=0
# Content-addressed module BMI/object cache + shared CARGO_TARGET_DIR (enables
# RUSTY_CPP_MODULE_CACHE for the per-crate parity-test invocations). OPT-IN via
# --cache. The caches are content-addressed (BMI keyed on the .cppm bytes +
# transitive imports + clang/flags + the TRANSPILER REVISION (git hash + dirty +
# binary mtime); cargo target deduped by cargo's own fingerprint), so a crate
# whose transpiled output CHANGES — or a transpiler rebuild that could re-emit it —
# gets a cache miss and is rebuilt, never a stale artifact. Default OFF; win modest:
# the caches skip the cargo stages + the C++ build, but the (uncached) Stage-C
# transpile dominates, so a warm run is only ~10% faster than cold (16-crate
# matrix: ~21 vs ~23.5 min), while the cold/first run pays cargo-lock
# serialization + ~4.5 GB of cache population. Worth --cache for a fast iterate
# loop with LOCALIZED transpiler changes (only the touched crate rebuilds C++).
MODULE_CACHE=0
# Cross-crate parallelism. Crates are independent (separate work dirs + dep
# graphs), so they can build concurrently. With JOBS>1 the binary is pre-built
# once (so concurrent runs don't serialize on cargo's build lock), each crate
# runs with TMPDIR under its own work dir, and all crates run regardless of
# failures (parallel can't cleanly abort peers).
#
# Default ≈ 60% of cores. Note this is the *crate* concurrency, which is
# MEMORY-binding: the serde-family crates precompile GB-scale modules, so on a
# memory-constrained host the heavy crates can OOM at a high --jobs — lower it
# (e.g. --jobs 3) if so. `--jobs 1` restores the exact sequential legacy path.
# Keep --work-root on a roomy filesystem (not a small tmpfs).
_matrix_cores="$(nproc 2>/dev/null || echo 4)"
JOBS=$(( (_matrix_cores * 6 + 5) / 10 ))
[[ "${JOBS}" -lt 1 ]] && JOBS=1
FIRST_FAIL_CRATE=""
FIRST_FAIL_WORK_DIR=""
FIRST_FAIL_LOG=""

print_usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Run parity matrix across crate set:
  either, tap, cfg-if, take_mut, arrayvec, semver, bitflags, smallvec, itertools, once_cell, serde_bytes, serde_repr, serde_core, serde, pollster

Options:
  --crate <name>      Run only one matrix crate
  --work-root <dir>   Root directory for per-crate parity work dirs
  --keep-work-dirs    Keep/reuse existing per-crate work dirs
  --jobs <N>          Build N crates concurrently (default ≈60% of cores). N>1
                      pre-builds the binary once and runs all crates (keep
                      --work-root on a roomy filesystem; TMPDIR is under each
                      crate's work dir). Crate concurrency is memory-binding —
                      lower (e.g. --jobs 3) if the serde-family crates OOM;
                      --jobs 1 is the sequential legacy path.
  --import-std       Use parity import-std mode (emit import std; and libc++ std module precompile)
  --cache / --no-cache  Enable/disable the content-addressed module BMI/object cache
                      + shared CARGO_TARGET_DIR (default: OFF). Content-keyed, so a
                      changed crate is always rebuilt. ~10% faster warm; best for a
                      fast iterate loop with localized transpiler changes.
  --prefer-rusty-unit  Prefer rusty::Unit spelling in generated output
  --prefer-rusty-views  Prefer rusty::StrView / rusty::Span spellings in generated output
  --continue-on-fail  Continue running all crates even after failures
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
        --jobs)
            if [[ $# -lt 2 ]]; then
                echo "error: --jobs requires a value" >&2
                exit 2
            fi
            if ! [[ "$2" =~ ^[1-9][0-9]*$ ]]; then
                echo "error: --jobs requires a positive integer" >&2
                exit 2
            fi
            JOBS="$2"
            shift 2
            ;;
        --import-std)
            IMPORT_STD=1
            shift
            ;;
        --no-cache)
            MODULE_CACHE=0
            shift
            ;;
        --cache)
            MODULE_CACHE=1
            shift
            ;;
        --prefer-rusty-unit)
            PREFER_RUSTY_UNIT=1
            shift
            ;;
        --prefer-rusty-views)
            PREFER_RUSTY_VIEWS=1
            shift
            ;;
        --continue-on-fail)
            CONTINUE_ON_FAIL=1
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

# Enable the content-addressed module cache + shared cargo target for every
# per-crate parity-test invocation (unless an outer env already set it, or
# --no-cache turned it off). Respects a pre-set RUSTY_CPP_MODULE_CACHE.
if [[ "${MODULE_CACHE}" -eq 1 && -z "${RUSTY_CPP_MODULE_CACHE:-}" ]]; then
    export RUSTY_CPP_MODULE_CACHE=1
fi

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
    local work_dir="${WORK_ROOT}/${crate}"

    local manifest_rel="${CRATE_MANIFEST_REL[${crate}]:-Cargo.toml}"
    if [[ -f "${crate_dir}/${manifest_rel}" ]]; then
        return 0
    fi

    # Only registry crates need a repo/ref (local crates returned above).
    # Access these AFTER the early return so `set -u` doesn't trip on a
    # local-only crate (e.g. `vec`) that has no CRATE_REPO entry.
    local repo="${CRATE_REPO[${crate}]}"
    local ref="${CRATE_REF[${crate}]}"

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

    # Per-crate post-clone prep. Kept OUTSIDE the crate dir (the clone above
    # wipes it) under crate_preps/<crate>.sh, invoked with the crate dir. Used
    # to trim untranspilable test targets — e.g. indexmap/hashbrown drop their
    # quickcheck/rand tests, whose getrandom -> libc (raw C FFI) dependency
    # chain cannot be transpiled.
    local prep_script="${SCRIPT_DIR}/crate_preps/${crate}.sh"
    if [[ -f "${prep_script}" ]]; then
        echo "  Applying ${crate} prep..."
        bash "${prep_script}" "${crate_dir}"
    fi
}

run_parity_for_crate() {
    local crate="$1"
    local crate_dir="${SCRIPT_DIR}/${crate}"
    local manifest_rel="${CRATE_MANIFEST_REL[${crate}]:-Cargo.toml}"
    local manifest="${crate_dir}/${manifest_rel}"
    local work_dir="${WORK_ROOT}/${crate}"
    local matrix_log="${work_dir}/matrix.log"
    if [[ "${DRY_RUN}" -eq 1 ]]; then
        echo "crate: ${crate}"
        echo "  manifest: ${manifest}"
        local dry_cmd="cargo run --release -p rusty-cpp-transpiler -- parity-test --manifest-path ${manifest} --stop-after run --work-dir ${work_dir}"
        if [[ "${IMPORT_STD}" -eq 1 ]]; then
            dry_cmd="${dry_cmd} --import-std"
        fi
        if [[ "${PREFER_RUSTY_UNIT}" -eq 1 ]]; then
            dry_cmd="${dry_cmd} --prefer-rusty-unit-alias"
        fi
        if [[ "${PREFER_RUSTY_VIEWS}" -eq 1 ]]; then
            dry_cmd="${dry_cmd} --prefer-rusty-view-aliases"
        fi
        echo "  command: ${dry_cmd}"
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

    # `--release` cuts transpile-time by ~2.7× on the serde-family crates
    # (debug serde_repr: 14m28s; release: 5m24s). The default 1800s matrix
    # timeout couldn't fit the larger crates in debug mode and they were
    # killed before Stage D; release lets them complete end-to-end.
    #
    # Invoke the pre-built binary directly (not `cargo run`): with --jobs N>1,
    # concurrent `cargo run` calls would serialize on cargo's build lock even
    # when the binary is current. The binary is pre-built once before the loop.
    local -a cmd=(
        "${TRANSPILER_BIN}"
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
    if [[ "${IMPORT_STD}" -eq 1 ]]; then
        cmd+=(--import-std)
    fi
    if [[ "${PREFER_RUSTY_UNIT}" -eq 1 ]]; then
        cmd+=(--prefer-rusty-unit-alias)
    fi
    if [[ "${PREFER_RUSTY_VIEWS}" -eq 1 ]]; then
        cmd+=(--prefer-rusty-view-aliases)
    fi

    echo "crate: ${crate}"
    echo "  manifest: ${manifest}"
    echo "  command: ${cmd[*]}"

    # Keep each crate's intermediate/scratch files (clang temporaries, etc.)
    # under its own work dir rather than the shared /tmp tmpfs, so concurrent
    # crates (--jobs N>1) don't exhaust tmpfs.
    local crate_tmp="${work_dir}/.tmp"
    mkdir -p "${crate_tmp}"

    if TMPDIR="${crate_tmp}" "${cmd[@]}" >"${matrix_log}" 2>&1; then
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
KNOWN_FAIL=0

echo "═══════════════════════════════════════════════════════════════════════"
echo "Parity Matrix"
echo "  crates: $(join_crates "${CRATES_TO_RUN[@]}")"
echo "  work root: ${WORK_ROOT}"
if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "  mode: dry-run"
fi
if [[ "${CONTINUE_ON_FAIL}" -eq 1 ]]; then
    echo "  continue-on-fail: yes"
fi
if [[ "${JOBS}" -gt 1 ]]; then
    echo "  jobs: ${JOBS} (parallel; runs every crate regardless of failures)"
fi
echo "═══════════════════════════════════════════════════════════════════════"

# Pre-build the binary once so the per-crate runs invoke a ready binary
# directly (run_parity_for_crate calls target/release/... not `cargo run`),
# which is required for --jobs N>1 (concurrent `cargo run` would serialize on
# cargo's build lock).
if [[ "${DRY_RUN}" -eq 0 && -z "${RUSTY_CPP_TRANSPILER_BIN:-}" ]]; then
    echo "Pre-building rusty-cpp-transpiler (release)..."
    if ! cargo build --release -p rusty-cpp-transpiler; then
        echo "error: pre-build of rusty-cpp-transpiler failed" >&2
        exit 1
    fi
fi

if [[ "${JOBS}" -gt 1 && "${DRY_RUN}" -eq 0 ]]; then
    # Parallel: fan out crates behind a `wait -n` semaphore, record each
    # crate's PASS/FAIL to a result file, then tally. Parallel mode always runs
    # every crate (it can't cleanly abort in-flight peers), so it behaves as
    # --continue-on-fail. Per-crate stdout interleaves; each crate's clean log
    # is still at <work-dir>/matrix.log.
    results_dir="${WORK_ROOT}/.matrix-results"
    rm -rf "${results_dir}"
    mkdir -p "${results_dir}"
    running=0
    for crate in "${CRATES_TO_RUN[@]}"; do
        (
            echo ""
            echo "━━ ${crate} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            if ensure_crate_checkout "${crate}" && run_parity_for_crate "${crate}"; then
                echo "PASS" >"${results_dir}/${crate}"
            else
                echo "FAIL" >"${results_dir}/${crate}"
            fi
        ) &
        running=$((running + 1))
        if [[ "${running}" -ge "${JOBS}" ]]; then
            wait -n
            running=$((running - 1))
        fi
    done
    wait
    for crate in "${CRATES_TO_RUN[@]}"; do
        TOTAL=$((TOTAL + 1))
        if [[ "$(cat "${results_dir}/${crate}" 2>/dev/null)" == "PASS" ]]; then
            PASS=$((PASS + 1))
        elif is_known_fail "${crate}"; then
            KNOWN_FAIL=$((KNOWN_FAIL + 1))
            echo "  KNOWN-FAIL (not a regression): ${crate}" >&2
        else
            FAIL=$((FAIL + 1))
            # run_parity_for_crate ran in a subshell, so its
            # record_first_failure globals never reached us. Reconstruct the
            # first failure (in crate order) from the result files so the
            # final summary still reports artifact paths in parallel mode.
            record_first_failure \
                "${crate}" \
                "${WORK_ROOT}/${crate}" \
                "${WORK_ROOT}/${crate}/matrix.log"
        fi
    done
else
    for crate in "${CRATES_TO_RUN[@]}"; do
        TOTAL=$((TOTAL + 1))
        echo ""
        echo "━━ ${crate} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

        if ! ensure_crate_checkout "${crate}"; then
            FAIL=$((FAIL + 1))
            if [[ "${CONTINUE_ON_FAIL}" -eq 0 ]]; then
                break
            fi
            continue
        fi
        if run_parity_for_crate "${crate}"; then
            PASS=$((PASS + 1))
        elif is_known_fail "${crate}"; then
            KNOWN_FAIL=$((KNOWN_FAIL + 1))
            echo "  KNOWN-FAIL (not a regression): ${crate}" >&2
        else
            FAIL=$((FAIL + 1))
            if [[ "${CONTINUE_ON_FAIL}" -eq 0 ]]; then
                break
            fi
        fi
    done
fi

echo ""
echo "═══════════════════════════════════════════════════════════════════════"
printf "Summary: total=%d pass=%d fail=%d known-fail=%d\n" "${TOTAL}" "${PASS}" "${FAIL}" "${KNOWN_FAIL}"
if [[ "${KNOWN_FAIL}" -gt 0 ]]; then
    printf "Known-fail crates (pre-existing, not regressions): %s\n" "$(join_crates "${KNOWN_FAIL_CRATES[@]}")"
fi
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
