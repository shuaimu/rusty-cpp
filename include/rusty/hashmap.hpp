#ifndef RUSTY_HASHMAP_HPP
#define RUSTY_HASHMAP_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <utility>
#include <functional>
#include <immintrin.h>  // For SSE2/AVX2 intrinsics
#include "option.hpp"
#include "vec.hpp"

// Swiss Table HashMap - High-performance hash table based on Google's SwissTable/Abseil
// and Rust's HashMap implementation
//
// Key features:
// - Metadata bytes for fast probing (H2 hash)
// - SIMD acceleration for parallel metadata comparison
// - Quadratic probing for better cache locality
// - Robin Hood hashing for reduced probe distances
// - Load factor of 7/8 for better memory efficiency

// @safe
namespace rusty {

// Control byte values
constexpr uint8_t EMPTY = 0b11111111;     // 0xFF - empty slot
constexpr uint8_t DELETED = 0b10000000;   // 0x80 - tombstone
constexpr uint8_t SENTINEL = 0b11111110;  // 0xFE - end of table sentinel

// Helper to check if control byte represents empty or deleted
inline bool is_empty_or_deleted(uint8_t ctrl) {
    // Empty: 11111111, Deleted: 10000000
    // Both have high bit set and differ in low bits
    return ctrl >= DELETED;
}

// Helper to check if control byte represents a full slot
inline bool is_full(uint8_t ctrl) {
    return ctrl < DELETED;
}

// Group size for SIMD operations (16 bytes for SSE2)
constexpr size_t GROUP_SIZE = 16;

// Portable SIMD group operations
struct Group {
    uint8_t bytes[GROUP_SIZE];
    
    // Load group from control bytes
    static Group load(const uint8_t* ctrl) {
        Group g;
        std::memcpy(g.bytes, ctrl, GROUP_SIZE);
        return g;
    }
    
    // Match bytes equal to value (returns bitmask)
    uint32_t match_byte(uint8_t value) const {
        uint32_t mask = 0;
        
#ifdef __SSE2__
        // SSE2 accelerated version
        __m128i group = _mm_loadu_si128(reinterpret_cast<const __m128i*>(bytes));
        __m128i match = _mm_set1_epi8(value);
        __m128i cmp = _mm_cmpeq_epi8(group, match);
        mask = _mm_movemask_epi8(cmp);
#else
        // Portable fallback
        for (size_t i = 0; i < GROUP_SIZE; i++) {
            if (bytes[i] == value) {
                mask |= (1u << i);
            }
        }
#endif
        return mask;
    }
    
    // Match empty slots (EMPTY or DELETED)
    uint32_t match_empty_or_deleted() const {
        uint32_t mask = 0;
        
#ifdef __SSE2__
        // SSE2 version: check if high bit is set
        __m128i group = _mm_loadu_si128(reinterpret_cast<const __m128i*>(bytes));
        __m128i high_bit = _mm_set1_epi8(0x80);
        __m128i cmp = _mm_and_si128(group, high_bit);
        mask = _mm_movemask_epi8(cmp);
#else
        // Portable fallback
        for (size_t i = 0; i < GROUP_SIZE; i++) {
            if (is_empty_or_deleted(bytes[i])) {
                mask |= (1u << i);
            }
        }
#endif
        return mask;
    }
};

// Extract H1 hash (for bucket index)
inline size_t h1_hash(size_t hash) {
    return hash;
}

// Extract H2 hash (for metadata - 7 bits)
inline uint8_t h2_hash(size_t hash) {
    // Use top 7 bits of hash, ensure it's never >= 0x80
    return (hash >> 57) & 0x7F;
}

// Next power of 2 that is >= n and >= 16
inline size_t capacity_to_buckets(size_t n) {
    if (n < 16) return 16;
    
    // Round up to next power of 2
    n--;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n |= n >> 32;
    n++;
    
    return n;
}

// ProbeSeq generates probe indices using quadratic probing
struct ProbeSeq {
    size_t mask;
    size_t pos;
    size_t stride;
    
    ProbeSeq(size_t hash, size_t mask) : mask(mask), pos(h1_hash(hash) & mask), stride(0) {}
    
    size_t offset() const { return pos; }
    
    void next() {
        stride += GROUP_SIZE;
        pos = (pos + stride) & mask;
    }
};

template <typename K, typename V, typename Hash = std::hash<K>, typename KeyEqual = std::equal_to<K>>
class HashMap {
private:
    // Bucket structure
    struct Bucket {
        // We use separate arrays for better cache locality
        // Instead of array of structs, we have struct of arrays
    };
    
