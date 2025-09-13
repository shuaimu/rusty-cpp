// STL Lifetime Annotations for RustyCpp
// 
// This header provides lifetime annotations for C++ STL types to enable
// borrow checking without modifying the standard library headers.
//
// Usage:
//   #include <stl_lifetimes.hpp>
//   #include <vector>
//   #include <map>
//   // Your code with lifetime-checked STL types
//
// The annotations use a special comment syntax that the RustyCpp analyzer
// recognizes to apply lifetime rules to STL types.

#ifndef RUSTYCPP_STL_LIFETIMES_HPP
#define RUSTYCPP_STL_LIFETIMES_HPP

// Type-level lifetime annotations for STL containers
// These annotations tell RustyCpp how to track lifetimes through STL types

// @type_lifetime: std::vector<T> {
//   iterator: &'self
//   const_iterator: &'self
//   reference: &'self mut
//   const_reference: &'self
//   pointer: *mut
//   const_pointer: *const
//   at(size_t) -> &'self mut
//   at(size_t) const -> &'self
//   operator[](size_t) -> &'self mut
//   operator[](size_t) const -> &'self
//   front() -> &'self mut
//   front() const -> &'self
//   back() -> &'self mut
//   back() const -> &'self
//   data() -> *mut
//   data() const -> *const
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
//   push_back(T) -> owned
//   emplace_back(Args...) -> owned
//   pop_back() -> owned
// }

// @type_lifetime: std::map<K, V> {
//   iterator: &'self mut
//   const_iterator: &'self
//   at(const K&) -> &'self mut
//   at(const K&) const -> &'self
//   operator[](const K&) -> &'self mut
//   find(const K&) -> &'self mut
//   find(const K&) const -> &'self
//   insert(pair<K,V>) -> owned
//   emplace(K, V) -> owned
//   erase(const K&) -> owned
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
// }

// @type_lifetime: std::unordered_map<K, V> {
//   iterator: &'self mut
//   const_iterator: &'self
//   at(const K&) -> &'self mut
//   at(const K&) const -> &'self
//   operator[](const K&) -> &'self mut
//   find(const K&) -> &'self mut
//   find(const K&) const -> &'self
//   insert(pair<K,V>) -> owned
//   emplace(K, V) -> owned
//   erase(const K&) -> owned
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
// }

// @type_lifetime: std::pair<T1, T2> {
//   first: owned
//   second: owned
// }

// @type_lifetime: std::unique_ptr<T> {
//   get() -> *mut
//   get() const -> *const
//   operator*() -> &'self mut
//   operator*() const -> &'self
//   operator->() -> *mut
//   operator->() const -> *const
//   release() -> owned
//   reset(T*) -> owned
// }

// @type_lifetime: std::shared_ptr<T> {
//   get() -> *const
//   operator*() -> &'self
//   operator*() const -> &'self
//   operator->() -> *const
//   operator->() const -> *const
//   reset(T*) -> owned
//   use_count() const -> owned
// }

// @type_lifetime: std::string {
//   c_str() const -> *const
//   data() -> *mut
//   data() const -> *const
//   at(size_t) -> &'self mut
//   at(size_t) const -> &'self
//   operator[](size_t) -> &'self mut
//   operator[](size_t) const -> &'self
//   front() -> &'self mut
//   front() const -> &'self
//   back() -> &'self mut
//   back() const -> &'self
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
//   append(const string&) -> owned
//   push_back(char) -> owned
//   pop_back() -> owned
// }

// @type_lifetime: std::optional<T> {
//   value() -> &'self mut
//   value() const -> &'self
//   operator*() -> &'self mut
//   operator*() const -> &'self
//   operator->() -> *mut
//   operator->() const -> *const
//   value_or(T) const -> owned
//   has_value() const -> owned
// }

// @type_lifetime: std::array<T, N> {
//   at(size_t) -> &'self mut
//   at(size_t) const -> &'self
//   operator[](size_t) -> &'self mut
//   operator[](size_t) const -> &'self
//   front() -> &'self mut
//   front() const -> &'self
//   back() -> &'self mut
//   back() const -> &'self
//   data() -> *mut
//   data() const -> *const
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
// }

