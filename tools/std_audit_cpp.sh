#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

SCOPE="runtime"
PROFILE="host-minimal"
FORMAT="text"

usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Audit C++ std usage in rusty-cpp sources.

Options:
  --scope <runtime|transpiler|all>      Source scope (default: runtime)
  --profile <host-minimal|strict-no-std> Forbidden-include profile (default: host-minimal)
  --format <text|json>                  Output format (default: text)
  -h, --help                            Show this help
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --scope)
            SCOPE="${2:-}"
            shift 2
            ;;
        --profile)
            PROFILE="${2:-}"
            shift 2
            ;;
        --format)
            FORMAT="${2:-}"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown option '$1'" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [[ "${SCOPE}" != "runtime" && "${SCOPE}" != "transpiler" && "${SCOPE}" != "all" ]]; then
    echo "error: invalid --scope '${SCOPE}'" >&2
    exit 2
fi
if [[ "${PROFILE}" != "host-minimal" && "${PROFILE}" != "strict-no-std" ]]; then
    echo "error: invalid --profile '${PROFILE}'" >&2
    exit 2
fi
if [[ "${FORMAT}" != "text" && "${FORMAT}" != "json" ]]; then
    echo "error: invalid --format '${FORMAT}'" >&2
    exit 2
fi

declare -a SEARCH_PATHS=()
declare -a GLOBS=(
    -g '*.h'
    -g '*.hpp'
    -g '*.hh'
    -g '*.hxx'
    -g '*.c'
    -g '*.cc'
    -g '*.cpp'
    -g '*.cxx'
    -g '*.cppm'
)

case "${SCOPE}" in
    runtime)
        SEARCH_PATHS=("${REPO_ROOT}/include/rusty")
        ;;
    transpiler)
        SEARCH_PATHS=("${REPO_ROOT}/transpiler/src")
        ;;
    all)
        SEARCH_PATHS=(
            "${REPO_ROOT}/include/rusty"
            "${REPO_ROOT}/transpiler/src"
            "${REPO_ROOT}/tests/transpile_tests"
            "${REPO_ROOT}/examples"
        )
        ;;
esac

if ! command -v rg >/dev/null 2>&1; then
    echo "error: ripgrep (rg) is required" >&2
    exit 2
fi

std_lines=$(rg -n '\bstd::' "${SEARCH_PATHS[@]}" "${GLOBS[@]}" 2>/dev/null | wc -l | tr -d '[:space:]')
std_refs=$(rg -o '\bstd::[A-Za-z_][A-Za-z0-9_:]*' "${SEARCH_PATHS[@]}" "${GLOBS[@]}" 2>/dev/null | wc -l | tr -d '[:space:]')
file_count=$(rg --files "${SEARCH_PATHS[@]}" "${GLOBS[@]}" 2>/dev/null | wc -l | tr -d '[:space:]')

top_files=$(
    rg -n '\bstd::' "${SEARCH_PATHS[@]}" "${GLOBS[@]}" 2>/dev/null \
    | awk -F: '{count[$1]++} END {for (f in count) printf "%7d %s\n", count[f], f}' \
    | sort -nr \
    | head -n 10
)

top_symbols=$(
    rg -o --no-filename --no-line-number '\bstd::[A-Za-z_][A-Za-z0-9_:]*' "${SEARCH_PATHS[@]}" "${GLOBS[@]}" 2>/dev/null \
    | sort \
    | uniq -c \
    | sort -nr \
    | head -n 15
)

declare -a FORBIDDEN_INCLUDES=()
if [[ "${PROFILE}" == "host-minimal" ]]; then
    FORBIDDEN_INCLUDES=(
        "thread"
        "mutex"
        "condition_variable"
        "shared_mutex"
        "future"
        "filesystem"
    )
else
    FORBIDDEN_INCLUDES=(
        "vector"
        "string"
        "string_view"
        "tuple"
        "optional"
        "variant"
        "any"
        "typeindex"
        "type_traits"
        "memory"
        "functional"
        "map"
        "set"
        "unordered_map"
        "unordered_set"
        "algorithm"
        "utility"
        "thread"
        "mutex"
        "condition_variable"
        "shared_mutex"
        "future"
        "filesystem"
    )
fi

forbidden_hits=$(
    for header in "${FORBIDDEN_INCLUDES[@]}"; do
        rg -n "#include\\s*<${header}>" "${SEARCH_PATHS[@]}" "${GLOBS[@]}" 2>/dev/null || true
    done \
    | awk -F: '{count[$1]++} END {for (f in count) printf "%7d %s\n", count[f], f}' \
    | sort -nr
)

if [[ "${FORMAT}" == "json" ]]; then
    # Escape as simple JSON arrays of strings to keep dependencies minimal.
    top_files_json=$(printf '%s\n' "${top_files}" | sed '/^$/d' | sed 's/"/\\"/g' | awk '{printf "\"%s\",", $0}' | sed 's/,$//')
    top_symbols_json=$(printf '%s\n' "${top_symbols}" | sed '/^$/d' | sed 's/"/\\"/g' | awk '{printf "\"%s\",", $0}' | sed 's/,$//')
    forbidden_hits_json=$(printf '%s\n' "${forbidden_hits}" | sed '/^$/d' | sed 's/"/\\"/g' | awk '{printf "\"%s\",", $0}' | sed 's/,$//')
    cat <<EOF
{
  "scope": "${SCOPE}",
  "profile": "${PROFILE}",
  "files_scanned": ${file_count},
  "std_lines": ${std_lines},
  "std_refs": ${std_refs},
  "top_files": [${top_files_json}],
  "top_symbols": [${top_symbols_json}],
  "forbidden_include_hits": [${forbidden_hits_json}]
}
EOF
else
    cat <<EOF
[std-audit]
scope=${SCOPE}
profile=${PROFILE}
files_scanned=${file_count}
std_lines=${std_lines}
std_refs=${std_refs}

top_files:
${top_files:-  (none)}

top_symbols:
${top_symbols:-  (none)}

forbidden_include_hits:
${forbidden_hits:-  (none)}
EOF
fi
