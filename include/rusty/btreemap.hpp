#ifndef RUSTY_BTREEMAP_HPP
#define RUSTY_BTREEMAP_HPP

#include <algorithm>
#include <functional>
#include <memory>
#include <utility>
#include <cassert>
#include "option.hpp"
#include "vec.hpp"
#include "maybe_uninit.hpp"

// BTreeMap<K, V> - A B-Tree based ordered map
// Equivalent to Rust's std::collections::BTreeMap
//
// Key design decisions following Rust's implementation:
// - B = 6: Each node has 5-11 keys (except root: 0-11 keys)
// - Uses MaybeUninit/UninitArray for keys/values - NO default constructor required
// - Nodes are either internal (with children) or leaves
// - Keys and values stored contiguously for cache locality
// - Node splitting/merging maintains B-tree invariants
//
// This implementation does NOT require K or V to be default-constructible,
// making it compatible with types like Rc<T>, Box<T>, etc.

// @safe
namespace rusty {

// B-tree constants matching Rust's implementation
inline constexpr size_t BTREE_B = 6;                      // Branching factor
inline constexpr size_t BTREE_CAPACITY = 2 * BTREE_B - 1; // 11: Maximum keys in any node
inline constexpr size_t BTREE_MIN_LEN = BTREE_B - 1;      // 5: Minimum keys in non-root node

template <typename K, typename V, typename Compare = std::less<K>>
class BTreeMap {
private:
    // Forward declarations
    struct LeafNode;
    struct InternalNode;

    // Node types use UninitArray - no default constructor required for K or V

    // LeafNode: Contains keys and values, no children
    struct LeafNode {
        UninitArray<K, BTREE_CAPACITY> keys;
        UninitArray<V, BTREE_CAPACITY> vals;
        uint16_t len;

        // Linked list for iteration (optional optimization)
        LeafNode* next;
        LeafNode* prev;

        LeafNode() : len(0), next(nullptr), prev(nullptr) {}

        ~LeafNode() {
            // Destroy only initialized elements
            keys.destroy_range(len);
            vals.destroy_range(len);
        }

        // No copy - move only
        LeafNode(const LeafNode&) = delete;
        LeafNode& operator=(const LeafNode&) = delete;

        bool is_full() const { return len >= BTREE_CAPACITY; }
        bool is_underfull() const { return len < BTREE_MIN_LEN; }

        // Binary search for key position
        // Returns the index where key should be (or is)
        template<typename Comp>
        size_t search_key(const K& key, const Comp& comp) const {
            size_t left = 0;
            size_t right = len;
            while (left < right) {
                size_t mid = left + (right - left) / 2;
                if (comp(keys[mid], key)) {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            }
            return left;
        }

        // Check if key at position equals the search key
        template<typename Comp>
        bool key_eq(size_t pos, const K& key, const Comp& comp) const {
            return pos < len && !comp(keys[pos], key) && !comp(key, keys[pos]);
        }

        // Insert key-value at position (assumes space available)
        void insert_at(size_t pos, K key, V val) {
            assert(len < BTREE_CAPACITY);
            // Shift existing elements right
            keys.shift_right(pos, len - pos);
            vals.shift_right(pos, len - pos);
            // Construct new elements
            keys.construct_at(pos, std::move(key));
            vals.construct_at(pos, std::move(val));
            ++len;
        }

        // Remove key-value at position, returns the value
        V remove_at(size_t pos) {
            assert(pos < len);
            V val = std::move(vals[pos]);
            keys.destroy_at(pos);
            vals.destroy_at(pos);
            // Shift remaining elements left
            keys.shift_left(pos, len - pos - 1);
            vals.shift_left(pos, len - pos - 1);
            --len;
            return val;
        }

        // Split leaf: moves right half to new leaf, returns (new_leaf, median_key)
        std::pair<LeafNode*, K> split() {
            size_t mid = len / 2;
            LeafNode* right = new LeafNode();

            // Move right half to new node
            for (size_t i = mid; i < len; ++i) {
                right->keys.construct_at(right->len, std::move(keys[i]));
                right->vals.construct_at(right->len, std::move(vals[i]));
                keys.destroy_at(i);
                vals.destroy_at(i);
                ++right->len;
            }

            // Median key goes up to parent (copy first key of right)
            K median = right->keys[0];  // Copy
            len = mid;

            // Update linked list
            right->next = next;
            right->prev = this;
            if (next) next->prev = right;
            next = right;

            return {right, std::move(median)};
        }

        // Merge with right sibling (this absorbs right)
        void merge_with_right(LeafNode* right) {
            for (size_t i = 0; i < right->len; ++i) {
                keys.construct_at(len, std::move(right->keys[i]));
                vals.construct_at(len, std::move(right->vals[i]));
                right->keys.destroy_at(i);
                right->vals.destroy_at(i);
                ++len;
            }
            right->len = 0;

            // Update linked list
            next = right->next;
            if (right->next) right->next->prev = this;
        }
    };

