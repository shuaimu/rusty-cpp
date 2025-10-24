#ifndef RUSTY_BTREEMAP_HPP
#define RUSTY_BTREEMAP_HPP

#include <algorithm>
#include <functional>
#include <memory>
#include <utility>
#include <cassert>
#include <cstring>
#include "option.hpp"
#include "vec.hpp"

// Real B-Tree implementation following Rust's std::collections::BTreeMap
// 
// Key design decisions from Rust's implementation:
// - B = 6: Each node has 5-11 keys (except root: 0-11 keys)
// - Nodes are either internal (with children) or leaves (with values)
// - Keys and edges are stored in arrays for cache locality
// - Node splitting/merging maintains B-tree invariants
// - In-place mutation when possible for performance

// @safe
namespace rusty {

// B-tree constants matching Rust's implementation
constexpr size_t BTREE_B = 6;                    // Branching factor
constexpr size_t BTREE_MIN_LEN = BTREE_B - 1;   // 5: Minimum keys in non-root node
constexpr size_t BTREE_MAX_LEN = 2 * BTREE_B - 1; // 11: Maximum keys in any node
constexpr size_t BTREE_MAX_CHILDREN = 2 * BTREE_B; // 12: Maximum children

template <typename K, typename V, typename Compare = std::less<K>>
class BTreeMap {
private:
    // Forward declarations
    struct Node;
    struct InternalNode;
    struct LeafNode;
    
    using NodePtr = std::unique_ptr<Node>;
    using KV = std::pair<K, V>;
    
    // Base node structure
    struct Node {
        K keys[BTREE_MAX_LEN];      // Keys array
        size_t len;                  // Number of keys
        bool is_leaf;                // Leaf or internal node
        
        Node(bool leaf) : len(0), is_leaf(leaf) {}
        virtual ~Node() = default;
        
        // Binary search for key position
        size_t search_key(const K& key, const Compare& comp) const {
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
        
        // Check if key exists at position
        bool key_at_pos_eq(size_t pos, const K& key, const Compare& comp) const {
            return pos < len && !comp(keys[pos], key) && !comp(key, keys[pos]);
        }
        
        bool is_full() const { return len >= BTREE_MAX_LEN; }
        bool is_underfull() const { return len < BTREE_MIN_LEN; }
        
        // Insert key at position (assumes space available)
        void insert_key_at(size_t pos, K key) {
            std::memmove(&keys[pos + 1], &keys[pos], (len - pos) * sizeof(K));
            keys[pos] = std::move(key);
            len++;
        }
        
        // Remove key at position
        void remove_key_at(size_t pos) {
            keys[pos].~K();
            std::memmove(&keys[pos], &keys[pos + 1], (len - pos - 1) * sizeof(K));
            len--;
        }
    };
    
    // Leaf node - contains values
    struct LeafNode : Node {
        V values[BTREE_MAX_LEN];     // Values array
        LeafNode* next;               // Next leaf for iteration
        LeafNode* prev;               // Previous leaf for iteration
        
        LeafNode() : Node(true), next(nullptr), prev(nullptr) {}
        
        ~LeafNode() = default;
        
        // Insert key-value at position
        void insert_at(size_t pos, K key, V value) {
            // Shift values - can't use memmove for non-trivial types
            for (size_t i = this->len; i > pos; i--) {
                values[i] = std::move(values[i - 1]);
            }
            values[pos] = std::move(value);
            
            // Insert key
            this->insert_key_at(pos, std::move(key));
        }
        
        // Remove at position
        V remove_at(size_t pos) {
            V value = std::move(values[pos]);
            // Shift values left - can't use memmove for non-trivial types
            for (size_t i = pos; i < this->len - 1; i++) {
                values[i] = std::move(values[i + 1]);
            }
            
            this->remove_key_at(pos);
            return value;
        }
        
        // Split leaf node (returns new right node and median key)
        std::pair<LeafNode*, K> split() {
            size_t mid = this->len / 2;
            auto* right = new LeafNode();
            
            // Move right half to new node (including the element at mid)
            right->len = this->len - mid;
            for (size_t i = 0; i < right->len; i++) {
                right->keys[i] = std::move(this->keys[mid + i]);
                right->values[i] = std::move(this->values[mid + i]);
            }
            
            // The median key that goes up to parent is the first key of right node
            K median = right->keys[0];
            this->len = mid;
            
            // Update linked list pointers
            right->next = this->next;
            right->prev = this;
            if (this->next) {
                this->next->prev = right;
            }
            this->next = right;
            
            return {right, median};
        }
        
