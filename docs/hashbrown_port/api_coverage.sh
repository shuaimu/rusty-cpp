#!/usr/bin/env bash
# Per-member API coverage probe for the SHIPPED hashbrown_port — the port that
# `rusty::HashMap`/`HashSet` currently alias.
#
# WHY THIS EXISTS: it is the apples-to-apples counterpart of
# docs/rusty/api_coverage.sh. The std port is replacing this one (see the
# 2026-07-27 directive at the top of docs/port_regen/STATUS.md), and that swap
# is only safe if the std port covers at least what this one does. Same 54
# members, same one-TU-per-member method, so the two numbers are comparable.
#
# Requires the hashbrown_port BMIs to exist:
#   cmake --build .rusty-modules-cache/build --target hashbrown_port
#
# Usage: api_coverage.sh [pcm_dir]     (default .rusty-modules-cache/pcm)
set -uo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PCM="${1:-$REPO/.rusty-modules-cache/pcm}"
[[ -f "$PCM/hashbrown_port.map.pcm" ]] || { echo "no BMIs in $PCM — build hashbrown_port first"; exit 1; }

D="$(mktemp -d)"; trap 'rm -rf "$D"' EXIT
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include -fprebuilt-module-path=$PCM"

emit() { # name, body
  cat > "$D/$1.cpp" <<EOF
#include <cstdint>
#include <string_view>
#include <rusty/rusty.hpp>
import hashbrown_port.raw;
import hashbrown_port.map;
import hashbrown_port.set;
import hashbrown_port.hasher;
using rusty::port::collections::hashbrown::HashMap;
using rusty::port::collections::hashbrown::HashSet;
using HM = HashMap<int,int>;
using HS = HashSet<int>;
int main() {
$2
  return 0;
}
EOF
}

# ---- HashMap (same 54 members as docs/rusty/api_coverage.sh) ----
emit map_new           '  auto m = HM::new_(); (void)m.len();'
emit map_with_capacity '  auto m = HM::with_capacity(16); (void)m.capacity();'
emit map_insert_get    '  auto m = HM::new_(); m.insert(1,10); (void)m.get(1).unwrap();'
emit map_get_mut       '  auto m = HM::new_(); m.insert(1,10); auto o = m.get_mut(1); (void)o.is_some();'
emit map_remove        '  auto m = HM::new_(); m.insert(1,10); (void)m.remove(1).is_some();'
emit map_remove_entry  '  auto m = HM::new_(); m.insert(1,10); (void)m.remove_entry(1).is_some();'
emit map_contains_key  '  auto m = HM::new_(); (void)m.contains_key(1);'
emit map_get_key_value '  auto m = HM::new_(); m.insert(1,10); (void)m.get_key_value(1).is_some();'
emit map_clear         '  auto m = HM::new_(); m.insert(1,10); m.clear();'
emit map_is_empty      '  auto m = HM::new_(); (void)m.is_empty();'
emit map_reserve       '  auto m = HM::new_(); m.reserve(32);'
emit map_try_reserve   '  auto m = HM::new_(); (void)m.try_reserve(32).is_ok();'
emit map_shrink_to_fit '  auto m = HM::new_(); m.shrink_to_fit();'
emit map_shrink_to     '  auto m = HM::new_(); m.shrink_to(4);'
emit map_keys          '  auto m = HM::new_(); m.insert(1,10); auto k = m.keys(); (void)k.next().is_some();'
emit map_values        '  auto m = HM::new_(); m.insert(1,10); auto v = m.values(); (void)v.next().is_some();'
emit map_values_mut    '  auto m = HM::new_(); m.insert(1,10); auto v = m.values_mut(); (void)v.next().is_some();'
emit map_iter          '  auto m = HM::new_(); m.insert(1,10); auto it = m.iter(); (void)it.next().is_some();'
emit map_iter_mut      '  auto m = HM::new_(); m.insert(1,10); auto it = m.iter_mut(); (void)it.next().is_some();'
emit map_into_iter     '  auto m = HM::new_(); m.insert(1,10); auto it = std::move(m).into_iter(); (void)it.next().is_some();'
emit map_into_keys     '  auto m = HM::new_(); m.insert(1,10); auto it = std::move(m).into_keys(); (void)it.next().is_some();'
emit map_into_values   '  auto m = HM::new_(); m.insert(1,10); auto it = std::move(m).into_values(); (void)it.next().is_some();'
emit map_entry         '  auto m = HM::new_(); auto e = m.entry(1); (void)sizeof(e);'
emit map_retain        '  auto m = HM::new_(); m.insert(1,10); m.retain([](auto&, auto&){ return true; });'
emit map_drain         '  auto m = HM::new_(); m.insert(1,10); auto d = m.drain(); (void)d.next().is_some();'
emit map_extract_if    '  auto m = HM::new_(); m.insert(1,10); auto d = m.extract_if([](auto&, auto&){ return true; }); (void)d.next().is_some();'
emit map_extend        '  auto m = HM::new_(); auto n = HM::new_(); m.extend(std::move(n));'
emit map_clone         '  auto m = HM::new_(); m.insert(1,10); auto c = m.clone(); (void)c.len();'
emit map_eq            '  auto a = HM::new_(); auto b = HM::new_(); (void)(a == b);'
emit map_hasher        '  auto m = HM::new_(); (void)m.hasher();'
emit map_with_hasher   '  auto m = HM::with_hasher(rusty::port::collections::hashbrown::DefaultHasher{}); (void)m.len();'
emit map_strkey        '  auto m = HashMap<std::string_view,int>::new_(); m.insert("a",1); (void)m.get("a").unwrap();'

