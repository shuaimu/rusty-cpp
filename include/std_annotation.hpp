// Standard Library Safety Annotations for RustyCpp
//
// This header provides pre-declared safety annotations for common C++ standard
// library functions, so users don't need to manually annotate every std usage.
//
// Usage:
//   #include <std_annotation.hpp>
//
//   // Now you can use std functions in @safe code without extra annotations:
//   // @safe
//   void my_function() {
//       std::vector<int> vec = {1, 2, 3};
//       std::sort(vec.begin(), vec.end());  // Works without extra annotation!
//       std::cout << "Hello" << std::endl;   // Works!
//   }
//
// This file marks commonly-used std functions as safe, allowing them to be
// called from @safe code without wrapping in @unsafe blocks.

#ifndef RUSTYCPP_STD_ANNOTATION_HPP
#define RUSTYCPP_STD_ANNOTATION_HPP

// ============================================================================
// Utility Functions - Core C++
// ============================================================================

// @external: {
//   std::move: [safe, (T& x) -> T&& where x: 'a, return: 'a]
//   std::forward: [safe, (T& x) -> T&& where x: 'a, return: 'a]
//   std::swap: [safe, (T& a, T& b) -> void]
//   std::exchange: [safe, (T& obj, U&& new_val) -> T]
// }

// ============================================================================
// Smart Pointers - Memory Management
// ============================================================================

// std::unique_ptr operations
// @external: {
//   std::make_unique: [safe, (Args&&... args) -> owned std::unique_ptr<T>]
//   std::unique_ptr::get: [safe, () -> T* where this: 'a, return: 'a]
//   std::unique_ptr::release: [safe, () -> owned T*]
//   std::unique_ptr::reset: [safe, (T* ptr) -> void]
//   std::unique_ptr::operator*: [safe, () -> T& where this: 'a, return: 'a]
//   std::unique_ptr::operator->: [safe, () -> T* where this: 'a, return: 'a]
//   std::unique_ptr::operator bool: [safe, () const -> bool]
// }

// std::shared_ptr operations
// @external: {
//   std::make_shared: [safe, (Args&&... args) -> owned std::shared_ptr<T>]
//   std::shared_ptr::get: [safe, () const -> T* where this: 'a, return: 'a]
//   std::shared_ptr::reset: [safe, (T* ptr) -> void]
//   std::shared_ptr::operator*: [safe, () const -> const T& where this: 'a, return: 'a]
//   std::shared_ptr::operator->: [safe, () const -> const T* where this: 'a, return: 'a]
//   std::shared_ptr::operator bool: [safe, () const -> bool]
//   std::shared_ptr::use_count: [safe, () const -> long]
// }

// std::weak_ptr operations
// @external: {
//   std::weak_ptr::lock: [safe, () const -> std::shared_ptr<T>]
//   std::weak_ptr::expired: [safe, () const -> bool]
//   std::weak_ptr::use_count: [safe, () const -> long]
// }

// ============================================================================
// UNSAFE Operations (Require @unsafe functions)
// ============================================================================

// NOTE: The following operations are NOT safe and must be used in @unsafe functions:
//
// Smart pointer casts - can break type safety:
// @external: {
//   std::dynamic_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
//   std::static_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
//   std::const_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
//   std::reinterpret_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
// }
//
// C++ cast operators - operate on raw pointers:
// @external: {
//   dynamic_cast: [unsafe, (T* ptr) -> U*]
//   static_cast: [unsafe, (T value) -> U]
//   const_cast: [unsafe, (T value) -> U]
//   reinterpret_cast: [unsafe, (T value) -> U]
// }
//
// Raw pointer operations:
// @external: {
//   std::unique_ptr::get: [unsafe, () -> T* where this: 'a, return: 'a]
//   std::shared_ptr::get: [unsafe, () const -> T* where this: 'a, return: 'a]
//   std::unique_ptr::release: [unsafe, () -> owned T*]
//   std::addressof: [unsafe, (T& value) -> T* where value: 'a, return: 'a]
//   std::launder: [unsafe, (T* ptr) -> T* where ptr: 'a, return: 'a]
// }
//
// Type reinterpretation:
// @external: {
//   std::bit_cast: [unsafe, (const From& from) -> To]
// }
//
// Shared-from-this (can throw if misused):
// @external: {
//   std::enable_shared_from_this::shared_from_this: [unsafe, () -> std::shared_ptr<T>]
//   std::enable_shared_from_this::weak_from_this: [unsafe, () -> std::weak_ptr<T>]
// }

// ============================================================================
// Type Utilities (Truly Safe)
// ============================================================================