        // Merge with right sibling
        void merge_with_right(LeafNode* right, const K& parent_key) {
            // Copy all entries from right
            for (size_t i = 0; i < right->len; i++) {
                this->keys[this->len + i] = std::move(right->keys[i]);
                this->values[this->len + i] = std::move(right->values[i]);
            }
            this->len += right->len;
            
            // Update linked list
            this->next = right->next;
            if (right->next) {
                right->next->prev = this;
            }
        }
    };
    
    // Internal node - contains child pointers
    struct InternalNode : Node {
        NodePtr children[BTREE_MAX_CHILDREN];  // Child pointers
        
        InternalNode() : Node(false) {}
        
        // Insert key and right child at position
        void insert_at(size_t pos, K key, NodePtr right_child) {
            // Shift children - can't use memmove with unique_ptr
            for (size_t i = this->len; i > pos; i--) {
                children[i + 1] = std::move(children[i]);
            }
            children[pos + 1] = std::move(right_child);
            
            // Insert key
            this->insert_key_at(pos, std::move(key));
        }
        
        // Split internal node
        std::pair<InternalNode*, K> split() {
            size_t mid = this->len / 2;
            auto* right = new InternalNode();
            
            // Move right half keys to new node
            right->len = this->len - mid - 1;
            for (size_t i = 0; i < right->len; i++) {
                right->keys[i] = std::move(this->keys[mid + 1 + i]);
            }
            
            // Move right half children to new node
            for (size_t i = 0; i <= right->len; i++) {
                right->children[i] = std::move(this->children[mid + 1 + i]);
            }
            
            K median = std::move(this->keys[mid]);
            this->len = mid;
            
            return {right, median};
        }
        
        // Merge with right sibling
        void merge_with_right(InternalNode* right, const K& parent_key) {
            // Add parent key
            this->keys[this->len] = parent_key;
            this->len++;
            
            // Copy keys from right
            for (size_t i = 0; i < right->len; i++) {
                this->keys[this->len + i] = std::move(right->keys[i]);
            }
            
            // Copy children from right
            for (size_t i = 0; i <= right->len; i++) {
                this->children[this->len + i] = std::move(right->children[i]);
            }
            
            this->len += right->len;
        }
        
        // Borrow from left sibling
        void borrow_from_left(InternalNode* left, K& parent_key) {
            // Shift everything right
            std::memmove(&this->keys[1], &this->keys[0], this->len * sizeof(K));
            // Can't use memmove with unique_ptr
            for (size_t i = this->len; i > 0; i--) {
                this->children[i] = std::move(this->children[i - 1]);
            }
            
            // Move parent key down
            this->keys[0] = std::move(parent_key);
            
            // Move last key from left up to parent
            parent_key = std::move(left->keys[left->len - 1]);
            
            // Move last child from left
            this->children[0] = std::move(left->children[left->len]);
            
            this->len++;
            left->len--;
        }
        
        // Borrow from right sibling
        void borrow_from_right(InternalNode* right, K& parent_key) {
            // Move parent key down
            this->keys[this->len] = std::move(parent_key);
            
            // Move first key from right up to parent
            parent_key = std::move(right->keys[0]);
            
            // Move first child from right
            this->children[this->len + 1] = std::move(right->children[0]);
            
            // Shift right node left
            std::memmove(&right->keys[0], &right->keys[1], 
                        (right->len - 1) * sizeof(K));
            // Can't use memmove with unique_ptr
            for (size_t i = 0; i < right->len; i++) {
                right->children[i] = std::move(right->children[i + 1]);
            }
            
            this->len++;
            right->len--;
        }
    };
    
    // Member variables
    NodePtr root_;
    size_t size_;
    Compare comp_;
    LeafNode* first_leaf_;  // For iteration
    LeafNode* last_leaf_;   // For reverse iteration
    