    // InternalNode: Contains keys and child pointers
    // Following Rust's design: internal node "contains" a leaf node conceptually
    struct InternalNode {
        UninitArray<K, BTREE_CAPACITY> keys;
        void* edges[BTREE_CAPACITY + 1]; // Child pointers (simple array, no init needed)
        uint16_t len;      // Number of keys (edges = len + 1)
        bool children_are_leaves;

        InternalNode(bool leaves) : len(0), children_are_leaves(leaves) {
            // Initialize all edges to nullptr for safety
            for (size_t i = 0; i < BTREE_CAPACITY + 1; ++i) {
                edges[i] = nullptr;
            }
        }

        ~InternalNode() {
            keys.destroy_range(len);
            // Destroy edges - need to cast to proper type
            for (size_t i = 0; i <= len; ++i) {
                void* edge = edges[i];
                if (edge) {
                    if (children_are_leaves) {
                        delete static_cast<LeafNode*>(edge);
                    } else {
                        delete static_cast<InternalNode*>(edge);
                    }
                }
            }
        }

        InternalNode(const InternalNode&) = delete;
        InternalNode& operator=(const InternalNode&) = delete;

        bool is_full() const { return len >= BTREE_CAPACITY; }
        bool is_underfull() const { return len < BTREE_MIN_LEN; }

        template<typename Comp>
        size_t search_key(const K& key, const Comp& comp) const {
            size_t left = 0;
            size_t right = len;
            while (left < right) {
                size_t mid = left + (right - left) / 2;
                if (comp(keys[mid], key)) {
                    left = mid + 1;
                } else {
                    right = mid;
                }
            }
            return left;
        }

        // Get child at index (returns void*)
        void* child_at(size_t i) const {
            assert(i <= len);
            return edges[i];
        }

        LeafNode* leaf_child_at(size_t i) const {
            assert(children_are_leaves);
            return static_cast<LeafNode*>(edges[i]);
        }

        InternalNode* internal_child_at(size_t i) const {
            assert(!children_are_leaves);
            return static_cast<InternalNode*>(edges[i]);
        }

        // Insert key and right child at position
        void insert_at(size_t pos, K key, void* right_child) {
            assert(len < BTREE_CAPACITY);
            // Shift keys right
            keys.shift_right(pos, len - pos);
            // Shift edges right (from pos+1)
            for (size_t i = len; i > pos; --i) {
                edges[i + 1] = edges[i];
            }
            // Insert new key and edge
            keys.construct_at(pos, std::move(key));
            edges[pos + 1] = right_child;
            ++len;
        }

        // Split internal node
        std::pair<InternalNode*, K> split() {
            size_t mid = len / 2;
            InternalNode* right = new InternalNode(children_are_leaves);

            // Move right half keys (excluding median)
            for (size_t i = mid + 1; i < len; ++i) {
                right->keys.construct_at(right->len, std::move(keys[i]));
                keys.destroy_at(i);
                ++right->len;
            }

            // Move right half edges
            for (size_t i = mid + 1; i <= len; ++i) {
                right->edges[i - mid - 1] = edges[i];
            }

            // Extract median key
            K median = std::move(keys[mid]);
            keys.destroy_at(mid);

            len = mid;
            return {right, std::move(median)};
        }
    };

