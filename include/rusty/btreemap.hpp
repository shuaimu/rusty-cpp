#ifndef RUSTY_BTREEMAP_HPP
#define RUSTY_BTREEMAP_HPP

#include <cstddef>
#include <map>
#include <stdexcept>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>
#include <vector>

#include "option.hpp"
#include "vec.hpp"
#include "hashmap.hpp"

// @safe
namespace rusty {

inline constexpr size_t BTREE_B = 6;
inline constexpr size_t BTREE_CAPACITY = 2 * BTREE_B - 1;
inline constexpr size_t BTREE_MIN_LEN = BTREE_B - 1;

template <typename K, typename V, typename Compare = std::less<K>>
class BTreeMap {
private:
    using map_type = std::map<K, V, Compare>;

    map_type map_;

    template<typename X>
    static std::remove_cvref_t<X> clone_value(const X& value) {
        using U = std::remove_cvref_t<X>;
        if constexpr (requires(const U& v) { v.clone(); }) {
            return value.clone();
        } else if constexpr (std::is_copy_constructible_v<U>) {
            return U(value);
        } else {
            static_assert(std::is_copy_constructible_v<U>, "BTreeMap clone requires copy-constructible or clone()-able elements");
        }
    }

    template<typename X>
    static std::remove_cvref_t<X> own_value(X&& value) {
        using U = std::remove_cvref_t<X>;
        if constexpr (std::is_lvalue_reference_v<X&&>) {
            return clone_value(value);
        } else {
            return U(std::forward<X>(value));
        }
    }

    template<typename L, typename R>
    static bool value_eq(const L& lhs, const R& rhs) {
        if constexpr (requires { lhs == rhs; }) {
            return static_cast<bool>(lhs == rhs);
        } else if constexpr (requires { rhs == lhs; }) {
            return static_cast<bool>(rhs == lhs);
        } else if constexpr (requires { lhs.eq(rhs); }) {
            return static_cast<bool>(lhs.eq(rhs));
        } else if constexpr (requires { rhs.eq(lhs); }) {
            return static_cast<bool>(rhs.eq(lhs));
        } else {
            return false;
        }
    }

