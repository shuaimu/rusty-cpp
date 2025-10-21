#ifndef RUSTY_HASHSET_HPP
#define RUSTY_HASHSET_HPP

#include "hashmap.hpp"
#include "vec.hpp"
#include <functional>

// @safe
namespace rusty {

// HashSet implemented as a thin wrapper around HashMap<T, ()>
// Uses unit type () as value, represented here as an empty struct
template <typename T, typename Hash = std::hash<T>>
class HashSet {
private:
    struct Unit {  // Empty struct to represent Rust's unit type ()
        bool operator==(const Unit&) const { return true; }
        bool operator!=(const Unit&) const { return false; }
    };
    HashMap<T, Unit, Hash> map_;
    
public:
    // Constructors
    HashSet() = default;
    
    // @lifetime: owned
    static auto make() {
        return HashSet();
    }
    
    // @lifetime: owned
    static HashSet with_capacity(size_t cap) {
        HashSet set;
        set.map_ = HashMap<T, Unit, Hash>::with_capacity(cap);
        return set;
    }
    
    // Move constructor
    HashSet(HashSet&& other) noexcept : map_(std::move(other.map_)) {}
    
    // Move assignment
    HashSet& operator=(HashSet&& other) noexcept {
        if (this != &other) {
            map_ = std::move(other.map_);
        }
        return *this;
    }
    
    // Delete copy operations
    HashSet(const HashSet&) = delete;
    HashSet& operator=(const HashSet&) = delete;
    
    // Clone for explicit copying
    // @lifetime: owned
    HashSet clone() const {
        HashSet result;
        result.map_ = map_.clone();
        return result;
    }
    
    // Size and capacity
    size_t len() const { return map_.len(); }
    size_t capacity() const { return map_.capacity(); }
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
                return Option<const T*>(Some(&key));
            }
        }
        return Option<const T*>(None);
    }
    
    // Take element (remove and return)
    // @lifetime: owned
    Option<T> take(const T& value) {
        // Since we can't directly extract the key from HashMap,
        // we need to check if it exists first
        if (map_.contains_key(value)) {
            map_.remove(value);
            // Create a copy of the value to return
            return Some(value);
        }
        return None;
    }
    
    // Replace element (returns old value if existed)
    // @lifetime: owned
    Option<T> replace(T value) {
        if (map_.contains_key(value)) {
            T old = value;  // Copy for return
            map_.insert(std::move(value), Unit{});
            return Some(old);
        } else {
            map_.insert(std::move(value), Unit{});
            return None;
        }
    }
    
    // Set operations
    
    // Union: self ∪ other (all elements from both sets)
    // @lifetime: owned
    HashSet union_with(const HashSet& other) const {
        HashSet result = this->clone();
        for (auto [key, _] : other.map_) {
            result.insert(key);
        }
        return result;
    }
    
    // Intersection: self ∩ other (elements in both sets)
    // @lifetime: owned
    HashSet intersection(const HashSet& other) const {
        HashSet result;
        // Iterate over smaller set for efficiency
        const HashSet* smaller = this->len() <= other.len() ? this : &other;
        const HashSet* larger = this->len() <= other.len() ? &other : this;
        
        for (auto [key, _] : smaller->map_) {
            if (larger->contains(key)) {
                result.insert(key);
            }
        }
        return result;
    }
    
    // Difference: self - other (elements in self but not in other)
    // @lifetime: owned
    HashSet difference(const HashSet& other) const {
        HashSet result;
        for (auto [key, _] : map_) {
            if (!other.contains(key)) {
                result.insert(key);
            }
        }
        return result;
    }
    
    // Symmetric difference: self △ other (elements in either but not both)
    // @lifetime: owned
    HashSet symmetric_difference(const HashSet& other) const {
        HashSet result;
        for (auto [key, _] : map_) {
            if (!other.contains(key)) {
                result.insert(key);
            }
        }
        for (auto [key, _] : other.map_) {
            if (!this->contains(key)) {
                result.insert(key);
            }
        }
        return result;
    }
    
    // Check if disjoint (no common elements)
    bool is_disjoint(const HashSet& other) const {
        // Check smaller set against larger
        const HashSet* smaller = this->len() <= other.len() ? this : &other;
        const HashSet* larger = this->len() <= other.len() ? &other : this;
        
        for (auto [key, _] : smaller->map_) {
            if (larger->contains(key)) {
                return false;
            }
        }
        return true;
    }
    
    // Check if subset
    bool is_subset(const HashSet& other) const {
        if (this->len() > other.len()) {
            return false;
        }
        for (auto [key, _] : map_) {
            if (!other.contains(key)) {
                return false;
            }
        }
        return true;
    }
    
    // Check if superset
    bool is_superset(const HashSet& other) const {
        return other.is_subset(*this);
    }
    
    // Extend from another set
    void extend(HashSet&& other) {
        map_.extend(std::move(other.map_));
    }
    
    // Retain only elements matching predicate
    template<typename Pred>
    void retain(Pred pred) {
        map_.retain([&pred](const T& key, const Unit&) {
            return pred(key);
        });
    }
    
    // Drain all elements into a Vec
    // @lifetime: owned
    Vec<T> drain() {
        Vec<T> result = Vec<T>::with_capacity(map_.len());
        for (auto [key, _] : map_) {
            result.push(key);
        }
        map_.clear();
        return result;
    }
    
    // Iterator support (iterate over keys only)
    class iterator {
    private:
        typename HashMap<T, Unit, Hash>::iterator inner_;
        
    public:
        iterator(typename HashMap<T, Unit, Hash>::iterator it) : inner_(it) {}
        
        const T& operator*() {
            return (*inner_).first;
        }
        
        const T* operator->() {
            return &((*inner_).first);
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
        typename HashMap<T, Unit, Hash>::const_iterator inner_;
        
    public:
        const_iterator(typename HashMap<T, Unit, Hash>::const_iterator it) : inner_(it) {}
        
        const T& operator*() const {
            return (*inner_).first;
        }
        
        const T* operator->() const {
            return &((*inner_).first);
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
    
    // Convert to Vec
    // @lifetime: owned
    Vec<T> to_vec() const {
        Vec<T> result = Vec<T>::with_capacity(map_.len());
        for (auto [key, _] : map_) {
            result.push(key);
        }
        return result;
    }
    
    // Equality comparison
    bool operator==(const HashSet& other) const {
        if (this->len() != other.len()) {
            return false;
        }
        for (auto [key, _] : map_) {
            if (!other.contains(key)) {
                return false;
            }
        }
        return true;
    }
    
    bool operator!=(const HashSet& other) const {
        return !(*this == other);
    }
};

// Factory functions
template<typename T>
// @lifetime: owned
HashSet<T> hashset() {
    return HashSet<T>::make();
}

template<typename T>
// @lifetime: owned
HashSet<T> hashset_with_capacity(size_t cap) {
    return HashSet<T>::with_capacity(cap);
}

// Create HashSet from Vec
template<typename T>
// @lifetime: owned
HashSet<T> hashset_from_vec(Vec<T> vec) {
    HashSet<T> set;
    for (size_t i = 0; i < vec.len(); i++) {
        set.insert(std::move(vec[i]));
    }
    return set;
}

// Note: For std::vector compatibility, include <vector> and use:
// template<typename T>
// HashSet<T> hashset_from_std_vec(std::vector<T> vec) {
//     HashSet<T> set;
//     for (auto& elem : vec) {
//         set.insert(std::move(elem));
//     }
//     return set;
// }

} // namespace rusty

#endif // RUSTY_HASHSET_HPP