    uint8_t* ctrl_;          // Control bytes (metadata)
    K* keys_;                // Keys array
    V* values_;              // Values array
    size_t bucket_mask_;     // Capacity - 1 (for fast modulo)
    size_t size_;            // Number of elements
    size_t growth_left_;     // Number of elements we can add before resize
    Hash hasher_;
    KeyEqual key_eq_;
    
    // Find the insert position for a key
    struct FindResult {
        size_t index;
        bool found;
    };
    
    FindResult find_insert_slot(const K& key) const {
        size_t hash = hasher_(key);
        uint8_t h2 = h2_hash(hash);
        ProbeSeq seq(hash, bucket_mask_);
        
        while (true) {
            Group g = Group::load(&ctrl_[seq.offset()]);
            
            // Check for matching H2 in this group
            uint32_t matches = g.match_byte(h2);
            while (matches) {
                size_t bit = __builtin_ctz(matches);  // Count trailing zeros
                size_t index = (seq.offset() + bit) & bucket_mask_;
                
                if (key_eq_(keys_[index], key)) {
                    return {index, true};
                }
                
                matches &= matches - 1;  // Clear lowest bit
            }
            
            // Check for empty slot in this group
            uint32_t empties = g.match_byte(EMPTY);
            if (empties) {
                size_t bit = __builtin_ctz(empties);
                size_t index = (seq.offset() + bit) & bucket_mask_;
                return {index, false};
            }
            
            seq.next();
        }
    }
    
    // Find existing key
    size_t find_key(const K& key) const {
        if (size_ == 0) return static_cast<size_t>(-1);
        
        size_t hash = hasher_(key);
        uint8_t h2 = h2_hash(hash);
        ProbeSeq seq(hash, bucket_mask_);
        
        while (true) {
            Group g = Group::load(&ctrl_[seq.offset()]);
            
            // Check for matching H2
            uint32_t matches = g.match_byte(h2);
            while (matches) {
                size_t bit = __builtin_ctz(matches);
                size_t index = (seq.offset() + bit) & bucket_mask_;
                
                if (key_eq_(keys_[index], key)) {
                    return index;
                }
                
                matches &= matches - 1;
            }
            
            // If we see any empty (not deleted), key doesn't exist
            if (g.match_byte(EMPTY)) {
                return static_cast<size_t>(-1);
            }
            
            seq.next();
        }
    }
    
    // Allocate storage for given capacity
    void allocate(size_t capacity) {
        size_t buckets = capacity_to_buckets(capacity);
        bucket_mask_ = buckets - 1;
        
        // Allocate control bytes with extra GROUP_SIZE for sentinel
        ctrl_ = static_cast<uint8_t*>(::operator new(buckets + GROUP_SIZE));
        std::memset(ctrl_, EMPTY, buckets + GROUP_SIZE);
        
        // Allocate keys and values
        keys_ = static_cast<K*>(::operator new(buckets * sizeof(K)));
        values_ = static_cast<V*>(::operator new(buckets * sizeof(V)));
        
        // Set growth_left to 7/8 of capacity
        growth_left_ = buckets - buckets / 8;
    }
    
    // Deallocate all storage
    void deallocate() {
        if (ctrl_) {
            // Destroy live elements
            size_t capacity = bucket_mask_ + 1;
            for (size_t i = 0; i < capacity; i++) {
                if (is_full(ctrl_[i])) {
                    keys_[i].~K();
                    values_[i].~V();
                }
            }
            
            ::operator delete(ctrl_);
            ::operator delete(keys_);
            ::operator delete(values_);
            
            ctrl_ = nullptr;
            keys_ = nullptr;
            values_ = nullptr;
        }
    }
    
    // Resize the table
    void resize() {
        size_t old_capacity = bucket_mask_ + 1;
        uint8_t* old_ctrl = ctrl_;
        K* old_keys = keys_;
        V* old_values = values_;
        
        // Allocate new larger table
        size_t new_capacity = old_capacity * 2;
        allocate(new_capacity);
        size_ = 0;
        
        // Rehash all elements
        for (size_t i = 0; i < old_capacity; i++) {
            if (is_full(old_ctrl[i])) {
                insert_unique_unchecked(
                    std::move(old_keys[i]),
                    std::move(old_values[i])
                );
                old_keys[i].~K();
                old_values[i].~V();
            }
        }
        
        // Free old storage
        ::operator delete(old_ctrl);
        ::operator delete(old_keys);
        ::operator delete(old_values);
    }
    