// Type utilities that are truly safe (no pointers, no type reinterpretation)
// @external: {
//   std::as_const: [safe, (T& value) -> const T& where value: 'a, return: 'a]
//   std::to_underlying: [safe, (Enum e) -> std::underlying_type_t<Enum>]
// }

// ============================================================================
// Containers - Constructors and Basic Operations
// ============================================================================

// std::vector operations
// @external: {
//   std::vector::push_back: [safe, (const T& value) -> void]
//   std::vector::emplace_back: [safe, (Args&&... args) -> T&]
//   std::vector::pop_back: [safe, () -> void]
//   std::vector::clear: [safe, () -> void]
//   std::vector::size: [safe, () const -> size_t]
//   std::vector::empty: [safe, () const -> bool]
//   std::vector::capacity: [safe, () const -> size_t]
//   std::vector::reserve: [safe, (size_t n) -> void]
//   std::vector::resize: [safe, (size_t n) -> void]
//   std::vector::operator[]: [safe, (size_t n) -> T& where this: 'a, return: 'a]
//   std::vector::at: [safe, (size_t n) -> T& where this: 'a, return: 'a]
//   std::vector::front: [safe, () -> T& where this: 'a, return: 'a]
//   std::vector::back: [safe, () -> T& where this: 'a, return: 'a]
//   std::vector::data: [safe, () -> T* where this: 'a, return: 'a]
//   std::vector::begin: [safe, () -> iterator where this: 'a, return: 'a]
//   std::vector::end: [safe, () -> iterator where this: 'a, return: 'a]
// }

// std::string operations
// @external: {
//   std::string::size: [safe, () const -> size_t]
//   std::string::length: [safe, () const -> size_t]
//   std::string::empty: [safe, () const -> bool]
//   std::string::clear: [safe, () -> void]
//   std::string::operator[]: [safe, (size_t n) -> char& where this: 'a, return: 'a]
//   std::string::at: [safe, (size_t n) -> char& where this: 'a, return: 'a]
//   std::string::front: [safe, () -> char& where this: 'a, return: 'a]
//   std::string::back: [safe, () -> char& where this: 'a, return: 'a]
//   std::string::c_str: [safe, () const -> const char* where this: 'a, return: 'a]
//   std::string::data: [safe, () -> char* where this: 'a, return: 'a]
//   std::string::append: [safe, (const std::string& str) -> std::string&]
//   std::string::operator+=: [safe, (const std::string& str) -> std::string&]
//   std::string::operator+: [safe, (const std::string& lhs, const std::string& rhs) -> std::string]
//   std::string::substr: [safe, (size_t pos, size_t len) const -> std::string]
//   std::string::find: [safe, (const std::string& str) const -> size_t]
// }

// std::map operations
// @external: {
//   std::map::operator[]: [safe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::map::at: [safe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::map::insert: [safe, (const pair<Key,Value>& val) -> pair<iterator,bool>]
//   std::map::emplace: [safe, (Args&&... args) -> pair<iterator,bool>]
//   std::map::erase: [safe, (const Key& key) -> size_t]
//   std::map::find: [safe, (const Key& key) -> iterator where this: 'a, return: 'a]
//   std::map::size: [safe, () const -> size_t]
//   std::map::empty: [safe, () const -> bool]
//   std::map::clear: [safe, () -> void]
//   std::map::begin: [safe, () -> iterator where this: 'a, return: 'a]
//   std::map::end: [safe, () -> iterator where this: 'a, return: 'a]
// }

// std::unordered_map operations
// @external: {
//   std::unordered_map::operator[]: [safe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::unordered_map::at: [safe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::unordered_map::insert: [safe, (const pair<Key,Value>& val) -> pair<iterator,bool>]
//   std::unordered_map::emplace: [safe, (Args&&... args) -> pair<iterator,bool>]
//   std::unordered_map::erase: [safe, (const Key& key) -> size_t]
//   std::unordered_map::find: [safe, (const Key& key) -> iterator where this: 'a, return: 'a]
//   std::unordered_map::size: [safe, () const -> size_t]
//   std::unordered_map::empty: [safe, () const -> bool]
//   std::unordered_map::clear: [safe, () -> void]
//   std::unordered_map::begin: [safe, () -> iterator where this: 'a, return: 'a]
//   std::unordered_map::end: [safe, () -> iterator where this: 'a, return: 'a]
// }

