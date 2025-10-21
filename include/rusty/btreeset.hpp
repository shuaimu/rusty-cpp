#ifndef RUSTY_BTREESET_HPP
#define RUSTY_BTREESET_HPP

#include "btreemap.hpp"
#include "vec.hpp"
#include <functional>

// @safe
namespace rusty {

// BTreeSet implemented as a thin wrapper around BTreeMap<T, ()>
// Maintains elements in sorted order
template <typename T, typename Compare = std::less<T>>
class BTreeSet {
private:
    struct Unit {  // Empty struct to represent Rust's unit type ()
        bool operator==(const Unit&) const { return true; }
    };
    BTreeMap<T, Unit, Compare> map_;
    
public:
    // Constructors
    BTreeSet() = default;
    
    // @lifetime: owned
    static auto make() {
        return BTreeSet();
    }
    
    // Move constructor
    BTreeSet(BTreeSet&& other) noexcept : map_(std::move(other.map_)) {}
    
    // Move assignment
    BTreeSet& operator=(BTreeSet&& other) noexcept {
        if (this != &other) {
            map_ = std::move(other.map_);
        }
        return *this;
    }
    
    // Delete copy operations
    BTreeSet(const BTreeSet&) = delete;
    BTreeSet& operator=(const BTreeSet&) = delete;
    
    // Clone for explicit copying
    // @lifetime: owned
    BTreeSet clone() const {
        BTreeSet result;
        result.map_ = map_.clone();
        return result;
    }
    
    // Size and capacity
    size_t len() const { return map_.len(); }
    bool is_empty() const { return map_.is_empty(); }
    
    // Clear all elements
    void clear() {
        map_.clear();
    }
    
    // Insert element
    bool insert(T value) {
        bool existed = map_.contains_key(value);
        map_.insert(std::move(value), Unit{});
        return !existed;  // Return true if newly inserted
    }
    
    // Remove element
    bool remove(const T& value) {
        return map_.remove(value).is_some();
    }
    
    // Check if element exists
    bool contains(const T& value) const {
        return map_.contains_key(value);
    }
    
    // Get element (returns Option<const T*>)
    // @lifetime: (&'a) -> &'a
    Option<const T*> get(const T& value) const {
        for (const auto& [key, _] : map_) {
            if (key == value) {
                return Some(&key);
            }
        }
        return None;
    }
    
    // Take element (remove and return)
    // @lifetime: owned
    Option<T> take(const T& value) {
        if (map_.contains_key(value)) {
            map_.remove(value);
            return Some(value);
        }
        return None;
    }
    
    // Replace element (returns old value if existed)
    // @lifetime: owned
    Option<T> replace(T value) {
        if (map_.contains_key(value)) {
            T old = value;
            map_.insert(std::move(value), Unit{});
            return Some(old);
        } else {
            map_.insert(std::move(value), Unit{});
            return None;
        }
    }
    
    // Get first (minimum) element
    // @lifetime: (&'a) -> &'a
    Option<const T*> first() const {
        auto first_kv = map_.first_key_value();
        if (first_kv.is_some()) {
            return Some(first_kv.unwrap().first);
        }
        return None;
    }
    
    // Get last (maximum) element
    // @lifetime: (&'a) -> &'a
    Option<const T*> last() const {
        auto last_kv = map_.last_key_value();
        if (last_kv.is_some()) {
            return Some(last_kv.unwrap().first);
        }
        return None;
    }
    
    // Pop first (minimum) element
    // @lifetime: owned
    Option<T> pop_first() {
        auto popped = map_.pop_first();
        if (popped.is_some()) {
            return Some(std::move(popped.unwrap().first));
        }
        return None;
    }
    
    // Pop last (maximum) element
    // @lifetime: owned
    Option<T> pop_last() {
        auto popped = map_.pop_last();
        if (popped.is_some()) {
            return Some(std::move(popped.unwrap().first));
        }
        return None;
    }
    
    // Range operations
    
    // Get all elements in range [min, max]
    // @lifetime: owned
    Vec<T> range(const T& min, const T& max) const {
        auto map_range = map_.range(min, max);
        Vec<T> result = Vec<T>::with_capacity(map_range.len());
        for (size_t i = 0; i < map_range.len(); i++) {
            result.push(map_range[i].first);
        }
        return result;
    }
    
    // Split off everything after value (exclusive)
    // @lifetime: owned
    BTreeSet split_off(const T& value) {
        BTreeSet result;
        result.map_ = map_.split_off(value);
        return result;
    }
    
    // Append another set (all values in other must be > all values in self)
    void append(BTreeSet&& other) {
        map_.append(std::move(other.map_));
    }
    
    // Set operations
    
    // Union: self ∪ other (all elements from both sets)
    // @lifetime: owned
    BTreeSet union_with(const BTreeSet& other) const {
        BTreeSet result = this->clone();
        for (const auto& [key, _] : other.map_) {
            result.insert(key);
        }
        return result;
    }
    
    // Intersection: self ∩ other (elements in both sets)
    // @lifetime: owned
    BTreeSet intersection(const BTreeSet& other) const {
        BTreeSet result;
        // Iterate over smaller set for efficiency
        const BTreeSet* smaller = this->len() <= other.len() ? this : &other;
        const BTreeSet* larger = this->len() <= other.len() ? &other : this;
        
        for (const auto& [key, _] : smaller->map_) {
            if (larger->contains(key)) {
                result.insert(key);
            }
        }
        return result;
    }
    