    // Root can be either a leaf or internal node
    enum class RootType { Empty, Leaf, Internal };

    union RootNode {
        LeafNode* leaf;
        InternalNode* internal;

        RootNode() : leaf(nullptr) {}
    };

    RootNode root_;
    RootType root_type_;
    size_t size_;
    Compare comp_;
    LeafNode* first_leaf_;
    LeafNode* last_leaf_;

    // Helper: find leaf containing key
    std::pair<LeafNode*, size_t> find_leaf(const K& key) const {
        if (root_type_ == RootType::Empty) {
            return {nullptr, 0};
        }

        if (root_type_ == RootType::Leaf) {
            LeafNode* leaf = root_.leaf;
            size_t pos = leaf->search_key(key, comp_);
            if (leaf->key_eq(pos, key, comp_)) {
                return {leaf, pos};
            }
            return {nullptr, 0};
        }

        // Internal node - descend
        InternalNode* node = root_.internal;
        while (!node->children_are_leaves) {
            size_t idx = 0;
            while (idx < node->len && !comp_(key, node->keys[idx])) {
                ++idx;
            }
            node = node->internal_child_at(idx);
        }

        // Now at internal node with leaf children
        size_t idx = 0;
        while (idx < node->len && !comp_(key, node->keys[idx])) {
            ++idx;
        }
        LeafNode* leaf = node->leaf_child_at(idx);
        size_t pos = leaf->search_key(key, comp_);
        if (leaf->key_eq(pos, key, comp_)) {
            return {leaf, pos};
        }
        return {nullptr, 0};
    }

    // Insert into leaf, handling splits
    Option<V> insert_into_leaf(LeafNode* leaf, K key, V value) {
        size_t pos = leaf->search_key(key, comp_);

        if (leaf->key_eq(pos, key, comp_)) {
            // Key exists, update value
            V old = std::move(leaf->vals[pos]);
            leaf->vals[pos] = std::move(value);
            return Some(std::move(old));
        }

        // Insert new entry
        if (!leaf->is_full()) {
            leaf->insert_at(pos, std::move(key), std::move(value));
            ++size_;
            return None;
        }

        // Leaf is full - need to split
        // For simplicity, insert then split
        // First, make room by splitting
        auto [new_leaf, median] = leaf->split();

        // Determine which leaf to insert into
        if (comp_(key, median)) {
            leaf->insert_at(pos, std::move(key), std::move(value));
        } else {
            size_t new_pos = new_leaf->search_key(key, comp_);
            new_leaf->insert_at(new_pos, std::move(key), std::move(value));
        }

        // Propagate split upward
        propagate_split(leaf, std::move(median), new_leaf);
        ++size_;
        return None;
    }

    // Propagate a split upward through the tree
    void propagate_split(void* left_child, K median, void* right_child) {
        // Find parent and insert median + right_child
        // For simplicity in this implementation, we rebuild from root if needed

        if (root_type_ == RootType::Leaf) {
            // Root was a leaf, now becomes internal
            InternalNode* new_root = new InternalNode(true);
            new_root->keys.construct_at(0, std::move(median));
            new_root->edges[0] = root_.leaf;
            new_root->edges[1] = right_child;
            new_root->len = 1;
            root_.internal = new_root;
            root_type_ = RootType::Internal;
            return;
        }

        // Need to find path to the child and insert
        // This is a simplified version - a production implementation would
        // maintain parent pointers or use a stack during descent
        insert_into_internal_recursive(root_.internal, left_child, std::move(median), right_child);
    }

