    // Hand-written-HashMap-compatibility extension: default ctor delegates
    // to default_(). Real Rust HashMap is only constructible via `new()`
    // / `default()` / `with_capacity()`; the C++ aggregate translation
    // otherwise has no default ctor (RawTable's ctor takes 3 args). Mako
    // rrr's `SerializableRegistryMap` / `AnyMessageRegistryMap` and many
    // ported map-of-X structs declare a plain `rusty::HashMap<K,V> map;`
    // field; surface a default ctor here so those keep building.
    HashMap()
        : hash_builder(),
          table(RawTable<std::tuple<K, V>, A>::new_()) {}
    // Two-arg ctor previously provided implicitly via aggregate
    // initialization (`HashMap{hash, table}`); adding the default ctor
    // above suppressed the aggregate, so spell out the field-init form
    // explicitly here. Used by `with_hasher` / `with_capacity_*` /
    // `default_()` factories below.
    HashMap(S hash_builder_init, RawTable<std::tuple<K, V>, A> table_init)
        : hash_builder(std::move(hash_builder_init)),
          table(std::move(table_init)) {}

    // Hand-written-HashMap-compatibility lookups: `contains_key`,
    // `get`, `remove`. Real Rust expresses these with a `Q: Borrow<K>`
    // trait bound that the transpiler skips; provide concrete K-typed
    // overloads here using the existing table.find()/erase() entry
    // points (same pattern HashSet uses above in set.cppm).
    bool contains_key(const K& key) const {
        auto& m = const_cast<HashMap<K, V, S, A>&>(*this);
        auto h = make_hash<K, S>(m.hash_builder, key);
        return m.table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == key;
        }).is_some();
    }
    ::rusty::Option<V&> get(const K& key) {
        auto h = make_hash<K, S>(this->hash_builder, key);
        auto b = this->table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == key;
        });
        if (b.is_none()) return ::rusty::Option<V&>(::rusty::None);
        auto&& bucket = b.unwrap();
        return ::rusty::Option<V&>(std::get<1>(bucket.as_mut()));
    }
    ::rusty::Option<const V&> get(const K& key) const {
        auto& m = const_cast<HashMap<K, V, S, A>&>(*this);
        auto h = make_hash<K, S>(m.hash_builder, key);
        auto b = m.table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == key;
        });
        if (b.is_none()) return ::rusty::Option<const V&>(::rusty::None);
        auto&& bucket = b.unwrap();
        return ::rusty::Option<const V&>(
            static_cast<const V&>(std::get<1>(bucket.as_mut())));
    }
    ::rusty::Option<V> remove(const K& key) {
        auto h = make_hash<K, S>(this->hash_builder, key);
        auto b = this->table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == key;
        });
        if (b.is_none()) return ::rusty::Option<V>(::rusty::None);
        auto bucket = b.unwrap();
        auto kv = this->table.remove(bucket);
        return ::rusty::Option<V>(
            std::move(std::get<1>(std::get<0>(kv))));
    }

    // STL-compat range iteration. Wraps the underlying `iter_mut()`
    // Rust-style next() iterator with a stateful adapter that exposes
    // operator++ / operator* / != so consumers can write
    // `for (auto& [k,v] : map)`. `*it` returns `std::tuple<const K&, V&>`
    // which destructures into k (const K) and v (V&). All actual storage
    // remains in the map; the tuple only holds references.
    struct stl_iter_t {
        // The raw iterator owns the bucket position. We cache the
        // current Bucket so operator* can return refs into it without
        // re-advancing.
        rusty::Option<IterMut<K, V>> raw;
        rusty::Option<std::tuple<const K&, V&>> cur;

        stl_iter_t() : raw(rusty::None), cur(rusty::None) {}
        explicit stl_iter_t(IterMut<K, V> it) : raw(std::move(it)), cur(rusty::None) {
            advance();
        }
        void advance() {
            if (raw.is_none()) { cur = rusty::None; return; }
            cur = raw.as_mut().unwrap().next();
        }
        stl_iter_t& operator++() { advance(); return *this; }
        std::tuple<const K&, V&> operator*() {
            return cur.as_ref().unwrap();
        }
        bool operator==(const stl_iter_t& o) const {
            return cur.is_none() && o.cur.is_none();
        }
        bool operator!=(const stl_iter_t& o) const { return !(*this == o); }
    };

    stl_iter_t begin() {
        return stl_iter_t(this->iter_mut());
    }
    stl_iter_t end() {
        return stl_iter_t{};
    }

    // operator[]: container-style insert-or-default-and-get. Returns a
    // reference into the map's storage. Hand-written HashMap relied on
    // std::unordered_map's `[]` returning a default-constructed value
    // for missing keys. We replicate by inserting a default V on miss.
    V& operator[](const K& key) {
        auto h = make_hash<K, S>(this->hash_builder, key);
        auto b = this->table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == key;
        });
        if (b.is_some()) {
            return std::get<1>(b.unwrap().as_mut());
        }
        // Not present: insert default V.
        this->insert(K(key), V{});
        auto b2 = this->table.find(h, [&](const auto& kv) {
            return std::get<0>(kv) == key;
        });
        return std::get<1>(b2.unwrap().as_mut());
    }