// std::set operations
// @external: {
//   std::set::insert: [safe, (const T& value) -> pair<iterator,bool>]
//   std::set::emplace: [safe, (Args&&... args) -> pair<iterator,bool>]
//   std::set::erase: [safe, (const T& value) -> size_t]
//   std::set::find: [safe, (const T& value) const -> const_iterator where this: 'a, return: 'a]
//   std::set::count: [safe, (const T& value) const -> size_t]
//   std::set::size: [safe, () const -> size_t]
//   std::set::empty: [safe, () const -> bool]
//   std::set::clear: [safe, () -> void]
//   std::set::begin: [safe, () -> iterator where this: 'a, return: 'a]
//   std::set::end: [safe, () -> iterator where this: 'a, return: 'a]
// }

// std::unordered_set operations (similar to set)
// @external: {
//   std::unordered_set::insert: [safe, (const T& value) -> pair<iterator,bool>]
//   std::unordered_set::emplace: [safe, (Args&&... args) -> pair<iterator,bool>]
//   std::unordered_set::erase: [safe, (const T& value) -> size_t]
//   std::unordered_set::find: [safe, (const T& value) const -> const_iterator where this: 'a, return: 'a]
//   std::unordered_set::count: [safe, (const T& value) const -> size_t]
//   std::unordered_set::size: [safe, () const -> size_t]
//   std::unordered_set::empty: [safe, () const -> bool]
//   std::unordered_set::clear: [safe, () -> void]
// }

// std::pair operations
// @external: {
//   std::make_pair: [safe, (T1&& first, T2&& second) -> pair<T1,T2>]
//   std::pair::first: [safe, field -> T1&]
//   std::pair::second: [safe, field -> T2&]
// }

// std::tuple operations
// @external: {
//   std::make_tuple: [safe, (Args&&... args) -> tuple<Args...>]
//   std::get: [safe, (tuple<Args...>& t) -> T& where t: 'a, return: 'a]
//   std::tuple_size: [safe, type_trait -> size_t]
// }

// std::optional operations (C++17)
// @external: {
//   std::make_optional: [safe, (T&& value) -> optional<T>]
//   std::optional::value: [safe, () -> T& where this: 'a, return: 'a]
//   std::optional::value_or: [safe, (T&& default_val) const -> T]
//   std::optional::has_value: [safe, () const -> bool]
//   std::optional::operator*: [safe, () -> T& where this: 'a, return: 'a]
//   std::optional::operator->: [safe, () -> T* where this: 'a, return: 'a]
//   std::optional::operator bool: [safe, () const -> bool]
//   std::optional::reset: [safe, () -> void]
// }

// std::variant operations (C++17)
// @external: {
//   std::holds_alternative: [safe, (const variant<Ts...>& v) -> bool]
//   std::get: [safe, (variant<Ts...>& v) -> T& where v: 'a, return: 'a]
//   std::get_if: [safe, (variant<Ts...>* v) -> T* where v: 'a, return: 'a]
//   std::visit: [safe, (Visitor&& vis, variant<Ts...>& v) -> decltype(auto)]
// }

// ============================================================================
// Algorithms - Common STL Algorithms
// ============================================================================

// Non-modifying sequence operations
// @external: {
//   std::find: [safe, (InputIt first, InputIt last, const T& value) -> InputIt where first: 'a, return: 'a]
//   std::find_if: [safe, (InputIt first, InputIt last, UnaryPred pred) -> InputIt where first: 'a, return: 'a]
//   std::find_if_not: [safe, (InputIt first, InputIt last, UnaryPred pred) -> InputIt where first: 'a, return: 'a]
//   std::count: [safe, (InputIt first, InputIt last, const T& value) -> typename iterator_traits<InputIt>::difference_type]
//   std::count_if: [safe, (InputIt first, InputIt last, UnaryPred pred) -> typename iterator_traits<InputIt>::difference_type]
//   std::all_of: [safe, (InputIt first, InputIt last, UnaryPred pred) -> bool]
//   std::any_of: [safe, (InputIt first, InputIt last, UnaryPred pred) -> bool]
//   std::none_of: [safe, (InputIt first, InputIt last, UnaryPred pred) -> bool]
//   std::for_each: [safe, (InputIt first, InputIt last, UnaryFunc func) -> UnaryFunc]
// }