    // Recursively find where to insert the split result
    bool insert_into_internal_recursive(InternalNode* node, void* left_child, K median, void* right_child) {
        // Find which child contains left_child
        for (size_t i = 0; i <= node->len; ++i) {
            if (node->child_at(i) == left_child) {
                // Found it - insert median and right_child after position i
                if (!node->is_full()) {
                    node->insert_at(i, std::move(median), right_child);
                    return true;
                }

                // Node is full - need to split
                auto [new_node, up_median] = node->split();

                // Determine which node gets the new entry
                if (i <= node->len) {
                    node->insert_at(i, std::move(median), right_child);
                } else {
                    size_t new_i = i - node->len - 1;
                    new_node->insert_at(new_i, std::move(median), right_child);
                }

                // Propagate this split upward
                if (node == root_.internal) {
                    // Create new root
                    InternalNode* new_root = new InternalNode(false);
                    new_root->keys.construct_at(0, std::move(up_median));
                    new_root->edges[0] = static_cast<void*>(node);
                    new_root->edges[1] = static_cast<void*>(new_node);
                    new_root->len = 1;
                    root_.internal = new_root;
                } else {
                    // Continue propagating - recursive call to parent
                    propagate_split(node, std::move(up_median), new_node);
                }
                return true;
            }

            // Descend into child if internal
            if (!node->children_are_leaves) {
                if (insert_into_internal_recursive(node->internal_child_at(i),
                                                   left_child, std::move(median), right_child)) {
                    return true;
                }
            }
        }
        return false;
    }

public:
    // Constructors
    BTreeMap() : root_type_(RootType::Empty), size_(0), first_leaf_(nullptr), last_leaf_(nullptr) {}

    static BTreeMap make() {
        return BTreeMap();
    }

    // Move constructor
    BTreeMap(BTreeMap&& other) noexcept
        : root_(other.root_), root_type_(other.root_type_), size_(other.size_),
          comp_(std::move(other.comp_)), first_leaf_(other.first_leaf_), last_leaf_(other.last_leaf_) {
        other.root_.leaf = nullptr;
        other.root_type_ = RootType::Empty;
        other.size_ = 0;
        other.first_leaf_ = other.last_leaf_ = nullptr;
    }

    // Move assignment
    BTreeMap& operator=(BTreeMap&& other) noexcept {
        if (this != &other) {
            clear();
            root_ = other.root_;
            root_type_ = other.root_type_;
            size_ = other.size_;
            comp_ = std::move(other.comp_);
            first_leaf_ = other.first_leaf_;
            last_leaf_ = other.last_leaf_;

            other.root_.leaf = nullptr;
            other.root_type_ = RootType::Empty;
            other.size_ = 0;
            other.first_leaf_ = other.last_leaf_ = nullptr;
        }
        return *this;
    }

    // No copy
    BTreeMap(const BTreeMap&) = delete;
    BTreeMap& operator=(const BTreeMap&) = delete;

    // Destructor
    ~BTreeMap() {
        clear();
    }

    // Size
    size_t len() const { return size_; }
    bool is_empty() const { return size_ == 0; }

    // Clear
    void clear() {
        if (root_type_ == RootType::Leaf) {
            delete root_.leaf;
        } else if (root_type_ == RootType::Internal) {
            delete root_.internal;
        }
        root_.leaf = nullptr;
        root_type_ = RootType::Empty;
        size_ = 0;
        first_leaf_ = last_leaf_ = nullptr;
    }

    // Insert
    Option<V> insert(K key, V value) {
        if (root_type_ == RootType::Empty) {
            // Create first leaf
            LeafNode* leaf = new LeafNode();
            leaf->insert_at(0, std::move(key), std::move(value));
            root_.leaf = leaf;
            root_type_ = RootType::Leaf;
            first_leaf_ = last_leaf_ = leaf;
            size_ = 1;
            return None;
        }

        if (root_type_ == RootType::Leaf) {
            return insert_into_leaf(root_.leaf, std::move(key), std::move(value));
        }

        // Find the right leaf
        InternalNode* node = root_.internal;
        while (!node->children_are_leaves) {
            size_t idx = 0;
            while (idx < node->len && !comp_(key, node->keys[idx])) {
                ++idx;
            }
            node = node->internal_child_at(idx);
        }

        size_t idx = 0;
        while (idx < node->len && !comp_(key, node->keys[idx])) {
            ++idx;
        }
        LeafNode* leaf = node->leaf_child_at(idx);
        return insert_into_leaf(leaf, std::move(key), std::move(value));
    }