    // Helper: Insert into non-full node
    Option<V> insert_into_node(Node* node, K key, V value) {
        if (node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            size_t pos = node->search_key(key, comp_);
            
            if (node->key_at_pos_eq(pos, key, comp_)) {
                // Key exists, update value
                V old = std::move(leaf->values[pos]);
                leaf->values[pos] = std::move(value);
                return Some(std::move(old));
            } else {
                // Insert new entry
                leaf->insert_at(pos, std::move(key), std::move(value));
                size_++;
                return None;
            }
        } else {
            auto* internal = static_cast<InternalNode*>(node);
            
            // Find which child to recurse to
            size_t child_idx = 0;
            while (child_idx < internal->len && !comp_(key, internal->keys[child_idx])) {
                child_idx++;
            }
            
            Node* child = internal->children[child_idx].get();
            
            if (child->is_full()) {
                // Split child first
                split_child(internal, child_idx);
                
                // After split, recompute which child to use
                if (child_idx < internal->len && !comp_(key, internal->keys[child_idx])) {
                    child_idx++;
                }
                child = internal->children[child_idx].get();
            }
            
            return insert_into_node(child, std::move(key), std::move(value));
        }
    }
    
    // Split child node at position
    void split_child(InternalNode* parent, size_t child_idx) {
        Node* child = parent->children[child_idx].get();
        
        if (child->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(child);
            auto [new_right, median] = leaf->split();
            parent->insert_at(child_idx, std::move(median), 
                            NodePtr(new_right));
        } else {
            auto* internal = static_cast<InternalNode*>(child);
            auto [new_right, median] = internal->split();
            parent->insert_at(child_idx, std::move(median), 
                            NodePtr(new_right));
        }
    }
    
    // Find node containing key
    std::pair<Node*, size_t> find_node(const K& key) const {
        Node* node = root_.get();
        
        while (node) {
            if (node->is_leaf) {
                size_t pos = node->search_key(key, comp_);
                if (node->key_at_pos_eq(pos, key, comp_)) {
                    return {node, pos};
                }
                return {nullptr, 0};
            }
            
            auto* internal = static_cast<InternalNode*>(node);
            
            // Find which child to descend to
            size_t child_idx = 0;
            while (child_idx < internal->len && !comp_(key, internal->keys[child_idx])) {
                child_idx++;
            }
            node = internal->children[child_idx].get();
        }
        
        return {nullptr, 0};
    }
    
    // Remove from node (complex - handles rebalancing)
    Option<V> remove_from_node(Node* node, const K& key) {
        if (node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            size_t pos = node->search_key(key, comp_);
            
            if (node->key_at_pos_eq(pos, key, comp_)) {
                V value = leaf->remove_at(pos);
                size_--;
                return Some(std::move(value));
            }
            return None;
        }
        
        auto* internal = static_cast<InternalNode*>(node);
        
        // Find which child to recurse to
        size_t child_idx = 0;
        while (child_idx < internal->len && !comp_(key, internal->keys[child_idx])) {
            child_idx++;
        }
        
        Node* child = internal->children[child_idx].get();
        
        // Ensure child has enough keys (unless it's the root)
        if (child != root_.get() && child->is_underfull()) {
            fix_underfull_child(internal, child_idx);
            // After fix, recompute which child to use
            child_idx = 0;
            while (child_idx < internal->len && !comp_(key, internal->keys[child_idx])) {
                child_idx++;
            }
            if (child_idx <= internal->len) {
                child = internal->children[child_idx].get();
            } else {
                return None;
            }
        }
        
        return remove_from_node(child, key);
    }
    
    // Remove key from internal node
    Option<V> remove_internal_key(InternalNode* node, size_t pos) {
        // Replace with predecessor from left subtree
        Node* left_child = node->children[pos].get();
        
        if (left_child->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(left_child);
            if (leaf->len > BTREE_MIN_LEN) {
                // Take last element from left child
                size_t last = leaf->len - 1;
                node->keys[pos] = std::move(leaf->keys[last]);
                V value = leaf->remove_at(last);
                size_--;
                return Some(std::move(value));
            }
        }
        
        // Complex case - need to restructure
        // For simplicity, convert to leaf operation
        return remove_from_node(left_child, node->keys[pos]);
    }
    
