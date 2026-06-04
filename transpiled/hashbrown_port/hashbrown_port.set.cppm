// HashSet facade — wraps HashMap <T, std::monostate, S>. Upstream Rust's
// `hashbrown::HashSet<T>` is exactly `HashMap<T, ()>`.
module;

#include <cstdint>
#include <cstddef>
#include <utility>
#include <variant>
#include <rusty/rusty.hpp>

export module hashbrown_port.set;
import hashbrown_port.map;
import hashbrown_port.hasher;

namespace rusty::port::collections::hashbrown {

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

    bool insert(T value) {
        auto prev = this->map.insert(std::move(value), std::monostate{});
        return prev.is_none();
    }

    bool contains(const T& value) const {
        // const_cast: HashMap::table.find() isn't const-correct in
        // the transpiled form (find threads through a non-const
        // table API internally).
        auto& m = const_cast<HashMap<T, std::monostate, S>&>(this->map);
        auto h = make_hash<T, S>(m.hash_builder, value);
        return m.table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == value;
        }).is_some();
    }

    bool remove(const T& value) {
        auto& m = this->map;
        auto h = make_hash<T, S>(m.hash_builder, value);
        auto eq = [&](const auto& kv) { return std::get<0>(kv) == value; };
        auto b = m.table.find(h, eq);
        if (b.is_none()) return false;
        m.table.erase(b.unwrap());
        return true;
    }

    void clear() {
        // RawTable::clear() has a pre-existing transpiler-emission
        // bug (self_.table on a ScopeGuard missing operator*).
        // Replace the backing map to clear; same semantic outcome.
        this->map = HashMap<T, std::monostate, S>::new_();
    }

    HashSet<T, S> clone() const { return HashSet<T, S>(this->map.clone()); }

    // STL-compat range iteration. Wraps the underlying HashMap's iter
    // (entries are tuple<T, monostate>) and projects to just the key
    // via operator*. Enables `for (const auto& x : set)`.
    struct stl_iter_t {
        typename HashMap<T, std::monostate, S>::stl_iter_t inner;

        stl_iter_t() = default;
        explicit stl_iter_t(typename HashMap<T, std::monostate, S>::stl_iter_t it)
            : inner(std::move(it)) {}
        stl_iter_t& operator++() { ++inner; return *this; }
        const T& operator*() {
            return std::get<0>(*inner);
        }
        bool operator==(const stl_iter_t& o) const { return inner == o.inner; }
        bool operator!=(const stl_iter_t& o) const { return inner != o.inner; }
    };

    stl_iter_t begin() {
        return stl_iter_t(this->map.begin());
    }
    stl_iter_t end() {
        return stl_iter_t(this->map.end());
    }
};

} // namespace rusty::port::collections::hashbrown
