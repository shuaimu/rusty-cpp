// Runtime validation of the transpiled Rust std port (module `rusty`):
// std::collections::{HashMap, HashSet} over the recursively-transpiled
// hashbrown dep, hashed by the std RandomState/DefaultHasher chain
// (rusty::hash::SipHasher underneath).
import rusty;
#include <cassert>
#include <cstdio>
#include <string_view>

using HM = collections::hash::map::HashMap<int, int, ::hash::random::RandomState>;
using HS = collections::hash::set::HashSet<int, ::hash::random::RandomState>;

int main() {
    auto m = HM::new_();
    assert(m.len() == 0);
    m.insert(1, 10);
    m.insert(2, 20);
    m.insert(3, 30);
    assert(m.len() == 3);
    assert(m.get(2).is_some() && m.get(2).unwrap() == 20);
    m.insert(2, 22);  // overwrite
    assert(m.get(2).unwrap() == 22);
    assert(m.remove(1).is_some());
    assert(m.len() == 2 && !m.get(1).is_some());

    auto s = HS::new_();
    assert(s.insert(7) && !s.insert(7));  // second insert: already present
    assert(s.contains(7) && !s.contains(8));


    // String keys: content-hashed through DefaultHasher/SipHasher
    using HMS = collections::hash::map::HashMap<std::string_view, int, ::hash::random::RandomState>;
    auto sm = HMS::new_();
    sm.insert("alpha", 1);
    sm.insert("beta", 2);
    assert(sm.len() == 2);
    assert(sm.get("beta").is_some() && sm.get("beta").unwrap() == 2);
    assert(!sm.get("delta").is_some());
    sm.insert("beta", 22);
    assert(sm.get("beta").unwrap() == 22 && sm.len() == 2);

    std::printf("rusty (std) runtime OK: HashMap(int+string keys) insert/get/overwrite/remove + HashSet\n");
    return 0;
}