    template<typename L, typename R>
    static bool key_eq(const L& lhs, const R& rhs) {
        if constexpr (requires { lhs == rhs; }) {
            return static_cast<bool>(lhs == rhs);
        } else if constexpr (requires { rhs == lhs; }) {
            return static_cast<bool>(rhs == lhs);
        } else if constexpr (requires { lhs.eq(rhs); }) {
            return static_cast<bool>(lhs.eq(rhs));
        } else if constexpr (requires { rhs.eq(lhs); }) {
            return static_cast<bool>(rhs.eq(lhs));
        } else if constexpr (
            requires { lhs.borrow(); }
            && std::is_convertible_v<decltype(lhs.borrow()), std::string_view>
            && std::is_convertible_v<R, std::string_view>) {
            return std::string_view(lhs.borrow()) == std::string_view(rhs);
        } else if constexpr (
            requires { rhs.borrow(); }
            && std::is_convertible_v<decltype(rhs.borrow()), std::string_view>
            && std::is_convertible_v<L, std::string_view>) {
            return std::string_view(lhs) == std::string_view(rhs.borrow());
        } else if constexpr (
            std::is_convertible_v<L, std::string_view>
            && std::is_convertible_v<R, std::string_view>) {
            return std::string_view(lhs) == std::string_view(rhs);
        } else if constexpr (requires { lhs.as_ref(); }) {
            using LRef = decltype(lhs.as_ref());
            if constexpr (!std::is_same_v<std::remove_cvref_t<LRef>, std::remove_cvref_t<L>>) {
                return key_eq(lhs.as_ref(), rhs);
            } else {
                return false;
            }
        } else if constexpr (requires { rhs.as_ref(); }) {
            using RRef = decltype(rhs.as_ref());
            if constexpr (!std::is_same_v<std::remove_cvref_t<RRef>, std::remove_cvref_t<R>>) {
                return key_eq(lhs, rhs.as_ref());
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    template<typename Q>
    typename map_type::iterator find_heterogeneous(const Q& key) {
        for (auto it = map_.begin(); it != map_.end(); ++it) {
            if (key_eq(it->first, key)) {
                return it;
            }
        }
        return map_.end();
    }

    template<typename Q>
    typename map_type::const_iterator find_heterogeneous(const Q& key) const {
        for (auto it = map_.begin(); it != map_.end(); ++it) {
            if (key_eq(it->first, key)) {
                return it;
            }
        }
        return map_.end();
    }

    template<typename Item>
    void insert_item(Item&& item) {
        auto&& key = std::get<0>(std::forward<Item>(item));
        auto&& value = std::get<1>(std::forward<Item>(item));
        insert(own_value(std::forward<decltype(key)>(key)), own_value(std::forward<decltype(value)>(value)));
    }

    template<typename Iter, typename Fn>
    static void for_each_item(Iter&& iter, Fn&& fn) {
        if constexpr (requires(std::remove_reference_t<Iter>& it) { it.next(); }) {
            auto it = std::forward<Iter>(iter);
            while (true) {
                auto maybe = it.next();
                if constexpr (requires { maybe.is_some(); maybe.unwrap(); }) {
                    if (!maybe.is_some()) {
                        break;
                    }
                    fn(maybe.unwrap());
                } else {
                    static_assert(
                        std::is_same_v<std::remove_cvref_t<decltype(maybe)>, void>,
                        "BTreeMap::from_iter/extend requires next() to return Option-like values");
                }
            }
        } else {
            for (auto&& item : iter) {
                fn(std::forward<decltype(item)>(item));
            }
        }
    }

public:
    BTreeMap() = default;

    static BTreeMap make() {
        return BTreeMap();
    }

    static BTreeMap new_() {
        return BTreeMap();
    }

    BTreeMap(BTreeMap&&) noexcept = default;
    BTreeMap& operator=(BTreeMap&&) noexcept = default;

    BTreeMap(const BTreeMap& other) {
        for (const auto& [key, value] : other.map_) {
            map_.emplace(clone_value(key), clone_value(value));
        }
    }

    BTreeMap& operator=(const BTreeMap& other) {
        if (this == &other) {
            return *this;
        }
        map_.clear();
        for (const auto& [key, value] : other.map_) {
            map_.emplace(clone_value(key), clone_value(value));
        }
        return *this;
    }

    size_t len() const { return map_.size(); }
    bool is_empty() const { return map_.empty(); }

    void clear() {
        map_.clear();
    }

    Option<V> insert(K key, V value) {
        auto it = map_.find(key);
        if (it != map_.end()) {
            V old = std::move(it->second);
            it->second = std::move(value);
            return Some(std::move(old));
        }
        map_.emplace(std::move(key), std::move(value));
        return None;
    }

    template<typename Q>
    Option<V&> get(const Q& key) {
        if constexpr (std::is_same_v<std::remove_cvref_t<Q>, K>) {
            auto it = map_.find(key);
            if (it != map_.end()) {
                return Option<V&>(it->second);
            }
            return None;
        } else {
            auto it = find_heterogeneous(key);
            if (it != map_.end()) {
                return Option<V&>(it->second);
            }
            return None;
        }
    }

    template<typename Q>
    Option<const V&> get(const Q& key) const {
        if constexpr (std::is_same_v<std::remove_cvref_t<Q>, K>) {
            auto it = map_.find(key);
            if (it != map_.end()) {
                return Option<const V&>(it->second);
            }
            return None;
        } else {
            auto it = find_heterogeneous(key);
            if (it != map_.end()) {
                return Option<const V&>(it->second);
            }
            return None;
        }
    }

    template<typename Q>
    Option<V&> get_mut(const Q& key) {
        return get(key);
    }

    template<typename Q>
    bool contains_key(const Q& key) const {
        return get(key).is_some();
    }

    template<typename Q>
    Option<std::tuple<const K&, const V&>> get_key_value(const Q& key) const {
        if constexpr (std::is_same_v<std::remove_cvref_t<Q>, K>) {
            auto it = map_.find(key);
            if (it != map_.end()) {
                return Option<std::tuple<const K&, const V&>>(std::tuple<const K&, const V&>(it->first, it->second));
            }
            return None;
        } else {
            auto it = find_heterogeneous(key);
            if (it != map_.end()) {
                return Option<std::tuple<const K&, const V&>>(std::tuple<const K&, const V&>(it->first, it->second));
            }
            return None;
        }
    }

    template<typename Q>
    Option<V> remove(const Q& key) {
        typename map_type::iterator it;
        if constexpr (std::is_same_v<std::remove_cvref_t<Q>, K>) {
            it = map_.find(key);
        } else {
            it = find_heterogeneous(key);
        }
        if (it == map_.end()) {
            return None;
        }
        V value = std::move(it->second);
        map_.erase(it);
        return Some(std::move(value));
    }

    template<typename Q>
    Option<std::tuple<K, V>> remove_entry(const Q& key) {
        typename map_type::iterator it;
        if constexpr (std::is_same_v<std::remove_cvref_t<Q>, K>) {
            it = map_.find(key);
        } else {
            it = find_heterogeneous(key);
        }
        if (it == map_.end()) {
            return None;
        }
        auto entry = std::tuple<K, V>(clone_value(it->first), std::move(it->second));
        map_.erase(it);
        return Option<std::tuple<K, V>>(std::move(entry));
    }

    template<typename Pred>
    void retain(Pred pred) {
        for (auto it = map_.begin(); it != map_.end();) {
            if (!pred(it->first, it->second)) {
                it = map_.erase(it);
            } else {
                ++it;
            }
        }
    }

    V& entry_value(K key) {
        return map_[std::move(key)];
    }

    auto entry(K key) {
        return detail::make_entry_probe(*this, std::move(key));
    }

    V& operator[](const K& key) {
        auto item = get_mut(key);
        if (item.is_some()) {
            return item.unwrap();
        }
        throw std::out_of_range("rusty::BTreeMap index: key not found");
    }

    const V& operator[](const K& key) const {
        auto item = get(key);
        if (item.is_some()) {
            return item.unwrap();
        }
        throw std::out_of_range("rusty::BTreeMap index: key not found");
    }

    template<typename Q>
    std::enable_if_t<!std::is_same_v<std::remove_cvref_t<Q>, K>, V&>
    operator[](const Q& key) {
        auto item = get_mut(key);
        if (item.is_some()) {
            return item.unwrap();
        }
        throw std::out_of_range("rusty::BTreeMap index: key not found");
    }

    template<typename Q>
    std::enable_if_t<!std::is_same_v<std::remove_cvref_t<Q>, K>, const V&>
    operator[](const Q& key) const {
        auto item = get(key);
        if (item.is_some()) {
            return item.unwrap();
        }
        throw std::out_of_range("rusty::BTreeMap index: key not found");
    }

    class iterator {
    private:
        typename map_type::iterator it_;

    public:
        explicit iterator(typename map_type::iterator it) : it_(it) {}

        std::tuple<const K&, V&> operator*() const {
            return std::tuple<const K&, V&>(it_->first, it_->second);
        }

        iterator& operator++() {
            ++it_;
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return it_ != other.it_;
        }

        bool operator==(const iterator& other) const {
            return it_ == other.it_;
        }
    };

    class const_iterator {
    private:
        typename map_type::const_iterator it_;

    public:
        explicit const_iterator(typename map_type::const_iterator it) : it_(it) {}

        std::tuple<const K&, const V&> operator*() const {
            return std::tuple<const K&, const V&>(it_->first, it_->second);
        }

        const_iterator& operator++() {
            ++it_;
            return *this;
        }

        bool operator!=(const const_iterator& other) const {
            return it_ != other.it_;
        }

        bool operator==(const const_iterator& other) const {
            return it_ == other.it_;
        }
    };

    class iter_range {
    private:
        typename map_type::const_iterator front_;
        typename map_type::const_iterator back_;
        size_t remaining_;

    public:
        using Item = std::tuple<const K&, const V&>;

        explicit iter_range(const map_type& map)
            : front_(map.begin()), back_(map.end()), remaining_(map.size()) {}

        Option<Item> next() {
            if (remaining_ == 0) {
                return None;
            }
            auto it = front_;
            ++front_;
            --remaining_;
            return Option<Item>(Item(it->first, it->second));
        }

        Option<Item> next_back() {
            if (remaining_ == 0) {
                return None;
            }
            --back_;
            --remaining_;
            return Option<Item>(Item(back_->first, back_->second));
        }

        std::tuple<size_t, Option<size_t>> size_hint() const {
            return std::make_tuple(remaining_, Option<size_t>(remaining_));
        }

        size_t len() const {
            return remaining_;
        }
    };

    class iter_mut_range {
    private:
        typename map_type::iterator front_;
        typename map_type::iterator back_;
        size_t remaining_;

    public:
        using Item = std::tuple<const K&, V&>;

        explicit iter_mut_range(map_type& map)
            : front_(map.begin()), back_(map.end()), remaining_(map.size()) {}

        Option<Item> next() {
            if (remaining_ == 0) {
                return None;
            }
            auto it = front_;
            ++front_;
            --remaining_;
            return Option<Item>(Item(it->first, it->second));
        }

        Option<Item> next_back() {
            if (remaining_ == 0) {
                return None;
            }
            --back_;
            --remaining_;
            return Option<Item>(Item(back_->first, back_->second));
        }

        std::tuple<size_t, Option<size_t>> size_hint() const {
            return std::make_tuple(remaining_, Option<size_t>(remaining_));
        }

        size_t len() const {
            return remaining_;
        }
    };

    class into_iter_range {
    private:
        map_type map_;
        typename map_type::iterator front_;
        typename map_type::iterator back_;
        size_t remaining_;

    public:
        using Item = std::tuple<K, V>;

        explicit into_iter_range(map_type&& map)
            : map_(std::move(map)), front_(map_.begin()), back_(map_.end()), remaining_(map_.size()) {}

        Option<Item> next() {
            if (remaining_ == 0) {
                return None;
            }
            auto it = front_;
            ++front_;
            --remaining_;
            return Option<Item>(Item(clone_value(it->first), std::move(it->second)));
        }

        Option<Item> next_back() {
            if (remaining_ == 0) {
                return None;
            }
            --back_;
            --remaining_;
            return Option<Item>(Item(clone_value(back_->first), std::move(back_->second)));
        }

        std::tuple<size_t, Option<size_t>> size_hint() const {
            return std::make_tuple(remaining_, Option<size_t>(remaining_));
        }

        size_t len() const {
            return remaining_;
        }
    };

    class keys_range {
    private:
        iter_range iter_;

    public:
        using Item = const K&;

        explicit keys_range(const map_type& map) : iter_(map) {}

        Option<Item> next() {
            auto item = iter_.next();
            if (item.is_none()) {
                return None;
            }
            return Option<Item>(std::get<0>(item.unwrap()));
        }

        Option<Item> next_back() {
            auto item = iter_.next_back();
            if (item.is_none()) {
                return None;
            }
            return Option<Item>(std::get<0>(item.unwrap()));
        }

        std::tuple<size_t, Option<size_t>> size_hint() const {
            return iter_.size_hint();
        }

        size_t len() const {
            return iter_.len();
        }
    };

    class values_range {
    private:
        iter_range iter_;

    public:
        using Item = const V&;

        explicit values_range(const map_type& map) : iter_(map) {}

        Option<Item> next() {
            auto item = iter_.next();
            if (item.is_none()) {
                return None;
            }
            return Option<Item>(std::get<1>(item.unwrap()));
        }

        Option<Item> next_back() {
            auto item = iter_.next_back();
            if (item.is_none()) {
                return None;
            }
            return Option<Item>(std::get<1>(item.unwrap()));
        }

        std::tuple<size_t, Option<size_t>> size_hint() const {
            return iter_.size_hint();
        }

        size_t len() const {
            return iter_.len();
        }
    };

    iterator begin() {
        return iterator(map_.begin());
    }

    iterator end() {
        return iterator(map_.end());
    }

    const_iterator begin() const {
        return const_iterator(map_.begin());
    }

    const_iterator end() const {
        return const_iterator(map_.end());
    }

    iter_range iter() const & {
        return iter_range(map_);
    }

    into_iter_range iter() && {
        return into_iter_range(std::move(map_));
    }

    iter_mut_range iter_mut() {
        return iter_mut_range(map_);
    }

    into_iter_range into_iter() & {
        map_type moved = std::move(map_);
        map_.clear();
        return into_iter_range(std::move(moved));
    }

    into_iter_range into_iter() && {
        return into_iter_range(std::move(map_));
    }

    iter_range into_iter() const & {
        return iter();
    }

    keys_range keys() const {
        return keys_range(map_);
    }

    values_range values() const {
        return values_range(map_);
    }

    BTreeMap clone() const {
        BTreeMap result;
        for (const auto& [key, value] : map_) {
            result.insert(clone_value(key), clone_value(value));
        }
        return result;
    }

    Option<std::pair<const K*, const V*>> first_key_value() const {
        if (map_.empty()) {
            return None;
        }
        const auto& entry = *map_.begin();
        return Option<std::pair<const K*, const V*>>(std::make_pair(&entry.first, &entry.second));
    }

    Option<std::pair<const K*, const V*>> last_key_value() const {
        if (map_.empty()) {
            return None;
        }
        auto it = map_.end();
        --it;
        return Option<std::pair<const K*, const V*>>(std::make_pair(&it->first, &it->second));
    }

    Option<std::pair<K, V>> pop_first() {
        if (map_.empty()) {
            return None;
        }
        auto it = map_.begin();
        auto out = std::make_pair(clone_value(it->first), std::move(it->second));
        map_.erase(it);
        return Option<std::pair<K, V>>(std::move(out));
    }

    Option<std::pair<K, V>> pop_last() {
        if (map_.empty()) {
            return None;
        }
        auto it = map_.end();
        --it;
        auto out = std::make_pair(clone_value(it->first), std::move(it->second));
        map_.erase(it);
        return Option<std::pair<K, V>>(std::move(out));
    }

    std::vector<std::pair<K, V>> range(const K& min, const K& max) const {
        std::vector<std::pair<K, V>> out;
        auto cmp = map_.key_comp();
        auto it = map_.lower_bound(min);
        while (it != map_.end() && cmp(it->first, max)) {
            out.emplace_back(clone_value(it->first), clone_value(it->second));
            ++it;
        }
        return out;
    }

    BTreeMap split_off(const K& key) {
        BTreeMap out;
        auto it = map_.lower_bound(key);
        while (it != map_.end()) {
            out.insert(clone_value(it->first), std::move(it->second));
            it = map_.erase(it);
        }
        return out;
    }

    void append(BTreeMap&& other) {
        extend(std::move(other));
    }

    void extend(BTreeMap&& other) {
        for (auto it = other.map_.begin(); it != other.map_.end(); ++it) {
            insert(clone_value(it->first), std::move(it->second));
        }
        other.clear();
    }

    template<typename Iter>
    void extend(Iter&& iter) {
        for_each_item(std::forward<Iter>(iter), [&](auto&& item) {
            insert_item(std::forward<decltype(item)>(item));
        });
    }

    template<typename Iter>
    static BTreeMap from_iter(Iter&& iter) {
        BTreeMap result;
        result.extend(std::forward<Iter>(iter));
        return result;
    }

    bool eq(const BTreeMap& other) const {
        return *this == other;
    }

    bool operator==(const BTreeMap& other) const {
        if (map_.size() != other.map_.size()) {
            return false;
        }

        auto it_l = map_.begin();
        auto it_r = other.map_.begin();
        auto cmp_l = map_.key_comp();
        auto cmp_r = other.map_.key_comp();

        for (; it_l != map_.end() && it_r != other.map_.end(); ++it_l, ++it_r) {
            const bool key_equal = !cmp_l(it_l->first, it_r->first) && !cmp_r(it_r->first, it_l->first);
            if (!key_equal) {
                return false;
            }
            if (!value_eq(it_l->second, it_r->second)) {
                return false;
            }
        }

        return it_l == map_.end() && it_r == other.map_.end();
    }

    bool operator!=(const BTreeMap& other) const {
        return !(*this == other);
    }
};

template<typename K, typename V>
BTreeMap<K, V> btreemap() {
    return BTreeMap<K, V>::make();
}

} // namespace rusty

#endif // RUSTY_BTREEMAP_HPP
