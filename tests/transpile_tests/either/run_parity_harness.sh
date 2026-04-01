#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: run_parity_harness.sh [options]

Automates the Phase 18 end-to-end parity workflow for tests/transpile_tests/either:
1) Rust baseline: cargo test
2) Transpile: rusty-cpp-transpiler --crate --expand
3) C++ build: compile generated either.cppm
4) C++ run: run generated smoke executable

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
SMOKE_MAIN="${WORK_DIR}/either_smoke_main.cpp"
SMOKE_BIN="${WORK_DIR}/either_smoke"

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
  require_cmd g++
fi

mkdir -p "${CPP_OUT_DIR}"

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
if [[ "${STOP_AFTER}" == "transpile" ]]; then
  echo "Stopped after stage: transpile"
  exit 0
fi

if [[ "${DRY_RUN}" -eq 0 && ! -f "${CPP_OUT_DIR}/either.cppm" ]]; then
  echo "error: expected transpiled output not found: ${CPP_OUT_DIR}/either.cppm" >&2
  exit 1
fi

echo "Stage 3/4: Build transpiled C++ module"
run_logged_in_dir "${WORK_DIR}" "${LOG_CPP_BUILD}" \
  g++ -std=c++23 -fmodules-ts -I "${REPO_ROOT}/include" -x c++ -c "${CPP_OUT_DIR}/either.cppm" -o "${MODULE_OBJ}"
if [[ "${STOP_AFTER}" == "build" ]]; then
  echo "Stopped after stage: build"
  exit 0
fi

if [[ "${DRY_RUN}" -eq 0 ]]; then
  cat > "${SMOKE_MAIN}" <<'EOF'
import either;

int main() {
  return 0;
}
EOF
fi

echo "Stage 4/4: Link and run C++ smoke executable"
run_logged_in_dir "${WORK_DIR}" "${LOG_CPP_RUN}" \
  g++ -std=c++23 -fmodules-ts -I "${REPO_ROOT}/include" "${SMOKE_MAIN}" "${MODULE_OBJ}" -o "${SMOKE_BIN}"
run_logged_in_dir "${WORK_DIR}" "${LOG_CPP_RUN}" \
  "${SMOKE_BIN}"
if [[ "${STOP_AFTER}" == "run" ]]; then
  echo "Stopped after stage: run"
fi

echo "Parity harness finished."
echo "Logs:"
echo "  Rust baseline : ${LOG_RUST}"
echo "  Transpile     : ${LOG_TRANSPILE}"
echo "  C++ build     : ${LOG_CPP_BUILD}"
echo "  C++ run       : ${LOG_CPP_RUN}"