    // Get
    Option<V*> get(const K& key) {
        auto [leaf, pos] = find_leaf(key);
        if (leaf) {
            return Some(&leaf->vals[pos]);
        }
        return None;
    }

    Option<const V*> get(const K& key) const {
        auto [leaf, pos] = find_leaf(key);
        if (leaf) {
            return Some(static_cast<const V*>(&leaf->vals[pos]));
        }
        return None;
    }

    // Contains
    bool contains_key(const K& key) const {
        auto [leaf, pos] = find_leaf(key);
        return leaf != nullptr;
    }

    // Remove (simplified - doesn't rebalance)
    Option<V> remove(const K& key) {
        auto [leaf, pos] = find_leaf(key);
        if (!leaf) {
            return None;
        }
        V value = leaf->remove_at(pos);
        --size_;
        return Some(std::move(value));
    }

    // Iterator support
    class iterator {
    private:
        LeafNode* leaf_;
        size_t index_;

    public:
        iterator(LeafNode* leaf, size_t idx) : leaf_(leaf), index_(idx) {}

        std::pair<const K&, V&> operator*() {
            return {leaf_->keys[index_], leaf_->vals[index_]};
        }

        iterator& operator++() {
            if (!leaf_) return *this;
            ++index_;
            if (index_ >= leaf_->len) {
                leaf_ = leaf_->next;
                index_ = 0;
            }
            return *this;
        }

        bool operator!=(const iterator& other) const {
            return leaf_ != other.leaf_ || index_ != other.index_;
        }

        bool operator==(const iterator& other) const {
            return leaf_ == other.leaf_ && index_ == other.index_;
        }
    };

    class const_iterator {
    private:
        const LeafNode* leaf_;
        size_t index_;

    public:
        const_iterator(const LeafNode* leaf, size_t idx) : leaf_(leaf), index_(idx) {}

        std::pair<const K&, const V&> operator*() const {
            return {leaf_->keys[index_], leaf_->vals[index_]};
        }

        const_iterator& operator++() {
            if (!leaf_) return *this;
            ++index_;
            if (index_ >= leaf_->len) {
                leaf_ = leaf_->next;
                index_ = 0;
            }
            return *this;
        }

        bool operator!=(const const_iterator& other) const {
            return leaf_ != other.leaf_ || index_ != other.index_;
        }

        bool operator==(const const_iterator& other) const {
            return leaf_ == other.leaf_ && index_ == other.index_;
        }
    };

    iterator begin() {
        if (first_leaf_ && first_leaf_->len > 0) {
            return iterator(first_leaf_, 0);
        }
        return iterator(nullptr, 0);
    }

    iterator end() {
        return iterator(nullptr, 0);
    }

    const_iterator begin() const {
        if (first_leaf_ && first_leaf_->len > 0) {
            return const_iterator(first_leaf_, 0);
        }
        return const_iterator(nullptr, 0);
    }

    const_iterator end() const {
        return const_iterator(nullptr, 0);
    }

    // Clone
    BTreeMap clone() const {
        BTreeMap result;
        for (const auto& [key, value] : *this) {
            result.insert(key, value);
        }
        return result;
    }

    // Keys
    Vec<K> keys() const {
        Vec<K> result = Vec<K>::with_capacity(size_);
        for (const auto& [key, _] : *this) {
            result.push(key);
        }
        return result;
    }

    // Values
    Vec<V> values() const {
        Vec<V> result = Vec<V>::with_capacity(size_);
        for (const auto& [_, value] : *this) {
            result.push(value);
        }
        return result;
    }
};

// Factory function
template<typename K, typename V>
BTreeMap<K, V> btreemap() {
    return BTreeMap<K, V>::make();
}

} // namespace rusty

#endif // RUSTY_BTREEMAP_HPP