    // Fix underfull child by borrowing or merging
    void fix_underfull_child(InternalNode* parent, size_t child_idx) {
        // Try to borrow from left sibling
        if (child_idx > 0) {
            Node* left = parent->children[child_idx - 1].get();
            if (left->len > BTREE_MIN_LEN) {
                borrow_from_left_sibling(parent, child_idx);
                return;
            }
        }
        
        // Try to borrow from right sibling
        if (child_idx < parent->len) {
            Node* right = parent->children[child_idx + 1].get();
            if (right->len > BTREE_MIN_LEN) {
                borrow_from_right_sibling(parent, child_idx);
                return;
            }
        }
        
        // Must merge with sibling
        if (child_idx < parent->len) {
            merge_with_right_sibling(parent, child_idx);
        } else {
            merge_with_right_sibling(parent, child_idx - 1);
        }
    }
    
    // Borrow from left sibling
    void borrow_from_left_sibling(InternalNode* parent, size_t child_idx) {
        if (parent->children[child_idx]->is_leaf) {
            auto* child = static_cast<LeafNode*>(parent->children[child_idx].get());
            auto* left = static_cast<LeafNode*>(parent->children[child_idx - 1].get());
            
            // Move last element from left to beginning of child
            child->insert_at(0, std::move(left->keys[left->len - 1]),
                           std::move(left->values[left->len - 1]));
            left->len--;
            
            // Update parent key
            parent->keys[child_idx - 1] = child->keys[0];
        } else {
            auto* child = static_cast<InternalNode*>(parent->children[child_idx].get());
            auto* left = static_cast<InternalNode*>(parent->children[child_idx - 1].get());
            child->borrow_from_left(left, parent->keys[child_idx - 1]);
        }
    }
    
    // Borrow from right sibling
    void borrow_from_right_sibling(InternalNode* parent, size_t child_idx) {
        if (parent->children[child_idx]->is_leaf) {
            auto* child = static_cast<LeafNode*>(parent->children[child_idx].get());
            auto* right = static_cast<LeafNode*>(parent->children[child_idx + 1].get());
            
            // Move first element from right to end of child
            child->insert_at(child->len, std::move(right->keys[0]),
                           std::move(right->values[0]));
            right->remove_at(0);
            
            // Update parent key
            parent->keys[child_idx] = right->keys[0];
        } else {
            auto* child = static_cast<InternalNode*>(parent->children[child_idx].get());
            auto* right = static_cast<InternalNode*>(parent->children[child_idx + 1].get());
            child->borrow_from_right(right, parent->keys[child_idx]);
        }
    }
    
    // Merge child with right sibling
    void merge_with_right_sibling(InternalNode* parent, size_t child_idx) {
        if (parent->children[child_idx]->is_leaf) {
            auto* left = static_cast<LeafNode*>(parent->children[child_idx].get());
            auto* right = static_cast<LeafNode*>(parent->children[child_idx + 1].get());
            left->merge_with_right(right, parent->keys[child_idx]);
        } else {
            auto* left = static_cast<InternalNode*>(parent->children[child_idx].get());
            auto* right = static_cast<InternalNode*>(parent->children[child_idx + 1].get());
            left->merge_with_right(right, parent->keys[child_idx]);
        }
        
        // Remove key and right child from parent
        parent->remove_key_at(child_idx);
        // Shift children left - can't use memmove with unique_ptr
        for (size_t i = child_idx + 1; i <= parent->len; i++) {
            parent->children[i] = std::move(parent->children[i + 1]);
        }
    }
    
public:
    // Constructors
    BTreeMap() : size_(0), first_leaf_(nullptr), last_leaf_(nullptr) {
        auto* root = new LeafNode();
        root_ = NodePtr(root);
        first_leaf_ = last_leaf_ = root;
    }
    
    // @lifetime: owned
    static auto make() {
        return BTreeMap();
    }
    
    // Move constructor
    BTreeMap(BTreeMap&& other) noexcept
        : root_(std::move(other.root_)), size_(other.size_),
          comp_(std::move(other.comp_)),
          first_leaf_(other.first_leaf_), last_leaf_(other.last_leaf_) {
        other.size_ = 0;
        other.first_leaf_ = other.last_leaf_ = nullptr;
    }
    