    // Difference: self - other (elements in self but not in other)
    // @lifetime: owned
    BTreeSet difference(const BTreeSet& other) const {
        BTreeSet result;
        for (const auto& [key, _] : map_) {
            if (!other.contains(key)) {
                result.insert(key);
            }
        }
        return result;
    }
    
    // Symmetric difference: self △ other (elements in either but not both)
    // @lifetime: owned
    BTreeSet symmetric_difference(const BTreeSet& other) const {
        BTreeSet result;
        for (const auto& [key, _] : map_) {
            if (!other.contains(key)) {
                result.insert(key);
            }
        }
        for (const auto& [key, _] : other.map_) {
            if (!this->contains(key)) {
                result.insert(key);
            }
        }
        return result;
    }
    
    // Check if disjoint (no common elements)
    bool is_disjoint(const BTreeSet& other) const {
        // Check smaller set against larger
        const BTreeSet* smaller = this->len() <= other.len() ? this : &other;
        const BTreeSet* larger = this->len() <= other.len() ? &other : this;
        
        for (const auto& [key, _] : smaller->map_) {
            if (larger->contains(key)) {
                return false;
            }
        }
        return true;
    }
    
    // Check if subset
    bool is_subset(const BTreeSet& other) const {
        if (this->len() > other.len()) {
            return false;
        }
        for (const auto& [key, _] : map_) {
            if (!other.contains(key)) {
                return false;
            }
        }
        return true;
    }
    
    // Check if superset
    bool is_superset(const BTreeSet& other) const {
        return other.is_subset(*this);
    }
    
    // Extend from another set
    void extend(BTreeSet&& other) {
        map_.extend(std::move(other.map_));
    }
    
    // Retain only elements matching predicate
    template<typename Pred>
    void retain(Pred pred) {
        map_.retain([&pred](const T& key, const Unit&) {
            return pred(key);
        });
    }
    
    // Drain all elements into a Vec (in sorted order)
    // @lifetime: owned
    Vec<T> drain() {
        Vec<T> result = Vec<T>::with_capacity(map_.len());
        for (const auto& [key, _] : map_) {
            result.push(key);
        }
        map_.clear();
        return result;
    }
    
    // Iterator support (iterate in sorted order)
    class iterator {
    private:
        typename BTreeMap<T, Unit, Compare>::iterator inner_;
        
    public:
        iterator(typename BTreeMap<T, Unit, Compare>::iterator it) : inner_(it) {}
        
        const T& operator*() {
            return inner_->first;
        }
        
        const T* operator->() {
            return &(inner_->first);
        }
        
        iterator& operator++() {
            ++inner_;
            return *this;
        }
        
        bool operator!=(const iterator& other) const {
            return inner_ != other.inner_;
        }
        
        bool operator==(const iterator& other) const {
            return inner_ == other.inner_;
        }
    };
    
    class const_iterator {
    private:
        typename BTreeMap<T, Unit, Compare>::const_iterator inner_;
        
    public:
        const_iterator(typename BTreeMap<T, Unit, Compare>::const_iterator it) : inner_(it) {}
        
        const T& operator*() const {
            return inner_->first;
        }
        
        const T* operator->() const {
            return &(inner_->first);
        }
        
        const_iterator& operator++() {
            ++inner_;
            return *this;
        }
        
        bool operator!=(const const_iterator& other) const {
            return inner_ != other.inner_;
        }
        
        bool operator==(const const_iterator& other) const {
            return inner_ == other.inner_;
        }
    };
    
    // @lifetime: (&'a) -> &'a
    iterator begin() {
        return iterator(map_.begin());
    }
    
    // @lifetime: (&'a) -> &'a
    iterator end() {
        return iterator(map_.end());
    }
    
    // @lifetime: (&'a) -> &'a
    const_iterator begin() const {
        return const_iterator(map_.begin());
    }
    
    // @lifetime: (&'a) -> &'a
    const_iterator end() const {
        return const_iterator(map_.end());
    }
    
    // Convert to Vec (in sorted order)
    // @lifetime: owned
    Vec<T> to_vec() const {
        Vec<T> result = Vec<T>::with_capacity(map_.len());
        for (const auto& [key, _] : map_) {
            result.push(key);
        }
        return result;
    }
    
    // Equality comparison
    bool operator==(const BTreeSet& other) const {
        return map_ == other.map_;
    }
    
    bool operator!=(const BTreeSet& other) const {
        return map_ != other.map_;
    }
};

// Factory functions
template<typename T>
// @lifetime: owned
BTreeSet<T> btreeset() {
    return BTreeSet<T>::make();
}

// Create BTreeSet from Vec
template<typename T>
// @lifetime: owned
BTreeSet<T> btreeset_from_vec(Vec<T> vec) {
    BTreeSet<T> set;
    for (size_t i = 0; i < vec.len(); i++) {
        set.insert(std::move(vec[i]));
    }
    return set;
}

// Note: For std::vector compatibility, include <vector> and use:
// template<typename T>
// BTreeSet<T> btreeset_from_std_vec(std::vector<T> vec) {
//     BTreeSet<T> set;
//     for (auto& elem : vec) {
//         set.insert(std::move(elem));
//     }
//     return set;
// }

} // namespace rusty

#endif // RUSTY_BTREESET_HPP