#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: run_parity_harness.sh [options]

Automates the Phase 18 end-to-end parity workflow for tests/transpile_tests/either:
1) Rust baseline: cargo test
2) Transpile:
   - crate output via rusty-cpp-transpiler --crate --expand
   - expanded tests output via cargo expand --lib --tests + single-file transpile
3) C++ build:
   - compile generated either.cppm
   - compile generated either_expanded_tests.cppm
4) C++ run: run generated expanded-test wrapper executable

Options:
  --work-dir <path>      Directory for generated artifacts/logs (default: mktemp)
  --keep-work-dir        Keep auto-created work directory even on success
  --dry-run              Print commands only, do not execute
  --stop-after <stage>   Stop after stage: baseline | transpile | build | run
  -h, --help             Show this help
EOF
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
EITHER_MANIFEST="${SCRIPT_DIR}/Cargo.toml"

WORK_DIR=""
KEEP_WORK_DIR=0
DRY_RUN=0
STOP_AFTER=""
AUTO_WORK_DIR=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --work-dir)
      shift
      if [[ $# -eq 0 ]]; then
        echo "error: --work-dir requires a value" >&2
        usage
        exit 2
      fi
      WORK_DIR="$1"
      ;;
    --keep-work-dir)
      KEEP_WORK_DIR=1
      ;;
    --dry-run)
      DRY_RUN=1
      ;;
    --stop-after)
      shift
      if [[ $# -eq 0 ]]; then
        echo "error: --stop-after requires a value" >&2
        usage
        exit 2
      fi
      STOP_AFTER="$1"
      case "${STOP_AFTER}" in
        baseline|transpile|build|run) ;;
        *)
          echo "error: invalid --stop-after value '${STOP_AFTER}'" >&2
          usage
          exit 2
          ;;
      esac
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument '$1'" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

if [[ -z "${WORK_DIR}" ]]; then
  WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/either-parity.XXXXXX")"
  AUTO_WORK_DIR=1
else
  mkdir -p "${WORK_DIR}"
fi

LOG_RUST="${WORK_DIR}/rust_cargo_test.log"
LOG_TRANSPILE="${WORK_DIR}/transpile.log"
LOG_CPP_BUILD="${WORK_DIR}/cpp_build.log"
LOG_CPP_RUN="${WORK_DIR}/cpp_run.log"
CPP_OUT_DIR="${WORK_DIR}/cpp_out"
MODULE_OBJ="${WORK_DIR}/either.o"
EXPANDED_TESTS_RS="${WORK_DIR}/either_expanded_tests.rs"
EXPANDED_TESTS_CPPM="${CPP_OUT_DIR}/either_expanded_tests.cppm"
EXPANDED_TESTS_OBJ="${WORK_DIR}/either_expanded_tests.o"
TEST_RUNNER_MAIN="${WORK_DIR}/either_expanded_tests_main.cpp"
TEST_RUNNER_BIN="${WORK_DIR}/either_expanded_tests_runner"

cleanup() {
  local exit_code=$?
  if [[ "${AUTO_WORK_DIR}" -eq 1 && "${KEEP_WORK_DIR}" -eq 0 && "${DRY_RUN}" -eq 0 && ${exit_code} -eq 0 ]]; then
    rm -rf "${WORK_DIR}"
    echo "Harness succeeded (temporary work dir removed)." >&2
  else
    echo "Harness artifacts: ${WORK_DIR}" >&2
  fi
}
trap cleanup EXIT

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command not found: $1" >&2
    exit 1
  fi
}

reset_artifacts() {
  # Make repeated runs with the same --work-dir deterministic by clearing stale
  # logs and outputs up front. This prevents false-green runs from leftovers.
  rm -rf "${CPP_OUT_DIR}"
  mkdir -p "${CPP_OUT_DIR}"

  : > "${LOG_RUST}"
  : > "${LOG_TRANSPILE}"
  : > "${LOG_CPP_BUILD}"
  : > "${LOG_CPP_RUN}"

  rm -f "${MODULE_OBJ}" "${EXPANDED_TESTS_RS}" "${EXPANDED_TESTS_OBJ}" "${TEST_RUNNER_MAIN}" "${TEST_RUNNER_BIN}"
}

run_logged() {
  local log_file="$1"
  shift
  local -a cmd=("$@")

  {
    printf '>>>'
    printf ' %q' "${cmd[@]}"
    printf '\n'
  } | tee -a "${log_file}"

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    return 0
  fi

  "${cmd[@]}" 2>&1 | tee -a "${log_file}"
  return "${PIPESTATUS[0]}"
}

run_logged_in_dir() {
  local dir="$1"
  shift
  local log_file="$1"
  shift
  local -a cmd=("$@")

  {
    printf '>>> (cd %q &&' "${dir}"
    printf ' %q' "${cmd[@]}"
    printf ' )\n'
  } | tee -a "${log_file}"

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    return 0
  fi

  (
    cd "${dir}"
    "${cmd[@]}"
  ) 2>&1 | tee -a "${log_file}"
  return "${PIPESTATUS[0]}"
}

if [[ "${DRY_RUN}" -eq 0 ]]; then
  require_cmd cargo
  if [[ "${STOP_AFTER}" != "baseline" && "${STOP_AFTER}" != "transpile" ]]; then
    require_cmd g++
  fi