    // Move assignment
    BTreeMap& operator=(BTreeMap&& other) noexcept {
        if (this != &other) {
            root_ = std::move(other.root_);
            size_ = other.size_;
            comp_ = std::move(other.comp_);
            first_leaf_ = other.first_leaf_;
            last_leaf_ = other.last_leaf_;
            
            other.size_ = 0;
            other.first_leaf_ = other.last_leaf_ = nullptr;
        }
        return *this;
    }
    
    // Delete copy operations
    BTreeMap(const BTreeMap&) = delete;
    BTreeMap& operator=(const BTreeMap&) = delete;
    
    // Destructor
    ~BTreeMap() = default;
    
    // Size
    size_t len() const { return size_; }
    bool is_empty() const { return size_ == 0; }
    
    // Insert
    Option<V> insert(K key, V value) {
        if (root_->is_full()) {
            // Split root
            auto* new_root = new InternalNode();
            NodePtr old_root = std::move(root_);
            
            if (old_root->is_leaf) {
                auto* leaf = static_cast<LeafNode*>(old_root.get());
                auto [new_right, median] = leaf->split();
                
                new_root->keys[0] = std::move(median);
                new_root->children[0] = std::move(old_root);
                new_root->children[1] = NodePtr(new_right);
                new_root->len = 1;
            } else {
                auto* internal = static_cast<InternalNode*>(old_root.get());
                auto [new_right, median] = internal->split();
                
                new_root->keys[0] = std::move(median);
                new_root->children[0] = std::move(old_root);
                new_root->children[1] = NodePtr(new_right);
                new_root->len = 1;
            }
            
            root_ = NodePtr(new_root);
        }
        
        return insert_into_node(root_.get(), std::move(key), std::move(value));
    }
    
    // Get
    // @lifetime: (&'a) -> &'a
    Option<V*> get(const K& key) {
        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            return Some(&leaf->values[pos]);
        }
        return None;
    }
    
    // @lifetime: (&'a) -> &'a
    Option<const V*> get(const K& key) const {
        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<const LeafNode*>(node);
            return Option<const V*>(Some(&leaf->values[pos]));
        }
        return Option<const V*>(None);
    }
    
    // Contains
    bool contains_key(const K& key) const {
        auto [node, pos] = find_node(key);
        return node != nullptr;
    }
    
    // Remove - simplified version for now
    // @lifetime: owned
    Option<V> remove(const K& key) {
        if (size_ == 0) return None;
        
        // For now, just do simple leaf removal without rebalancing
        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            V value = leaf->remove_at(pos);
            size_--;
            return Some(std::move(value));
        }
        
