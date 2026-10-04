#!/usr/bin/env bash
# Print the diff between the vendored modules in verus-erase/src and the
# upstream builtin_macros sources they were copied from, with each vendored
# file's provenance header stripped. The expected output is exactly the local
# changes each header lists; anything else is drift.
#
# Usage: verus-erase/upstream-diff.sh [UPSTREAM_BUILTIN_MACROS_SRC]
#
# Without an argument the upstream directory is located through Cargo: the
# verus_syn git dependency lives at <checkout>/dependencies/syn, and the
# macros at <checkout>/source/builtin_macros/src of the same checkout.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
upstream="${1:-}"
if [[ -z "$upstream" ]]; then
    syn_manifest="$(cargo metadata --format-version 1 --manifest-path "$here/Cargo.toml" \
        | python3 -c 'import json,sys; m=json.load(sys.stdin); print(next(p["manifest_path"] for p in m["packages"] if p["name"]=="verus_syn"))')"
    upstream="$(dirname "$(dirname "$(dirname "$syn_manifest")")")/source/builtin_macros/src"
fi
[[ -d "$upstream" ]] || { echo "upstream directory not found: $upstream" >&2; exit 2; }

status=0
while IFS= read -r vendored; do
    relative="${vendored#"$here/src/"}"
    body="$(mktemp)"
    # Drop everything up to and including the end-of-header marker.
    sed '1,/^\/\/ ---- end of vendoring header ----$/d' "$vendored" > "$body"
    diff -u --label "upstream/$relative" --label "vendored/$relative" \
        "$upstream/$relative" "$body" || status=1
    rm -f "$body"
done < <(grep -rl '^// ---- end of vendoring header ----$' "$here/src" | sort)
exit "$status"