fi

reset_artifacts

echo "Stage 1/4: Rust baseline (cargo test on either crate)"
run_logged "${LOG_RUST}" \
  cargo test --manifest-path "${EITHER_MANIFEST}"
if [[ "${STOP_AFTER}" == "baseline" ]]; then
  echo "Stopped after stage: baseline"
  exit 0
fi

echo "Stage 2/4: Transpile expanded either crate"
run_logged_in_dir "${REPO_ROOT}" "${LOG_TRANSPILE}" \
  cargo run -p rusty-cpp-transpiler -- --crate "${EITHER_MANIFEST}" --output-dir "${CPP_OUT_DIR}" --expand
run_logged_in_dir "${REPO_ROOT}" "${LOG_TRANSPILE}" \
  bash -lc "cargo expand --manifest-path \"${EITHER_MANIFEST}\" --lib --tests > \"${EXPANDED_TESTS_RS}\""
run_logged_in_dir "${REPO_ROOT}" "${LOG_TRANSPILE}" \
  cargo run -p rusty-cpp-transpiler -- "${EXPANDED_TESTS_RS}" --output "${EXPANDED_TESTS_CPPM}" --module-name "rustycpp.either_expanded_tests"
if [[ "${STOP_AFTER}" == "transpile" ]]; then
  echo "Stopped after stage: transpile"
  exit 0
fi

if [[ "${DRY_RUN}" -eq 0 && ! -f "${CPP_OUT_DIR}/either.cppm" ]]; then
  echo "error: expected transpiled output not found: ${CPP_OUT_DIR}/either.cppm" >&2
  exit 1
fi
if [[ "${DRY_RUN}" -eq 0 && ! -f "${EXPANDED_TESTS_CPPM}" ]]; then
  echo "error: expected expanded-tests transpiled output not found: ${EXPANDED_TESTS_CPPM}" >&2
  exit 1
fi

echo "Stage 3/4: Build transpiled C++ module"
run_logged_in_dir "${WORK_DIR}" "${LOG_CPP_BUILD}" \
  g++ -std=c++23 -fmodules-ts -I "${REPO_ROOT}/include" -x c++ -c "${CPP_OUT_DIR}/either.cppm" -o "${MODULE_OBJ}"
run_logged_in_dir "${WORK_DIR}" "${LOG_CPP_BUILD}" \
  g++ -std=c++23 -fmodules-ts -fmax-errors=80 -I "${REPO_ROOT}/include" -I "${REPO_ROOT}/tests/cpp/include" -x c++ -c "${EXPANDED_TESTS_CPPM}" -o "${EXPANDED_TESTS_OBJ}"
if [[ "${STOP_AFTER}" == "build" ]]; then
  echo "Stopped after stage: build"
  exit 0
fi

if [[ "${DRY_RUN}" -eq 0 ]]; then
  mapfile -t EXPANDED_TEST_WRAPPERS < <(
    sed -n 's/^[[:space:]]*export[[:space:]]\+void[[:space:]]\+\(rusty_test_[A-Za-z0-9_]\+\)[[:space:]]*(.*/\1/p' "${EXPANDED_TESTS_CPPM}"
  )
  if [[ "${#EXPANDED_TEST_WRAPPERS[@]}" -eq 0 ]]; then
    echo "error: no exported expanded test wrappers found in ${EXPANDED_TESTS_CPPM}" >&2
    exit 1
  fi

  {
    cat <<'EOF'
import either;
import rustycpp.either_expanded_tests;

extern "C" long write(int, const void*, unsigned long);

static void log_line(const char* msg) {
  unsigned long len = 0;
  while (msg[len] != '\0') {
    ++len;
  }
  (void)write(2, msg, len);
  (void)write(2, "\n", 1);
}

int main() {
EOF
    for fn_name in "${EXPANDED_TEST_WRAPPERS[@]}"; do
      readable_name="${fn_name#rusty_test_}"
      printf '  log_line("[RUN] %s");\n' "${readable_name}"
      printf '  %s();\n' "${fn_name}"
      printf '  log_line("[OK] %s");\n' "${readable_name}"
    done
    cat <<'EOF'
  return 0;
}
EOF
  } > "${TEST_RUNNER_MAIN}"
fi

echo "Stage 4/4: Link and run transpiled expanded test wrappers"
run_logged_in_dir "${WORK_DIR}" "${LOG_CPP_RUN}" \
  g++ -std=c++23 -fmodules-ts -I "${REPO_ROOT}/include" "${TEST_RUNNER_MAIN}" "${MODULE_OBJ}" "${EXPANDED_TESTS_OBJ}" -o "${TEST_RUNNER_BIN}"
run_logged_in_dir "${WORK_DIR}" "${LOG_CPP_RUN}" \
  "${TEST_RUNNER_BIN}"
if [[ "${STOP_AFTER}" == "run" ]]; then
  echo "Stopped after stage: run"
fi

echo "Parity harness finished."
echo "Logs:"
echo "  Rust baseline : ${LOG_RUST}"
echo "  Transpile     : ${LOG_TRANSPILE}"
echo "  C++ build     : ${LOG_CPP_BUILD}"
echo "  C++ run       : ${LOG_CPP_RUN}"