// Modifying sequence operations
// @external: {
//   std::copy: [safe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::copy_if: [safe, (InputIt first, InputIt last, OutputIt d_first, UnaryPred pred) -> OutputIt where d_first: 'a, return: 'a]
//   std::copy_n: [safe, (InputIt first, Size count, OutputIt result) -> OutputIt where result: 'a, return: 'a]
//   std::move: [safe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::fill: [safe, (ForwardIt first, ForwardIt last, const T& value) -> void]
//   std::fill_n: [safe, (OutputIt first, Size count, const T& value) -> OutputIt where first: 'a, return: 'a]
//   std::transform: [safe, (InputIt first, InputIt last, OutputIt d_first, UnaryOp op) -> OutputIt where d_first: 'a, return: 'a]
//   std::generate: [safe, (ForwardIt first, ForwardIt last, Generator gen) -> void]
//   std::remove: [safe, (ForwardIt first, ForwardIt last, const T& value) -> ForwardIt where first: 'a, return: 'a]
//   std::remove_if: [safe, (ForwardIt first, ForwardIt last, UnaryPred pred) -> ForwardIt where first: 'a, return: 'a]
//   std::replace: [safe, (ForwardIt first, ForwardIt last, const T& old_val, const T& new_val) -> void]
//   std::replace_if: [safe, (ForwardIt first, ForwardIt last, UnaryPred pred, const T& new_val) -> void]
// }

// Sorting and searching
// @external: {
//   std::sort: [safe, (RandomIt first, RandomIt last) -> void]
//   std::stable_sort: [safe, (RandomIt first, RandomIt last) -> void]
//   std::partial_sort: [safe, (RandomIt first, RandomIt middle, RandomIt last) -> void]
//   std::is_sorted: [safe, (ForwardIt first, ForwardIt last) -> bool]
//   std::binary_search: [safe, (ForwardIt first, ForwardIt last, const T& value) -> bool]
//   std::lower_bound: [safe, (ForwardIt first, ForwardIt last, const T& value) -> ForwardIt where first: 'a, return: 'a]
//   std::upper_bound: [safe, (ForwardIt first, ForwardIt last, const T& value) -> ForwardIt where first: 'a, return: 'a]
//   std::equal_range: [safe, (ForwardIt first, ForwardIt last, const T& value) -> pair<ForwardIt,ForwardIt>]
//   std::min: [safe, (const T& a, const T& b) -> const T& where a: 'a, b: 'a, return: 'a]
//   std::max: [safe, (const T& a, const T& b) -> const T& where a: 'a, b: 'a, return: 'a]
//   std::minmax: [safe, (const T& a, const T& b) -> pair<const T&, const T&>]
//   std::min_element: [safe, (ForwardIt first, ForwardIt last) -> ForwardIt where first: 'a, return: 'a]
//   std::max_element: [safe, (ForwardIt first, ForwardIt last) -> ForwardIt where first: 'a, return: 'a]
// }

// Set operations
// @external: {
//   std::set_union: [safe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, InputIt2 last2, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::set_intersection: [safe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, InputIt2 last2, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::set_difference: [safe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, InputIt2 last2, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
// }

// Numeric operations
// @external: {
//   std::accumulate: [safe, (InputIt first, InputIt last, T init) -> T]
//   std::inner_product: [safe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, T init) -> T]
//   std::adjacent_difference: [safe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::partial_sum: [safe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
// }

// ============================================================================
// Input/Output - iostream operations
// ============================================================================

// std::cout, std::cin, std::cerr operations
// @external: {
//   std::cout.operator<<: [safe, (const T& value) -> std::ostream&]
//   std::cin.operator>>: [safe, (T& value) -> std::istream&]
//   std::cerr.operator<<: [safe, (const T& value) -> std::ostream&]
//   std::clog.operator<<: [safe, (const T& value) -> std::ostream&]
//   std::endl: [safe, (std::ostream& os) -> std::ostream&]
//   std::flush: [safe, (std::ostream& os) -> std::ostream&]
//   std::getline: [safe, (std::istream& is, std::string& str) -> std::istream&]
// }

// File streams
// @external: {
//   std::ifstream::open: [safe, (const std::string& filename) -> void]
//   std::ifstream::close: [safe, () -> void]
//   std::ifstream::is_open: [safe, () const -> bool]
//   std::ifstream::good: [safe, () const -> bool]
//   std::ifstream::eof: [safe, () const -> bool]
//   std::ofstream::open: [safe, (const std::string& filename) -> void]
//   std::ofstream::close: [safe, () -> void]
//   std::ofstream::is_open: [safe, () const -> bool]
//   std::ofstream::good: [safe, () const -> bool]
// }

// String streams
// @external: {
//   std::stringstream::str: [safe, () const -> std::string]
//   std::stringstream::str: [safe, (const std::string& s) -> void]
//   std::ostringstream::str: [safe, () const -> std::string]
//   std::istringstream::str: [safe, (const std::string& s) -> void]
// }

// ============================================================================
// Utilities - Type Traits and Meta-programming
// ============================================================================

