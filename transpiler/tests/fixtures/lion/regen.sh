#!/usr/bin/env bash
# Regenerate the Lion snapshot that transpiler tests read (README.md here):
#
#   tree/        Lion's crates, as SRPC compiles them (Cargo.toml + src/)
#   expand/      a workspace depending on lion-reactor and lion-executor
#                exactly as SRPC's Cargo.toml does, locked like SRPC
#   expanded/    `cargo expand --lib` of every crate in tree/, from expand/
#   parity/      the parity cases (hand-written; only their lock is refreshed)
#   MANIFEST.toml  provenance, resolved features, and the parity cases' Rust
#                output, measured here with `cargo run`
#
# usage: regen.sh LION_CHECKOUT SRPC_CARGO_LOCK [OUT_DIR]
#
#   LION_CHECKOUT    a Lion git checkout (SRPC's third-party/lion)
#   SRPC_CARGO_LOCK  SRPC's Cargo.lock: the snapshot is resolved from it, so
#                    `verus!` is expanded by the verus_builtin_macros SRPC
#                    compiles (Verus git db81a74 today)
#   OUT_DIR          default: this directory. Another directory gets a copy
#                    of parity/ and can be tested in place with
#                    RUSTY_CPP_LION_FIXTURE=OUT_DIR.
#
# Needs cargo-expand, python3, rsync, and the crates SRPC's lockfile names in
# the local Cargo cache (or network access). The test lane never runs this.
set -euo pipefail

# SRPC pins no toolchain, so its Rust lane builds Lion with rustup's default
# stable. Without this, rusty-cpp's own rust-toolchain.toml (its stdlib-port
# pin) would apply to everything under this directory. The `verus!` erasure is
# the macro crate's code either way; the toolchain only changes how rustc
# expands std's macros and derives, which the comparison never reads exactly.
export RUSTUP_TOOLCHAIN=${RUSTUP_TOOLCHAIN:-stable}

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
lion=$(cd "$1" && pwd)
srpc_lock=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
out=${3:-$here}
mkdir -p "$out"
out=$(cd "$out" && pwd)
target=${CARGO_TARGET_DIR:-$(mktemp -d)}
mkdir -p "$target"
echo "cargo target dir: $target"

# The crates SRPC's graph reaches (srpc/Cargo.toml: lion-reactor and
# lion-executor, default-features = false), in dependency order.
crates=(
  lion-framework-spec
  lion-utility/lion-utility-spec
  lion-reactor-spec
  lion-executor-spec
  lion-slab
  lion-timer-wheel
  lion-reactor
  lion-executor
)

for dir in "${crates[@]}"; do
  mkdir -p "$out/tree/$dir"
  cp "$lion/$dir/Cargo.toml" "$out/tree/$dir/Cargo.toml"
  rsync -a --delete "$lion/$dir/src/" "$out/tree/$dir/src/"
  # Cargo reads the manifest's [[test]] targets even when only --lib builds.
  if [[ -d "$lion/$dir/tests" ]]; then
    rsync -a --delete "$lion/$dir/tests/" "$out/tree/$dir/tests/"
  fi
done
cp "$lion/LICENSE" "$out/LICENSE"
if [[ "$out" != "$here" ]]; then
  mkdir -p "$out/parity"
  rsync -a --exclude Cargo.lock "$here/parity/" "$out/parity/"
fi

mkdir -p "$out/expand/src" "$out/expanded"
cat > "$out/expand/Cargo.toml" <<'EOF'
[package]
name = "lion-expand"
version = "0.0.0"
edition = "2021"
publish = false

# SRPC's Lion dependencies, spelled as in srpc/Cargo.toml.
[dependencies]
lion-reactor = { path = "../tree/lion-reactor", default-features = false }
lion-executor = { path = "../tree/lion-executor", default-features = false }

[workspace]
EOF
printf '// Only a dependency root for regen.sh.\n' > "$out/expand/src/lib.rs"

# Start from SRPC's lockfile; Cargo drops the entries this graph does not use
# and keeps every version and git revision it does.
for ws in expand parity; do
  cp "$srpc_lock" "$out/$ws/Cargo.lock"
  (cd "$out/$ws" && CARGO_TARGET_DIR="$target" cargo metadata --format-version 1 > "$target/$ws-metadata.json")