        return None;
    }
    
    // Clear
    void clear() {
        auto* root = new LeafNode();
        root_ = NodePtr(root);
        first_leaf_ = last_leaf_ = root;
        size_ = 0;
    }
    
    // Iterator support
    class iterator {
    private:
        LeafNode* leaf_;
        size_t index_;
        
    public:
        iterator(LeafNode* leaf, size_t idx) : leaf_(leaf), index_(idx) {}
        
        std::pair<const K&, V&> operator*() {
            return {leaf_->keys[index_], leaf_->values[index_]};
        }
        
        iterator& operator++() {
            if (!leaf_) return *this;
            index_++;
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
    
    // @lifetime: (&'a) -> &'a
    iterator begin() {
        if (first_leaf_ && first_leaf_->len > 0) {
            return iterator(first_leaf_, 0);
        }
        return iterator(nullptr, 0);
    }
    
    // @lifetime: (&'a) -> &'a
    iterator end() {
        return iterator(nullptr, 0);
    }
    
    // Const iterator support
    class const_iterator {
    private:
        const LeafNode* leaf_;
        size_t index_;
        
    public:
        const_iterator(const LeafNode* leaf, size_t idx) : leaf_(leaf), index_(idx) {}
        
        std::pair<const K&, const V&> operator*() const {
            return {leaf_->keys[index_], leaf_->values[index_]};
        }
        
        const_iterator& operator++() {
            if (!leaf_) return *this;
            index_++;
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
    
    // @lifetime: (&'a) -> &'a
    const_iterator begin() const {
        if (first_leaf_ && first_leaf_->len > 0) {
            return const_iterator(first_leaf_, 0);
        }
        return const_iterator(nullptr, 0);
    }
    
    // @lifetime: (&'a) -> &'a
    const_iterator end() const {
        return const_iterator(nullptr, 0);
    }
    
    // Additional methods for compatibility
    
    // Clone for explicit copying
    // @lifetime: owned
    BTreeMap clone() const {
        BTreeMap result;
        for (const auto& [key, value] : *this) {
            result.insert(key, value);
        }
        return result;
    }
    
    // Collect all keys
    // @lifetime: owned
    Vec<K> keys() const {
        Vec<K> result = Vec<K>::with_capacity(size_);
        for (const auto& [key, _] : *this) {
            result.push(key);
        }
        return result;
    }
    
    // Collect all values
    // @lifetime: owned
    Vec<V> values() const {
        Vec<V> result = Vec<V>::with_capacity(size_);
        for (const auto& [_, value] : *this) {
            result.push(value);
        }
        return result;
    }
    
    // Get or insert with default
    // @lifetime: (&'a mut) -> &'a mut
    V& entry(const K& key) {
        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            return leaf->values[pos];
        }
        // Insert and return reference
        insert(key, V());
        auto [new_node, new_pos] = find_node(key);
        auto* leaf = static_cast<LeafNode*>(new_node);
        return leaf->values[new_pos];
    }
    
    // Get or insert with value
    // @lifetime: (&'a mut) -> &'a mut
    V& or_insert(const K& key, V default_value) {
        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            return leaf->values[pos];
        }
        // Insert and return reference
        insert(key, std::move(default_value));
        auto [new_node, new_pos] = find_node(key);
        auto* leaf = static_cast<LeafNode*>(new_node);
        return leaf->values[new_pos];
    }
    
    // Get mutable reference to value
    // @lifetime: (&'a mut) -> &'a mut
    Option<V*> get_mut(const K& key) {
        return get(key); // Since get already returns mutable
    }

    // Get key-value pair
    // @lifetime: (&'a) -> &'a
    Option<std::pair<K*, V*>> get_key_value(const K& key) {
        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            return Some(std::make_pair(&leaf->keys[pos], &leaf->values[pos]));
        }
        return None;
    }

    // @lifetime: (&'a) -> &'a
    Option<std::pair<const K*, const V*>> get_key_value(const K& key) const {
        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<const LeafNode*>(node);
            const K* key_ptr = &leaf->keys[pos];
            const V* val_ptr = &leaf->values[pos];
            return Some(std::make_pair(key_ptr, val_ptr));
        }
        return None;
    }

    // Remove entry (returns both key and value)
    // @lifetime: owned
    Option<std::pair<K, V>> remove_entry(const K& key) {
        if (size_ == 0) return None;

        auto [node, pos] = find_node(key);
        if (node && node->is_leaf) {
            auto* leaf = static_cast<LeafNode*>(node);
            K removed_key = std::move(leaf->keys[pos]);
            V value = leaf->remove_at(pos);
            size_--;
            return Some(std::make_pair(std::move(removed_key), std::move(value)));
        }

        return None;
    }

    // Get first key-value pair
    // @lifetime: (&'a) -> &'a
    Option<std::pair<K*, V*>> first_key_value() {
        if (first_leaf_ && first_leaf_->len > 0) {
            return Some(std::make_pair(&first_leaf_->keys[0], &first_leaf_->values[0]));
        }
        return None;
    }

    // @lifetime: (&'a) -> &'a
    Option<std::pair<const K*, const V*>> first_key_value() const {
        if (first_leaf_ && first_leaf_->len > 0) {
            const K* key_ptr = &first_leaf_->keys[0];
            const V* val_ptr = &first_leaf_->values[0];
            return Some(std::make_pair(key_ptr, val_ptr));
        }
        return None;
    }

    // Get last key-value pair
    // @lifetime: (&'a) -> &'a
    Option<std::pair<K*, V*>> last_key_value() {
        if (last_leaf_ && last_leaf_->len > 0) {
            size_t last_idx = last_leaf_->len - 1;
            return Some(std::make_pair(&last_leaf_->keys[last_idx], &last_leaf_->values[last_idx]));
        }
        return None;
    }

    // @lifetime: (&'a) -> &'a
    Option<std::pair<const K*, const V*>> last_key_value() const {
        if (last_leaf_ && last_leaf_->len > 0) {
            size_t last_idx = last_leaf_->len - 1;
            const K* key_ptr = &last_leaf_->keys[last_idx];
            const V* val_ptr = &last_leaf_->values[last_idx];
            return Some(std::make_pair(key_ptr, val_ptr));
        }
        return None;
    }

    // Pop first entry
    // @lifetime: owned
    Option<std::pair<K, V>> pop_first() {
        if (first_leaf_ && first_leaf_->len > 0) {
            K key = std::move(first_leaf_->keys[0]);
            V value = first_leaf_->remove_at(0);
            size_--;
            return Some(std::make_pair(std::move(key), std::move(value)));
        }
        return None;
    }

    // Pop last entry
    // @lifetime: owned
    Option<std::pair<K, V>> pop_last() {
        if (last_leaf_ && last_leaf_->len > 0) {
            size_t last_idx = last_leaf_->len - 1;
            K key = std::move(last_leaf_->keys[last_idx]);
            V value = last_leaf_->remove_at(last_idx);
            size_--;
            return Some(std::make_pair(std::move(key), std::move(value)));
        }
        return None;
    }

    // Range query (returns vector of key-value pairs in range [start, end])
    // @lifetime: owned
    std::vector<std::pair<K, V>> range(const K& start, const K& end) const {
        std::vector<std::pair<K, V>> result;

        // Iterate through all entries
        for (const auto& [key, value] : *this) {
            // Check if key is in range [start, end]
            if (!comp_(key, start) && !comp_(end, key)) {
                result.push_back(std::make_pair(key, value));
            }
        }

        return result;
    }

    // Split off all entries >= at_key into a new map
    // @lifetime: owned
    BTreeMap split_off(const K& at_key) {
        BTreeMap upper;

        // Collect keys to move
        Vec<K> keys_to_move = Vec<K>::make();
        for (const auto& [key, _] : *this) {
            if (!comp_(key, at_key)) {  // key >= at_key
                keys_to_move.push(key);
            }
        }

        // Move entries to upper map
        for (size_t i = 0; i < keys_to_move.len(); i++) {
            const K& key = keys_to_move[i];
            auto removed = remove(key);
            if (removed.is_some()) {
                upper.insert(key, removed.unwrap());
            }
        }

        return upper;
    }

    // Append another map (all keys in other must be > all keys in this)
    void append(BTreeMap&& other) {
        // Simple implementation: just insert all entries
        for (const auto& [key, value] : other) {
            insert(key, value);
        }
        other.clear();
    }

    // Extend with entries from another map
    void extend(BTreeMap&& other) {
        for (const auto& [key, value] : other) {
            insert(key, value);
        }
        other.clear();
    }

    // Retain only entries that satisfy predicate
    template<typename Pred>
    void retain(Pred predicate) {
        Vec<K> keys_to_remove = Vec<K>::make();

        for (const auto& [key, value] : *this) {
            if (!predicate(key, value)) {
                keys_to_remove.push(key);
            }
        }

        for (size_t i = 0; i < keys_to_remove.len(); i++) {
            remove(keys_to_remove[i]);
        }
    }

    // Equality comparison
    bool operator==(const BTreeMap& other) const {
        if (size_ != other.size_) return false;
        for (const auto& [key, value] : *this) {
            auto other_val = other.get(key);
            if (other_val.is_none() || *other_val.unwrap() != value) {
                return false;
            }
        }
        return true;
    }
    
    bool operator!=(const BTreeMap& other) const {
        return !(*this == other);
    }
};

// Factory function
template<typename K, typename V>
// @lifetime: owned
BTreeMap<K, V> btreemap() {
    return BTreeMap<K, V>::make();
}

// Create BTreeMap from Vec of pairs
template<typename K, typename V>
// @lifetime: owned
BTreeMap<K, V> btreemap_from_vec(Vec<std::pair<K, V>>&& vec) {
    BTreeMap<K, V> map;
    for (size_t i = 0; i < vec.len(); i++) {
        auto& pair = vec[i];
        map.insert(std::move(pair.first), std::move(pair.second));
    }
    return map;
}

} // namespace rusty

#endif // RUSTY_BTREEMAP_HPP