// Type traits (compile-time, always safe)
// @external: {
//   std::is_same: [safe, type_trait -> bool]
//   std::is_integral: [safe, type_trait -> bool]
//   std::is_floating_point: [safe, type_trait -> bool]
//   std::is_pointer: [safe, type_trait -> bool]
//   std::is_reference: [safe, type_trait -> bool]
//   std::is_const: [safe, type_trait -> bool]
//   std::is_move_constructible: [safe, type_trait -> bool]
//   std::is_copy_constructible: [safe, type_trait -> bool]
//   std::enable_if: [safe, type_trait -> type]
//   std::decay: [safe, type_trait -> type]
//   std::remove_reference: [safe, type_trait -> type]
//   std::remove_const: [safe, type_trait -> type]
// }

// ============================================================================
// Threading - Basic thread-safe operations
// ============================================================================

// Note: Most threading operations are inherently unsafe due to data races.
// Only marking truly safe operations here.

// @external: {
//   std::mutex::lock: [unsafe, () -> void]
//   std::mutex::unlock: [unsafe, () -> void]
//   std::mutex::try_lock: [unsafe, () -> bool]
//   std::lock_guard: [safe, constructor (std::mutex& m)]
//   std::unique_lock: [safe, constructor (std::mutex& m)]
//   std::this_thread::sleep_for: [safe, (const std::chrono::duration& d) -> void]
//   std::this_thread::yield: [safe, () -> void]
//   std::this_thread::get_id: [safe, () -> std::thread::id]
// }

// ============================================================================
// Chrono - Time operations (all safe)
// ============================================================================

// @external: {
//   std::chrono::system_clock::now: [safe, () -> std::chrono::time_point]
//   std::chrono::steady_clock::now: [safe, () -> std::chrono::time_point]
//   std::chrono::high_resolution_clock::now: [safe, () -> std::chrono::time_point]
//   std::chrono::duration_cast: [safe, (Duration d) -> TargetDuration]
// }

// ============================================================================
// Functional - Function objects and lambdas
// ============================================================================

// @external: {
//   std::function: [safe, type_wrapper]
//   std::bind: [safe, (Func&& func, Args&&... args) -> unspecified]
//   std::ref: [safe, (T& t) -> reference_wrapper<T> where t: 'a, return: 'a]
//   std::cref: [safe, (const T& t) -> reference_wrapper<const T> where t: 'a, return: 'a]
// }

// ============================================================================
// Memory - Safe memory operations
// ============================================================================

// @external: {
//   std::addressof: [unsafe, (T& arg) -> T*]
//   std::align: [unsafe, (size_t alignment, size_t size, void*& ptr, size_t& space) -> void*]
//   std::allocator::allocate: [unsafe, (size_t n) -> T*]
//   std::allocator::deallocate: [unsafe, (T* p, size_t n) -> void]
//   std::allocator::construct: [unsafe, (T* p, Args&&... args) -> void]
//   std::allocator::destroy: [unsafe, (T* p) -> void]
// }

// ============================================================================
// Usage Examples
// ============================================================================

// Example 1: Using containers in safe code
// @safe
// void container_example() {
//     std::vector<int> vec = {1, 2, 3};
//     vec.push_back(4);                    // OK: marked safe
//     std::sort(vec.begin(), vec.end());   // OK: marked safe
//
//     for (int x : vec) {                  // OK: safe iteration
//         std::cout << x << " ";           // OK: marked safe
//     }
//     std::cout << std::endl;              // OK: marked safe
// }

// Example 2: Using algorithms in safe code
// @safe
// void algorithm_example() {
//     std::vector<int> v1 = {1, 2, 3, 4, 5};
//     std::vector<int> v2(5);
//
//     std::copy(v1.begin(), v1.end(), v2.begin());  // OK: marked safe
//     auto it = std::find(v2.begin(), v2.end(), 3); // OK: marked safe
//
//     if (it != v2.end()) {
//         std::cout << "Found: " << *it << std::endl;  // OK
//     }
// }

// Example 3: Using smart pointers in safe code
// @safe
// void smart_pointer_example() {
//     auto ptr = std::make_unique<int>(42);  // OK: marked safe
//     std::cout << *ptr << std::endl;         // OK: marked safe
//
//     auto sptr = std::make_shared<int>(100); // OK: marked safe
//     std::cout << *sptr << std::endl;         // OK: marked safe
// }

// Example 4: String operations in safe code
// @safe
// void string_example() {
//     std::string s1 = "Hello";
//     std::string s2 = " World";
//     std::string s3 = s1 + s2;  // OK: marked safe
//
//     s3.append("!");            // OK: marked safe
//     std::cout << s3 << std::endl;  // OK: marked safe
// }

#endif // RUSTYCPP_STD_ANNOTATION_HPP