    // Insert without checking for duplicates (for resize)
    void insert_unique_unchecked(K key, V value) {
        size_t hash = hasher_(key);
        uint8_t h2 = h2_hash(hash);
        ProbeSeq seq(hash, bucket_mask_);
        
        while (true) {
            Group g = Group::load(&ctrl_[seq.offset()]);
            uint32_t empties = g.match_empty_or_deleted();
            
            if (empties) {
                size_t bit = __builtin_ctz(empties);
                size_t index = (seq.offset() + bit) & bucket_mask_;
                
                ctrl_[index] = h2;
                new (&keys_[index]) K(std::move(key));
                new (&values_[index]) V(std::move(value));
                size_++;
                growth_left_--;
                
                // Mirror the control byte for wraparound
                if (index < GROUP_SIZE) {
                    ctrl_[index + bucket_mask_ + 1] = h2;
                }
                
                return;
            }
            
            seq.next();
        }
    }
    
public:
    // Constructors
    HashMap() : ctrl_(nullptr), keys_(nullptr), values_(nullptr),
                     bucket_mask_(0), size_(0), growth_left_(0) {
        allocate(16);  // Start with minimum size
    }
    
    explicit HashMap(size_t capacity) 
        : ctrl_(nullptr), keys_(nullptr), values_(nullptr),
          bucket_mask_(0), size_(0), growth_left_(0) {
        allocate(capacity);
    }
    
    // Factory method - HashMap::make()
    // @lifetime: owned
    static HashMap make() {
        return HashMap();
    }
    
    // @lifetime: owned
    static HashMap with_capacity(size_t cap) {
        return HashMap(cap);
    }
    
    // Move constructor
    HashMap(HashMap&& other) noexcept
        : ctrl_(other.ctrl_), keys_(other.keys_), values_(other.values_),
          bucket_mask_(other.bucket_mask_), size_(other.size_),
          growth_left_(other.growth_left_),
          hasher_(std::move(other.hasher_)),
          key_eq_(std::move(other.key_eq_)) {
        other.ctrl_ = nullptr;
        other.keys_ = nullptr;
        other.values_ = nullptr;
        other.size_ = 0;
        other.bucket_mask_ = 0;
        other.growth_left_ = 0;
    }
    
    // Move assignment
    HashMap& operator=(HashMap&& other) noexcept {
        if (this != &other) {
            deallocate();
            
            ctrl_ = other.ctrl_;
            keys_ = other.keys_;
            values_ = other.values_;
            bucket_mask_ = other.bucket_mask_;
            size_ = other.size_;
            growth_left_ = other.growth_left_;
            hasher_ = std::move(other.hasher_);
            key_eq_ = std::move(other.key_eq_);
            
            other.ctrl_ = nullptr;
            other.keys_ = nullptr;
            other.values_ = nullptr;
            other.size_ = 0;
            other.bucket_mask_ = 0;
            other.growth_left_ = 0;
        }
        return *this;
    }
    
    // Delete copy operations
    HashMap(const HashMap&) = delete;
    HashMap& operator=(const HashMap&) = delete;
    
    // Destructor
    ~HashMap() {
        deallocate();
    }
    
    // Size and capacity
    size_t len() const { return size_; }
    bool is_empty() const { return size_ == 0; }
    size_t capacity() const { return bucket_mask_ + 1; }
    
    // Clear all elements
    void clear() {
        size_t capacity = bucket_mask_ + 1;
        for (size_t i = 0; i < capacity; i++) {
            if (is_full(ctrl_[i])) {
                keys_[i].~K();
                values_[i].~V();
                ctrl_[i] = EMPTY;
            }
        }
        
        // Reset sentinel bytes
        std::memset(&ctrl_[capacity], EMPTY, GROUP_SIZE);
        
        size_ = 0;
        growth_left_ = capacity - capacity / 8;
    }
    
    // Insert or update
    void insert(K key, V value) {
        if (growth_left_ == 0) {
            resize();
        }
        
        auto result = find_insert_slot(key);
        
        if (result.found) {
            // Update existing value
            values_[result.index] = std::move(value);
        } else {
            // Insert new entry
            size_t hash = hasher_(key);
            uint8_t h2 = h2_hash(hash);
            
            ctrl_[result.index] = h2;
            new (&keys_[result.index]) K(std::move(key));
            new (&values_[result.index]) V(std::move(value));
            
            // Mirror control byte for wraparound
            if (result.index < GROUP_SIZE) {
                ctrl_[result.index + bucket_mask_ + 1] = h2;
            }
            
            size_++;
            growth_left_--;
        }
    }
    