// @type_lifetime: std::deque<T> {
//   at(size_t) -> &'self mut
//   at(size_t) const -> &'self
//   operator[](size_t) -> &'self mut
//   operator[](size_t) const -> &'self
//   front() -> &'self mut
//   front() const -> &'self
//   back() -> &'self mut
//   back() const -> &'self
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
//   push_back(T) -> owned
//   push_front(T) -> owned
//   pop_back() -> owned
//   pop_front() -> owned
// }

// @type_lifetime: std::set<T> {
//   iterator: &'self
//   const_iterator: &'self
//   find(const T&) -> &'self
//   find(const T&) const -> &'self
//   insert(T) -> owned
//   emplace(T) -> owned
//   erase(const T&) -> owned
//   begin() -> &'self
//   begin() const -> &'self
//   end() -> &'self
//   end() const -> &'self
// }

// @type_lifetime: std::unordered_set<T> {
//   iterator: &'self
//   const_iterator: &'self
//   find(const T&) -> &'self
//   find(const T&) const -> &'self
//   insert(T) -> owned
//   emplace(T) -> owned
//   erase(const T&) -> owned
//   begin() -> &'self
//   begin() const -> &'self
//   end() -> &'self
//   end() const -> &'self
// }

// @type_lifetime: std::list<T> {
//   front() -> &'self mut
//   front() const -> &'self
//   back() -> &'self mut
//   back() const -> &'self
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
//   push_back(T) -> owned
//   push_front(T) -> owned
//   pop_back() -> owned
//   pop_front() -> owned
// }

// @type_lifetime: std::forward_list<T> {
//   front() -> &'self mut
//   front() const -> &'self
//   begin() -> &'self mut
//   begin() const -> &'self
//   end() -> &'self mut
//   end() const -> &'self
//   push_front(T) -> owned
//   pop_front() -> owned
// }

// @type_lifetime: std::stack<T> {
//   top() -> &'self mut
//   top() const -> &'self
//   push(T) -> owned
//   pop() -> owned
// }

// @type_lifetime: std::queue<T> {
//   front() -> &'self mut
//   front() const -> &'self
//   back() -> &'self mut
//   back() const -> &'self
//   push(T) -> owned
//   pop() -> owned
// }

// @type_lifetime: std::priority_queue<T> {
//   top() const -> &'self
//   push(T) -> owned
//   pop() -> owned
// }

// Iterator lifetime annotations
// @type_lifetime: std::vector<T>::iterator {
//   operator*() -> &'self mut
//   operator->() -> *mut
// }

// @type_lifetime: std::vector<T>::const_iterator {
//   operator*() -> &'self
//   operator->() -> *const
// }

// Algorithm lifetime annotations for common STL algorithms
namespace std {
    // @lifetime: (&'a, &'a) -> &'a
    // find returns iterator with same lifetime as range
    
    // @lifetime: (&'a mut, &'a mut, 'b) -> &'a mut
    // copy modifies destination, returns iterator
    
    // @lifetime: (&'a mut, &'a mut) -> owned
    // sort modifies in place
    
    // @lifetime: (&'a, &'a, 'b) -> owned
    // for_each takes ownership of functor
}

// Helper macros for marking STL usage as safe/unsafe in user code
#define STL_SAFE_REGION // @safe
#define STL_UNSAFE_REGION // @unsafe

// Example usage patterns that should be checked:
//
// @safe
// void example() {
//     std::vector<int> vec = {1, 2, 3};
//     int& ref = vec[0];  // &'vec mut
//     vec.push_back(4);   // ERROR: cannot modify vec while ref exists
// }
//
// @safe  
// void iterator_invalidation() {
//     std::vector<int> vec = {1, 2, 3};
//     auto it = vec.begin();  // &'vec mut
//     vec.push_back(4);       // ERROR: invalidates iterator
//     *it = 5;                // Would be use-after-invalidation
// }
//
// @safe
// void map_example() {
//     std::map<int, std::string> m;
//     m[1] = "one";
//     std::string& s = m[1];  // &'m mut
//     m.erase(1);             // ERROR: cannot modify while reference exists
// }

#endif // RUSTYCPP_STL_LIFETIMES_HPP