# ---- HashSet ----
emit set_new           '  auto s = HS::new_(); (void)s.len();'
emit set_with_capacity '  auto s = HS::with_capacity(8); (void)s.capacity();'
emit set_insert        '  auto s = HS::new_(); (void)s.insert(1);'
emit set_contains      '  auto s = HS::new_(); (void)s.contains(1);'
emit set_remove        '  auto s = HS::new_(); (void)s.remove(1);'
emit set_take          '  auto s = HS::new_(); (void)s.take(1).is_some();'
emit set_get           '  auto s = HS::new_(); (void)s.get(1).is_some();'
emit set_clear         '  auto s = HS::new_(); s.clear();'
emit set_iter          '  auto s = HS::new_(); s.insert(1); auto it = s.iter(); (void)it.next().is_some();'
emit set_into_iter     '  auto s = HS::new_(); s.insert(1); auto it = std::move(s).into_iter(); (void)it.next().is_some();'
emit set_drain         '  auto s = HS::new_(); s.insert(1); auto d = s.drain(); (void)d.next().is_some();'
emit set_retain        '  auto s = HS::new_(); s.insert(1); s.retain([](auto&){ return true; });'
emit set_union         '  auto a = HS::new_(); auto b = HS::new_(); auto u = a.union_(b); (void)u.next().is_some();'
emit set_intersection  '  auto a = HS::new_(); auto b = HS::new_(); auto u = a.intersection(b); (void)u.next().is_some();'
emit set_difference    '  auto a = HS::new_(); auto b = HS::new_(); auto u = a.difference(b); (void)u.next().is_some();'
emit set_symdiff       '  auto a = HS::new_(); auto b = HS::new_(); auto u = a.symmetric_difference(b); (void)u.next().is_some();'
emit set_is_subset     '  auto a = HS::new_(); auto b = HS::new_(); (void)a.is_subset(b);'
emit set_is_superset   '  auto a = HS::new_(); auto b = HS::new_(); (void)a.is_superset(b);'
emit set_is_disjoint   '  auto a = HS::new_(); auto b = HS::new_(); (void)a.is_disjoint(b);'
emit set_extend        '  auto s = HS::new_(); auto t = HS::new_(); s.extend(std::move(t));'
emit set_clone         '  auto s = HS::new_(); s.insert(1); auto c = s.clone(); (void)c.len();'
emit set_eq            '  auto a = HS::new_(); auto b = HS::new_(); (void)(a == b);'

probe_one() {
  local f="$1" n; n="$(basename "$f" .cpp)"
  if clang++ $FLAGS -fsyntax-only "$f" 2>"$D/$n.err"; then
    echo "PASS $n"
  else
    echo "FAIL $n :: $(grep -m1 ' error: ' "$D/$n.err" | sed 's/.*error: //' | cut -c1-80)"
  fi
}
export -f probe_one; export D FLAGS
ls "$D"/*.cpp | xargs -P 8 -I{} bash -c 'probe_one {}' | sort > "$D/results.txt"
echo "=== hashbrown_port API coverage: $(grep -c '^PASS' "$D/results.txt")/$(grep -c . "$D/results.txt") ==="
grep '^FAIL' "$D/results.txt" || true