    // Get value by key
    // @lifetime: (&'a) -> &'a
    Option<V*> get(const K& key) {
        size_t index = find_key(key);
        if (index != static_cast<size_t>(-1)) {
            return Some(&values_[index]);
        }
        return None;
    }
    
    // @lifetime: (&'a) -> &'a
    Option<const V*> get(const K& key) const {
        size_t index = find_key(key);
        if (index != static_cast<size_t>(-1)) {
            return Option<const V*>(Some(const_cast<const V*>(&values_[index])));
        }
        return Option<const V*>(None);
    }
    
    // Get mutable reference
    // @lifetime: (&'a mut) -> &'a mut
    Option<V*> get_mut(const K& key) {
        size_t index = find_key(key);
        if (index != static_cast<size_t>(-1)) {
            return Some(&values_[index]);
        }
        return None;
    }
    
    // Remove by key
    // @lifetime: owned
    Option<V> remove(const K& key) {
        size_t index = find_key(key);
        if (index != static_cast<size_t>(-1)) {
            V value = std::move(values_[index]);
            keys_[index].~K();
            values_[index].~V();
            ctrl_[index] = DELETED;
            
            // Mirror control byte for wraparound
            if (index < GROUP_SIZE) {
                ctrl_[index + bucket_mask_ + 1] = DELETED;
            }
            
            size_--;
            return Some(std::move(value));
        }
        return None;
    }
    
    // Check if key exists
    bool contains_key(const K& key) const {
        return find_key(key) != static_cast<size_t>(-1);
    }
    
    // Entry API for get-or-insert
    // @lifetime: (&'a mut) -> &'a mut
    V& entry(K key) {
        if (growth_left_ == 0) {
            resize();
        }
        
        auto result = find_insert_slot(key);
        
        if (!result.found) {
            size_t hash = hasher_(key);
            uint8_t h2 = h2_hash(hash);
            
            ctrl_[result.index] = h2;
            new (&keys_[result.index]) K(std::move(key));
            new (&values_[result.index]) V();
            
            if (result.index < GROUP_SIZE) {
                ctrl_[result.index + bucket_mask_ + 1] = h2;
            }
            
            size_++;
            growth_left_--;
        }
        
        return values_[result.index];
    }
    
    // Get or insert with value
    // @lifetime: (&'a mut) -> &'a mut
    V& or_insert(K key, V default_value) {
        if (growth_left_ == 0) {
            resize();
        }
        
        auto result = find_insert_slot(key);
        
        if (!result.found) {
            size_t hash = hasher_(key);
            uint8_t h2 = h2_hash(hash);
            
            ctrl_[result.index] = h2;
            new (&keys_[result.index]) K(std::move(key));
            new (&values_[result.index]) V(std::move(default_value));
            
            if (result.index < GROUP_SIZE) {
                ctrl_[result.index + bucket_mask_ + 1] = h2;
            }
            
            size_++;
            growth_left_--;
        }
        
        return values_[result.index];
    }
    
    // Get key-value pair
    // @lifetime: (&'a) -> &'a
    Option<std::pair<const K*, const V*>> get_key_value(const K& key) const {
        size_t index = find_key(key);
        if (index != static_cast<size_t>(-1)) {
            return Option<std::pair<const K*, const V*>>(
                Some(std::make_pair(&keys_[index], &values_[index]))
            );
        }
        return Option<std::pair<const K*, const V*>>(None);
    }
    
    // Remove entry and return both key and value
    // @lifetime: owned
    Option<std::pair<K, V>> remove_entry(const K& key) {
        size_t index = find_key(key);
        if (index != static_cast<size_t>(-1)) {
            K k = std::move(keys_[index]);
            V v = std::move(values_[index]);
            keys_[index].~K();
            values_[index].~V();
            ctrl_[index] = DELETED;
            
            if (index < GROUP_SIZE) {
                ctrl_[index + bucket_mask_ + 1] = DELETED;
            }
            
            size_--;
            return Some(std::make_pair(std::move(k), std::move(v)));
        }
        return None;
    }
    
    // Extend from another map
    void extend(HashMap&& other) {
        size_t cap = other.bucket_mask_ + 1;
        for (size_t i = 0; i < cap; i++) {
            if (is_full(other.ctrl_[i])) {
                insert(std::move(other.keys_[i]), std::move(other.values_[i]));
            }
        }
        other.clear();
    }
    
    // Retain only entries matching predicate
    template<typename Pred>
    void retain(Pred pred) {
        size_t cap = bucket_mask_ + 1;
        for (size_t i = 0; i < cap; i++) {
            if (is_full(ctrl_[i]) && !pred(keys_[i], values_[i])) {
                keys_[i].~K();
                values_[i].~V();
                ctrl_[i] = DELETED;
                
                if (i < GROUP_SIZE) {
                    ctrl_[i + bucket_mask_ + 1] = DELETED;
                }
                
                size_--;
            }
        }
    }
    
