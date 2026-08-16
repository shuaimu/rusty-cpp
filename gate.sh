#!/usr/bin/env bash
# Pre-push gate. Run this before pushing to main.
#
# WHY THIS EXISTS: the matrix is a LOCAL pre-push gate by design (a0ebd26d
# removed the CI job; it is back for pull requests only). But there was no
# committed script saying what "the gate" is, so in practice a 7-crate subset
# became the gate — and five regressions (path, semver, take_mut, serde_bytes,
# bitflags) each shipped on a commit reporting that subset green. A subset is
# not a proxy for the matrix. This script is the definition.
#
# Usage:
#   ./gate.sh              # units + full parity matrix
#   ./gate.sh --with-cache # also rebuild the modules cache (REQUIRED when you
#                          # touched include/rusty/**, transpiled/**, or any
#                          # vendored port — the matrix does not cover those)
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

WITH_CACHE=0
[[ "${1:-}" == "--with-cache" ]] && WITH_CACHE=1

# libclang for the checker's test binaries; harmless when already set.
export LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-/home/users/shuai/.linuxbrew/lib}"

fail=0
step() { printf '\n=== %s ===\n' "$1"; }

step "transpiler unit + integration tests"
# The transpiler is a SEPARATE cargo package — `cargo test` at the repo root
# runs the checker, not this. Getting that wrong costs a full cycle.
tp_log=$(mktemp)
( cd transpiler && cargo test --release ) > "$tp_log" 2>&1
tp_rc=$?
grep -hoE "test result: (ok|FAILED)\. [0-9]+ passed; [0-9]+ failed" "$tp_log" | sort | uniq -c
if [[ $tp_rc -ne 0 ]]; then
  # either_parity_harness spawns real builds and flakes when cargo runs test
  # BINARIES concurrently. Don't make a human remember that — check it here.
  # Any failure outside that harness is real and gates immediately.
  others=$(grep -E "^test [^ ]+ \.\.\. FAILED" "$tp_log" | grep -vc "either_parity" || true)
  if [[ "${others:-0}" -eq 0 ]]; then
    echo "   transpiler tests failed only in either_parity_harness — re-running it SERIALLY"
    if ( cd transpiler && cargo test --release --test either_parity_harness -- --test-threads=1 ) \
         > "$tp_log.serial" 2>&1; then
      echo "   serial re-run PASSED — parallel-contention flake, not a regression"
      grep -E "^test result:" "$tp_log.serial" | tail -1
    else
      echo "!! serial re-run ALSO failed — this is real"
      grep -E "^test [^ ]+ \.\.\. FAILED|^test result:" "$tp_log.serial" | tail -10
      fail=1
    fi
  else
    echo "!! transpiler tests failed (exit $tp_rc), ${others} failure(s) outside either_parity:"
    grep -E "^test [^ ]+ \.\.\. FAILED" "$tp_log" | grep -v "either_parity" | head -10
    echo "   full log: $tp_log"
    fail=1
  fi
fi

if [[ $WITH_CACHE -eq 1 ]]; then
  step "modules cache (header / vendored-port changes)"
  if [[ -d .rusty-modules-cache/build ]]; then
    ( cd .rusty-modules-cache/build && ninja ) 2>&1 | tail -5
    [[ ${PIPESTATUS[0]} -ne 0 ]] && { echo "!! ninja failed"; fail=1; }
  else
    echo "no .rusty-modules-cache/build — skipped (run cmake first if you need it)"
  fi
fi

step "full parity matrix (all crates)"
# Do NOT cargo build while this runs: the shared target/ makes a concurrent
# build fail the link and eat the Summary line.
matrix_log=$(mktemp)
bash tests/transpile_tests/run_parity_matrix.sh > "$matrix_log" 2>&1
summary=$(grep -E '^Summary:' "$matrix_log" | tail -1)
echo "${summary:-(no Summary line — matrix did not finish; see $matrix_log)}"
# `known-fail=N` also ends in `fail=N`, and a greedy `.*` backtracks to the
# LAST match — the first cut of this line reported known-fail as failures.
# Require whitespace immediately before `fail=` so only the real field matches.
fails=$(printf '%s\n' "$summary" | sed -nE 's/.*[[:space:]]fail=([0-9]+).*/\1/p')
if [[ "${fails:-1}" != "0" ]]; then
  echo "!! matrix has ${fails:-?} failing crate(s):"
  grep -oE "(FAIL): [A-Za-z_-]+" "$matrix_log" | sort -u
  echo "   full log: $matrix_log"
  fail=1
fi

printf '\n'
if [[ $fail -eq 0 ]]; then
  echo "GATE GREEN — safe to push."
  [[ $WITH_CACHE -eq 0 ]] && echo "(modules cache not rebuilt; re-run with --with-cache if you touched headers or vendored ports)"
else
  echo "GATE RED — do not push."
fi
exit $fail