done

python3 - "$out/expand/Cargo.lock" "$srpc_lock" <<'EOF'
import re, sys
def packages(path):
    found = {}
    for block in open(path).read().split('[[package]]')[1:]:
        name = re.search(r'^name = "([^"]+)"', block, re.M).group(1)
        version = re.search(r'^version = "([^"]+)"', block, re.M).group(1)
        source = re.search(r'^source = "([^"]+)"', block, re.M)
        found[(name, version)] = source.group(1) if source else None
    return found
ours, srpc = packages(sys.argv[1]), packages(sys.argv[2])
for key, source in ours.items():
    if key[0] == 'lion-expand':
        continue
    if srpc.get(key, 'missing') != source:
        sys.exit(f'{key} resolves to {source}, SRPC has {srpc.get(key, "missing")}')
EOF

for dir in "${crates[@]}"; do
  name=$(basename "$dir")
  (cd "$out/expand" && CARGO_TARGET_DIR="$target" cargo expand --locked -p "$name" --lib --theme none) \
    > "$out/expanded/$name.rs"
done

parity_output=$(cd "$out/parity" && CARGO_TARGET_DIR="$target" cargo run --locked --quiet --example run)

python3 - "$out" "$lion" "$srpc_lock" "$target/expand-metadata.json" "$parity_output" "${crates[@]}" <<'EOF'
import hashlib, json, os, re, subprocess, sys
out, lion, srpc_lock, metadata_path, parity_output, *crates = sys.argv[1:]
run = lambda *args, cwd=None: subprocess.run(args, cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()
lion_rev = run('git', '-C', lion, 'rev-parse', 'HEAD')
lion_dirty = run('git', '-C', lion, 'status', '--porcelain', '--', *crates) != ''
lock = open(os.path.join(out, 'expand/Cargo.lock')).read()
vstd = re.search(r'name = "vstd"\nversion = "[^"]+"\nsource = "([^"]+)"', lock).group(1)
metadata = json.load(open(metadata_path))
names = {p['id']: p['name'] for p in metadata['packages']}
features = {names[n['id']]: sorted(n['features']) for n in metadata['resolve']['nodes']}

def parity_digest():
    # Must match `parity_inputs_digest` in transpiler/tests/lion_parity.rs.
    files = []
    for top in ['parity', 'tree/lion-slab', 'tree/lion-timer-wheel']:
        for root, dirs, names_ in os.walk(os.path.join(out, top)):
            dirs[:] = [d for d in dirs if d != 'target']
            files += [os.path.relpath(os.path.join(root, n), out) for n in names_]
    digest = hashlib.sha256()
    for rel in sorted(files):
        digest.update(rel.encode() + b'\0' + open(os.path.join(out, rel), 'rb').read() + b'\0')
    return digest.hexdigest()

lines = [
    '# Generated by regen.sh. Do not edit by hand.',
    f'lion_rev = "{lion_rev}"',
    f'lion_dirty = {str(lion_dirty).lower()}',
    f'vstd_source = "{vstd}"',
    f'verus_git_rev = "{vstd.split("#")[1]}"',
    f'srpc_lock_sha256 = "{hashlib.sha256(open(srpc_lock, "rb").read()).hexdigest()}"',
    f'rustc = "{run("rustc", "--version")}"',
    f'cargo_expand = "{run("cargo", "expand", "--version")}"',
    '',
]
for crate in crates:
    name = os.path.basename(crate)
    lines += [
        '[[crates]]',
        f'name = "{name}"',
        f'dir = "tree/{crate}"',
        'features = [' + ', '.join(f'"{f}"' for f in features[name]) + ']',
        f'expanded = "expanded/{name}.rs"',
        '',
    ]
lines += [
    '[parity]',
    '# sha256 over parity/ and the two crates it uses (see lion_parity.rs);',
    '# `output` is `cargo run --example run` in parity/.',
    f'inputs_sha256 = "{parity_digest()}"',
    'output = """',
    parity_output,
    '"""',
]
open(os.path.join(out, 'MANIFEST.toml'), 'w').write('\n'.join(lines) + '\n')
EOF
echo "wrote $out/MANIFEST.toml"