    // Clone for explicit copying
    // @lifetime: owned
    HashMap clone() const {
        HashMap result(capacity());
        
        size_t cap = bucket_mask_ + 1;
        for (size_t i = 0; i < cap; i++) {
            if (is_full(ctrl_[i])) {
                result.insert(keys_[i], values_[i]);
            }
        }
        
        return result;
    }
    
    // Iterator support
    class iterator {
    private:
        HashMap* map_;
        size_t index_;
        
        void advance_to_next_full() {
            size_t capacity = map_->bucket_mask_ + 1;
            while (index_ < capacity && !is_full(map_->ctrl_[index_])) {
                index_++;
            }
        }
        
    public:
        iterator(HashMap* m, size_t i) : map_(m), index_(i) {
            advance_to_next_full();
        }
        
        std::pair<K&, V&> operator*() {
            return {map_->keys_[index_], map_->values_[index_]};
        }
        
        std::pair<K*, V*> operator->() {
            return {&map_->keys_[index_], &map_->values_[index_]};
        }
        
        iterator& operator++() {
            index_++;
            advance_to_next_full();
            return *this;
        }
        
        bool operator!=(const iterator& other) const {
            return index_ != other.index_;
        }
        
        bool operator==(const iterator& other) const {
            return index_ == other.index_;
        }
    };
    
    class const_iterator {
    private:
        const HashMap* map_;
        size_t index_;
        
        void advance_to_next_full() {
            size_t capacity = map_->bucket_mask_ + 1;
            while (index_ < capacity && !is_full(map_->ctrl_[index_])) {
                index_++;
            }
        }
        
    public:
        const_iterator(const HashMap* m, size_t i) : map_(m), index_(i) {
            advance_to_next_full();
        }
        
        std::pair<const K&, const V&> operator*() const {
            return {map_->keys_[index_], map_->values_[index_]};
        }
        
        std::pair<const K*, const V*> operator->() const {
            return {&map_->keys_[index_], &map_->values_[index_]};
        }
        
        const_iterator& operator++() {
            index_++;
            advance_to_next_full();
            return *this;
        }
        
        bool operator!=(const const_iterator& other) const {
            return index_ != other.index_;
        }
        
        bool operator==(const const_iterator& other) const {
            return index_ == other.index_;
        }
    };
    
    // @lifetime: (&'a) -> &'a
    iterator begin() {
        return iterator(this, 0);
    }
    
    // @lifetime: (&'a) -> &'a
    iterator end() {
        return iterator(this, bucket_mask_ + 1);
    }
    
    // @lifetime: (&'a) -> &'a
    const_iterator begin() const {
        return const_iterator(this, 0);
    }
    
    // @lifetime: (&'a) -> &'a
    const_iterator end() const {
        return const_iterator(this, bucket_mask_ + 1);
    }
    
    // Collect all keys
    // @lifetime: owned
    Vec<K> keys() const {
        Vec<K> result = Vec<K>::with_capacity(size_);
        size_t capacity = bucket_mask_ + 1;
        
        for (size_t i = 0; i < capacity; i++) {
            if (is_full(ctrl_[i])) {
                result.push(keys_[i]);
            }
        }
        
        return result;
    }
    
    // Collect all values
    // @lifetime: owned
    Vec<V> values() const {
        Vec<V> result = Vec<V>::with_capacity(size_);
        size_t capacity = bucket_mask_ + 1;
        
        for (size_t i = 0; i < capacity; i++) {
            if (is_full(ctrl_[i])) {
                result.push(values_[i]);
            }
        }
        
        return result;
    }
    
    // Equality comparison
    bool operator==(const HashMap& other) const {
        if (size_ != other.size_) return false;
        
        size_t capacity = bucket_mask_ + 1;
        for (size_t i = 0; i < capacity; i++) {
            if (is_full(ctrl_[i])) {
                auto other_val = other.get(keys_[i]);
                if (!other_val.is_some() || *other_val.unwrap() != values_[i]) {
                    return false;
                }
            }
        }
        
        return true;
    }
    
    bool operator!=(const HashMap& other) const {
        return !(*this == other);
    }
};

// Factory function
template<typename K, typename V>
// @lifetime: owned
HashMap<K, V> hashmap() {
    return HashMap<K, V>::make();
}

} // namespace rusty

#endif // RUSTY_HASHMAP_HPP