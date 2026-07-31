#!/usr/bin/env bash
# Per-member API coverage probe for the transpiled Rust **std** hash port
# (module `std_port` from docs/rusty/build.sh).
#
# WHY THIS EXISTS: `rusty::HashMap`/`HashSet` are being retargeted off the
# direct-hashbrown port onto this std port (see the 2026-07-27 directive at the
# top of docs/port_regen/STATUS.md). That swap is only safe once the std port
# covers at least what the old port did, so this measures the surface the
# upstream std tests actually exercise, one member per TU — a single failure
# must not mask the rest, which is exactly what a single combined probe does.
#
# Usage: api_coverage.sh <work_dir>          # builds if needed, then probes
#        REUSE=1 api_coverage.sh <work_dir>  # skip the rebuild
#
# Baseline 2026-07-30: 36/54 (CORRECTED — an earlier 48/54 used a .next()-only
# probe that overcounted; RC_DRAIN below is idiom-agnostic and stricter).
set -uo pipefail
W="${1:?usage: api_coverage.sh <work_dir>}"
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
[[ -n "${REUSE:-}" ]] || bash "$REPO/docs/rusty/build.sh" "$W" | tail -3
OUT="$W/out"
[[ -f "$OUT/std_port.pcm" && -f "$OUT/hashbrown/hashbrown.pcm" ]] || {
  echo "no BMI — build failed"; exit 1; }

D="$W/api_probes"; rm -rf "$D"; mkdir -p "$D"
FLAGS="-std=c++23 -DRUSTY_PORTABLE_INTRINSICS=1 -march=native -I$REPO/include"
MODS="-fmodule-file=std_port=$OUT/std_port.pcm -fmodule-file=hashbrown=$OUT/hashbrown/hashbrown.pcm"

emit() { # name, body
  cat > "$D/$1.cpp" <<EOF
import std_port;
#include <string_view>
using HM = std_port::collections::hash::map::HashMap<int,int,std_port::hash::random::RandomState>;
using HS = std_port::collections::hash::set::HashSet<int,std_port::hash::random::RandomState>;
// Idiom-agnostic iterator drain. The two ports expose iteration DIFFERENTLY —
// the std port yields `.next()`-style iterators, hashbrown_port is range-for /
// rusty::count compatible. Probing with one spelling reports the other port's
// working members as missing, which is exactly the error this harness made on
// its first run (it undercounted hashbrown_port by ~7 members). Test the
// CAPABILITY, not the spelling.
#define RC_DRAIN(expr) do { auto __it = (expr); \
    if constexpr (requires { __it.next(); }) { (void)__it.next(); } \
    else { for (auto&& __v : __it) { (void)__v; } } } while (0)
int main() {
$2
  return 0;
}
EOF
}

# ---- HashMap ----
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
emit map_keys 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(m.keys());'
emit map_values 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(m.values());'
emit map_values_mut 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(m.values_mut());'
emit map_iter 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(m.iter());'
emit map_iter_mut 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(m.iter_mut());'
emit map_into_iter 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(std::move(m).into_iter());'
emit map_into_keys 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(std::move(m).into_keys());'
emit map_into_values 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(std::move(m).into_values());'
emit map_entry         '  auto m = HM::new_(); auto e = m.entry(1); (void)sizeof(e);'
emit map_retain        '  auto m = HM::new_(); m.insert(1,10); m.retain([](auto&, auto&){ return true; });'
emit map_drain 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(m.drain());'
emit map_extract_if 'auto m = HM::new_(); m.insert(1,10); RC_DRAIN(m.extract_if([](auto&&...){ return true; }));'
emit map_extend        '  auto m = HM::new_(); auto n = HM::new_(); m.extend(std::move(n));'
emit map_clone         '  auto m = HM::new_(); m.insert(1,10); auto c = m.clone(); (void)c.len();'
emit map_eq            '  auto a = HM::new_(); auto b = HM::new_(); (void)(a == b);'
emit map_hasher        '  auto m = HM::new_(); (void)m.hasher();'
emit map_with_hasher   '  auto m = HM::with_hasher(std_port::hash::random::RandomState::new_()); (void)m.len();'
emit map_strkey        '  using M = std_port::collections::hash::map::HashMap<std::string_view,int,std_port::hash::random::RandomState>;
  auto m = M::new_(); m.insert("a",1); (void)m.get("a").unwrap();'

# ---- HashSet ----
emit set_new           '  auto s = HS::new_(); (void)s.len();'
emit set_with_capacity '  auto s = HS::with_capacity(8); (void)s.capacity();'
emit set_insert        '  auto s = HS::new_(); (void)s.insert(1);'
emit set_contains      '  auto s = HS::new_(); (void)s.contains(1);'
emit set_remove        '  auto s = HS::new_(); (void)s.remove(1);'
emit set_take          '  auto s = HS::new_(); (void)s.take(1).is_some();'
emit set_get           '  auto s = HS::new_(); (void)s.get(1).is_some();'
emit set_clear         '  auto s = HS::new_(); s.clear();'
emit set_iter 'auto s = HS::new_(); s.insert(1); RC_DRAIN(s.iter());'
emit set_into_iter 'auto s = HS::new_(); s.insert(1); RC_DRAIN(std::move(s).into_iter());'
emit set_drain 'auto s = HS::new_(); s.insert(1); RC_DRAIN(s.drain());'
emit set_retain        '  auto s = HS::new_(); s.insert(1); s.retain([](auto&){ return true; });'
emit set_union 'auto a = HS::new_(); auto b = HS::new_(); RC_DRAIN(a.union_(b));'
emit set_intersection 'auto a = HS::new_(); auto b = HS::new_(); RC_DRAIN(a.intersection(b));'
emit set_difference 'auto a = HS::new_(); auto b = HS::new_(); RC_DRAIN(a.difference(b));'
emit set_symdiff 'auto a = HS::new_(); auto b = HS::new_(); RC_DRAIN(a.symmetric_difference(b));'
emit set_is_subset     '  auto a = HS::new_(); auto b = HS::new_(); (void)a.is_subset(b);'
emit set_is_superset   '  auto a = HS::new_(); auto b = HS::new_(); (void)a.is_superset(b);'
emit set_is_disjoint   '  auto a = HS::new_(); auto b = HS::new_(); (void)a.is_disjoint(b);'
emit set_extend        '  auto s = HS::new_(); auto t = HS::new_(); s.extend(std::move(t));'
emit set_clone         '  auto s = HS::new_(); s.insert(1); auto c = s.clone(); (void)c.len();'
emit set_eq            '  auto a = HS::new_(); auto b = HS::new_(); (void)(a == b);'

probe_one() {
  local f="$1" n; n="$(basename "$f" .cpp)"
  if clang++ $FLAGS $MODS -fsyntax-only "$f" 2>"$D/$n.err"; then
    echo "PASS $n"
  else
    echo "FAIL $n :: $(grep -m1 ' error: ' "$D/$n.err" | sed 's/.*error: //' | cut -c1-90)"
  fi
}
export -f probe_one; export D FLAGS MODS
ls "$D"/*.cpp | xargs -P 8 -I{} bash -c 'probe_one {}' | sort > "$D/results.txt"
echo "=== std hash API coverage: $(grep -c '^PASS' "$D/results.txt")/$(grep -c . "$D/results.txt") ==="
grep '^FAIL' "$D/results.txt" || true
