// HashSet facade — wraps HashMap<T, std::monostate> the same way
// the upstream Rust hashbrown::HashSet wraps hashbrown::HashMap<T, ()>.
// The full Rust set module pulled in raw-entry / rustc-entry / iter
// types we'd rather not hand-port; a facade gives users the public
// surface (insert/contains/remove/len/iter) without that long tail.
module;

#include <cstdint>
#include <cstddef>
#include <utility>
#include <variant>
#include <rusty/rusty.hpp>

export module hashbrown_port.set;
import hashbrown_port.map;
import hashbrown_port.hasher;

export template<typename T, typename S = DefaultHasher>
struct HashSet {
    using Item = T;
    HashMap<T, std::monostate, S> map;

    HashSet() : map(HashMap<T, std::monostate, S>::new_()) {}
    HashSet(HashMap<T, std::monostate, S> m) : map(std::move(m)) {}

    static HashSet<T, S> new_() { return HashSet<T, S>(); }
    static HashSet<T, S> with_capacity(size_t capacity) {
        return HashSet<T, S>(HashMap<T, std::monostate, S>::with_capacity(capacity));
    }
    static HashSet<T, S> with_hasher(S hash_builder) {
        return HashSet<T, S>(HashMap<T, std::monostate, S>::with_hasher(std::move(hash_builder)));
    }
    static HashSet<T, S> with_capacity_and_hasher(size_t capacity, S hash_builder) {
        return HashSet<T, S>(HashMap<T, std::monostate, S>::with_capacity_and_hasher(capacity, std::move(hash_builder)));
    }

    size_t len() const { return this->map.len(); }
    bool is_empty() const { return this->map.is_empty(); }
    size_t capacity() const { return this->map.capacity(); }

    // Returns true if `value` was newly inserted (not previously present).
    bool insert(T value) {
        auto prev = this->map.insert(std::move(value), std::monostate{});
        return prev.is_none();
    }

    bool contains(const T& value) const {
        // const_cast needed because HashMap::contains_key isn't const-correct
        // in the transpiled form — find threads through the table API which
        // takes a non-const reference internally.
        auto& m = const_cast<HashMap<T, std::monostate, S>&>(this->map);
        auto h = ::make_hash<T, S>(m.hash_builder, value);
        return m.table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == value;
        }).is_some();
    }

    bool remove(const T& value) {
        auto& m = this->map;
        auto h = ::make_hash<T, S>(m.hash_builder, value);
        auto eq = [&](const auto& kv) { return std::get<0>(kv) == value; };
        auto b = m.table.find(h, eq);
        if (b.is_none()) return false;
        m.table.erase(b.unwrap());
        return true;
    }

    void clear() {
        // Sidestep RawTable::clear() — its current emit references
        // `self_.table` on a ScopeGuard without the operator*.
        // Replace the backing map instead; same semantic outcome.
        this->map = HashMap<T, std::monostate, S>::new_();
    }

    HashSet<T, S> clone() const {
        return HashSet<T, S>(this->map.clone());
    }
};
