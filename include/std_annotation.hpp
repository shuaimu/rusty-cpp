// Standard Library Lifetime Annotations for RustyCpp
//
// This header provides lifetime annotations for common C++ standard library
// functions. Since RustyCpp does not verify external code, all std functions
// are marked as [unsafe] - they must be called from @unsafe contexts.
//
// Usage:
//   #include <std_annotation.hpp>
//
//   // std functions require @unsafe context:
//   // @unsafe
//   void my_function() {
//       std::vector<int> vec = {1, 2, 3};
//       std::sort(vec.begin(), vec.end());
//       std::cout << "Hello" << std::endl;
//   }
//
// NOTE: All std library functions are [unsafe] because RustyCpp cannot verify
// external code. The programmer takes responsibility for correct usage.

#ifndef RUSTYCPP_STD_ANNOTATION_HPP
#define RUSTYCPP_STD_ANNOTATION_HPP

// ============================================================================
// Utility Functions - Core C++
// ============================================================================

// @external: {
//   std::move: [unsafe, (T& x) -> T&& where x: 'a, return: 'a]
//   std::forward: [unsafe, (T& x) -> T&& where x: 'a, return: 'a]
//   std::swap: [unsafe, (T& a, T& b) -> void]
//   std::exchange: [unsafe, (T& obj, U&& new_val) -> T]
// }

// ============================================================================
// Smart Pointers - Memory Management
// ============================================================================

// std::unique_ptr operations
// @external: {
//   std::make_unique: [unsafe, (Args&&... args) -> owned std::unique_ptr<T>]
//   std::unique_ptr::get: [unsafe, () -> T* where this: 'a, return: 'a]
//   std::unique_ptr::release: [unsafe, () -> owned T*]
//   std::unique_ptr::reset: [unsafe, (T* ptr) -> void]
//   std::unique_ptr::operator*: [unsafe, () -> T& where this: 'a, return: 'a]
//   std::unique_ptr::operator->: [unsafe, () -> T* where this: 'a, return: 'a]
//   std::unique_ptr::operator bool: [unsafe, () const -> bool]
// }

// std::shared_ptr operations
// @external: {
//   std::make_shared: [unsafe, (Args&&... args) -> owned std::shared_ptr<T>]
//   std::shared_ptr::get: [unsafe, () const -> T* where this: 'a, return: 'a]
//   std::shared_ptr::reset: [unsafe, (T* ptr) -> void]
//   std::shared_ptr::operator*: [unsafe, () const -> const T& where this: 'a, return: 'a]
//   std::shared_ptr::operator->: [unsafe, () const -> const T* where this: 'a, return: 'a]
//   std::shared_ptr::operator bool: [unsafe, () const -> bool]
//   std::shared_ptr::use_count: [unsafe, () const -> long]
// }

// std::weak_ptr operations
// @external: {
//   std::weak_ptr::lock: [unsafe, () const -> std::shared_ptr<T>]
//   std::weak_ptr::expired: [unsafe, () const -> bool]
//   std::weak_ptr::use_count: [unsafe, () const -> long]
// }

// Smart pointer casts
// @external: {
//   std::dynamic_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
//   std::static_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
//   std::const_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
//   std::reinterpret_pointer_cast: [unsafe, (const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
// }

// C++ cast operators
// @external: {
//   dynamic_cast: [unsafe, (T* ptr) -> U*]
//   static_cast: [unsafe, (T value) -> U]
//   const_cast: [unsafe, (T value) -> U]
//   reinterpret_cast: [unsafe, (T value) -> U]
// }

// ============================================================================
// Type Utilities
// ============================================================================

// @external: {
//   std::as_const: [unsafe, (T& value) -> const T& where value: 'a, return: 'a]
//   std::to_underlying: [unsafe, (Enum e) -> std::underlying_type_t<Enum>]
//   std::addressof: [unsafe, (T& value) -> T* where value: 'a, return: 'a]
//   std::launder: [unsafe, (T* ptr) -> T* where ptr: 'a, return: 'a]
//   std::bit_cast: [unsafe, (const From& from) -> To]
// }

// ============================================================================
// Containers - Constructors and Basic Operations
// ============================================================================

// std::vector operations
// @external: {
//   std::vector::push_back: [unsafe, (const T& value) -> void]
//   std::vector::emplace_back: [unsafe, (Args&&... args) -> T&]
//   std::vector::pop_back: [unsafe, () -> void]
//   std::vector::clear: [unsafe, () -> void]
//   std::vector::size: [unsafe, () const -> size_t]
//   std::vector::empty: [unsafe, () const -> bool]
//   std::vector::capacity: [unsafe, () const -> size_t]
//   std::vector::reserve: [unsafe, (size_t n) -> void]
//   std::vector::resize: [unsafe, (size_t n) -> void]
//   std::vector::operator[]: [unsafe, (size_t n) -> T& where this: 'a, return: 'a]
//   std::vector::at: [unsafe, (size_t n) -> T& where this: 'a, return: 'a]
//   std::vector::front: [unsafe, () -> T& where this: 'a, return: 'a]
//   std::vector::back: [unsafe, () -> T& where this: 'a, return: 'a]
//   std::vector::data: [unsafe, () -> T* where this: 'a, return: 'a]
//   std::vector::begin: [unsafe, () -> iterator where this: 'a, return: 'a]
//   std::vector::end: [unsafe, () -> iterator where this: 'a, return: 'a]
// }

// std::string operations
// @external: {
//   std::string::size: [unsafe, () const -> size_t]
//   std::string::length: [unsafe, () const -> size_t]
//   std::string::empty: [unsafe, () const -> bool]
//   std::string::clear: [unsafe, () -> void]
//   std::string::operator[]: [unsafe, (size_t n) -> char& where this: 'a, return: 'a]
//   std::string::at: [unsafe, (size_t n) -> char& where this: 'a, return: 'a]
//   std::string::front: [unsafe, () -> char& where this: 'a, return: 'a]
//   std::string::back: [unsafe, () -> char& where this: 'a, return: 'a]
//   std::string::c_str: [unsafe, () const -> const char* where this: 'a, return: 'a]
//   std::string::data: [unsafe, () -> char* where this: 'a, return: 'a]
//   std::string::append: [unsafe, (const std::string& str) -> std::string&]
//   std::string::operator+=: [unsafe, (const std::string& str) -> std::string&]
//   std::string::operator+: [unsafe, (const std::string& lhs, const std::string& rhs) -> std::string]
//   std::string::substr: [unsafe, (size_t pos, size_t len) const -> std::string]
//   std::string::find: [unsafe, (const std::string& str) const -> size_t]
// }

// std::map operations
// @external: {
//   std::map::operator[]: [unsafe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::map::at: [unsafe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::map::insert: [unsafe, (const pair<Key,Value>& val) -> pair<iterator,bool>]
//   std::map::emplace: [unsafe, (Args&&... args) -> pair<iterator,bool>]
//   std::map::erase: [unsafe, (const Key& key) -> size_t]
//   std::map::find: [unsafe, (const Key& key) -> iterator where this: 'a, return: 'a]
//   std::map::size: [unsafe, () const -> size_t]
//   std::map::empty: [unsafe, () const -> bool]
//   std::map::clear: [unsafe, () -> void]
//   std::map::begin: [unsafe, () -> iterator where this: 'a, return: 'a]
//   std::map::end: [unsafe, () -> iterator where this: 'a, return: 'a]
// }

// std::unordered_map operations
// @external: {
//   std::unordered_map::operator[]: [unsafe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::unordered_map::at: [unsafe, (const Key& key) -> Value& where this: 'a, return: 'a]
//   std::unordered_map::insert: [unsafe, (const pair<Key,Value>& val) -> pair<iterator,bool>]
//   std::unordered_map::emplace: [unsafe, (Args&&... args) -> pair<iterator,bool>]
//   std::unordered_map::erase: [unsafe, (const Key& key) -> size_t]
//   std::unordered_map::find: [unsafe, (const Key& key) -> iterator where this: 'a, return: 'a]
//   std::unordered_map::insert_or_assign: [unsafe, (const Key& key, M&& obj) -> pair<iterator,bool>]
//   std::unordered_map::size: [unsafe, () const -> size_t]
//   std::unordered_map::empty: [unsafe, () const -> bool]
//   std::unordered_map::clear: [unsafe, () -> void]
//   std::unordered_map::begin: [unsafe, () -> iterator where this: 'a, return: 'a]
//   std::unordered_map::end: [unsafe, () -> iterator where this: 'a, return: 'a]
// }

// std::set operations
// @external: {
//   std::set::set: [unsafe, () -> void]
//   std::set::insert: [unsafe, (const T& value) -> pair<iterator,bool>]
//   std::set::emplace: [unsafe, (Args&&... args) -> pair<iterator,bool>]
//   std::set::erase: [unsafe, (const T& value) -> size_t]
//   std::set::find: [unsafe, (const T& value) const -> const_iterator where this: 'a, return: 'a]
//   std::set::count: [unsafe, (const T& value) const -> size_t]
//   std::set::size: [unsafe, () const -> size_t]
//   std::set::empty: [unsafe, () const -> bool]
//   std::set::clear: [unsafe, () -> void]
//   std::set::begin: [unsafe, () -> iterator where this: 'a, return: 'a]
//   std::set::end: [unsafe, () -> iterator where this: 'a, return: 'a]
// }

// std::unordered_set operations
// @external: {
//   std::unordered_set::unordered_set: [unsafe, () -> void]
//   std::unordered_set::insert: [unsafe, (const T& value) -> pair<iterator,bool>]
//   std::unordered_set::emplace: [unsafe, (Args&&... args) -> pair<iterator,bool>]
//   std::unordered_set::erase: [unsafe, (const T& value) -> size_t]
//   std::unordered_set::find: [unsafe, (const T& value) const -> const_iterator where this: 'a, return: 'a]
//   std::unordered_set::count: [unsafe, (const T& value) const -> size_t]
//   std::unordered_set::size: [unsafe, () const -> size_t]
//   std::unordered_set::empty: [unsafe, () const -> bool]
//   std::unordered_set::clear: [unsafe, () -> void]
//   std::unordered_set::swap: [unsafe, (unordered_set& other) -> void]
// }

// std::pair operations
// @external: {
//   std::make_pair: [unsafe, (T1&& first, T2&& second) -> pair<T1,T2>]
//   std::pair::first: [unsafe, field -> T1&]
//   std::pair::second: [unsafe, field -> T2&]
// }

// std::tuple operations
// @external: {
//   std::make_tuple: [unsafe, (Args&&... args) -> tuple<Args...>]
//   std::get: [unsafe, (tuple<Args...>& t) -> T& where t: 'a, return: 'a]
//   std::tuple_size: [unsafe, type_trait -> size_t]
// }

// std::optional operations (C++17)
// @external: {
//   std::make_optional: [unsafe, (T&& value) -> optional<T>]
//   std::optional::value: [unsafe, () -> T& where this: 'a, return: 'a]
//   std::optional::value_or: [unsafe, (T&& default_val) const -> T]
//   std::optional::has_value: [unsafe, () const -> bool]
//   std::optional::operator*: [unsafe, () -> T& where this: 'a, return: 'a]
//   std::optional::operator->: [unsafe, () -> T* where this: 'a, return: 'a]
//   std::optional::operator bool: [unsafe, () const -> bool]
//   std::optional::reset: [unsafe, () -> void]
// }

// std::variant operations (C++17)
// @external: {
//   std::holds_alternative: [unsafe, (const variant<Ts...>& v) -> bool]
//   std::get: [unsafe, (variant<Ts...>& v) -> T& where v: 'a, return: 'a]
//   std::get_if: [unsafe, (variant<Ts...>* v) -> T* where v: 'a, return: 'a]
//   std::visit: [unsafe, (Visitor&& vis, variant<Ts...>& v) -> decltype(auto)]
// }

// ============================================================================
// Algorithms - Common STL Algorithms
// ============================================================================

// Non-modifying sequence operations
// @external: {
//   std::find: [unsafe, (InputIt first, InputIt last, const T& value) -> InputIt where first: 'a, return: 'a]
//   std::find_if: [unsafe, (InputIt first, InputIt last, UnaryPred pred) -> InputIt where first: 'a, return: 'a]
//   std::find_if_not: [unsafe, (InputIt first, InputIt last, UnaryPred pred) -> InputIt where first: 'a, return: 'a]
//   std::count: [unsafe, (InputIt first, InputIt last, const T& value) -> typename iterator_traits<InputIt>::difference_type]
//   std::count_if: [unsafe, (InputIt first, InputIt last, UnaryPred pred) -> typename iterator_traits<InputIt>::difference_type]
//   std::all_of: [unsafe, (InputIt first, InputIt last, UnaryPred pred) -> bool]
//   std::any_of: [unsafe, (InputIt first, InputIt last, UnaryPred pred) -> bool]
//   std::none_of: [unsafe, (InputIt first, InputIt last, UnaryPred pred) -> bool]
//   std::for_each: [unsafe, (InputIt first, InputIt last, UnaryFunc func) -> UnaryFunc]
// }

// Modifying sequence operations
// @external: {
//   std::copy: [unsafe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::copy_if: [unsafe, (InputIt first, InputIt last, OutputIt d_first, UnaryPred pred) -> OutputIt where d_first: 'a, return: 'a]
//   std::copy_n: [unsafe, (InputIt first, Size count, OutputIt result) -> OutputIt where result: 'a, return: 'a]
//   std::move: [unsafe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::fill: [unsafe, (ForwardIt first, ForwardIt last, const T& value) -> void]
//   std::fill_n: [unsafe, (OutputIt first, Size count, const T& value) -> OutputIt where first: 'a, return: 'a]
//   std::transform: [unsafe, (InputIt first, InputIt last, OutputIt d_first, UnaryOp op) -> OutputIt where d_first: 'a, return: 'a]
//   std::generate: [unsafe, (ForwardIt first, ForwardIt last, Generator gen) -> void]
//   std::remove: [unsafe, (ForwardIt first, ForwardIt last, const T& value) -> ForwardIt where first: 'a, return: 'a]
//   std::remove_if: [unsafe, (ForwardIt first, ForwardIt last, UnaryPred pred) -> ForwardIt where first: 'a, return: 'a]
//   std::replace: [unsafe, (ForwardIt first, ForwardIt last, const T& old_val, const T& new_val) -> void]
//   std::replace_if: [unsafe, (ForwardIt first, ForwardIt last, UnaryPred pred, const T& new_val) -> void]
// }

// Sorting and searching
// @external: {
//   std::sort: [unsafe, (RandomIt first, RandomIt last) -> void]
//   std::stable_sort: [unsafe, (RandomIt first, RandomIt last) -> void]
//   std::partial_sort: [unsafe, (RandomIt first, RandomIt middle, RandomIt last) -> void]
//   std::is_sorted: [unsafe, (ForwardIt first, ForwardIt last) -> bool]
//   std::binary_search: [unsafe, (ForwardIt first, ForwardIt last, const T& value) -> bool]
//   std::lower_bound: [unsafe, (ForwardIt first, ForwardIt last, const T& value) -> ForwardIt where first: 'a, return: 'a]
//   std::upper_bound: [unsafe, (ForwardIt first, ForwardIt last, const T& value) -> ForwardIt where first: 'a, return: 'a]
//   std::equal_range: [unsafe, (ForwardIt first, ForwardIt last, const T& value) -> pair<ForwardIt,ForwardIt>]
//   std::min: [unsafe, (const T& a, const T& b) -> const T& where a: 'a, b: 'a, return: 'a]
//   std::max: [unsafe, (const T& a, const T& b) -> const T& where a: 'a, b: 'a, return: 'a]
//   std::minmax: [unsafe, (const T& a, const T& b) -> pair<const T&, const T&>]
//   std::min_element: [unsafe, (ForwardIt first, ForwardIt last) -> ForwardIt where first: 'a, return: 'a]
//   std::max_element: [unsafe, (ForwardIt first, ForwardIt last) -> ForwardIt where first: 'a, return: 'a]
// }

// Set operations
// @external: {
//   std::set_union: [unsafe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, InputIt2 last2, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::set_intersection: [unsafe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, InputIt2 last2, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::set_difference: [unsafe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, InputIt2 last2, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
// }

// Numeric operations
// @external: {
//   std::accumulate: [unsafe, (InputIt first, InputIt last, T init) -> T]
//   std::inner_product: [unsafe, (InputIt1 first1, InputIt1 last1, InputIt2 first2, T init) -> T]
//   std::adjacent_difference: [unsafe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
//   std::partial_sum: [unsafe, (InputIt first, InputIt last, OutputIt d_first) -> OutputIt where d_first: 'a, return: 'a]
// }

// ============================================================================
// Input/Output - iostream operations
// ============================================================================

// std::cout, std::cin, std::cerr operations
// @external: {
//   std::cout.operator<<: [unsafe, (const T& value) -> std::ostream&]
//   std::cin.operator>>: [unsafe, (T& value) -> std::istream&]
//   std::cerr.operator<<: [unsafe, (const T& value) -> std::ostream&]
//   std::clog.operator<<: [unsafe, (const T& value) -> std::ostream&]
//   std::endl: [unsafe, (std::ostream& os) -> std::ostream&]
//   std::flush: [unsafe, (std::ostream& os) -> std::ostream&]
//   std::getline: [unsafe, (std::istream& is, std::string& str) -> std::istream&]
// }

// File streams
// @external: {
//   std::ifstream::open: [unsafe, (const std::string& filename) -> void]
//   std::ifstream::close: [unsafe, () -> void]
//   std::ifstream::is_open: [unsafe, () const -> bool]
//   std::ifstream::good: [unsafe, () const -> bool]
//   std::ifstream::eof: [unsafe, () const -> bool]
//   std::ofstream::open: [unsafe, (const std::string& filename) -> void]
//   std::ofstream::close: [unsafe, () -> void]
//   std::ofstream::is_open: [unsafe, () const -> bool]
//   std::ofstream::good: [unsafe, () const -> bool]
// }

// String streams
// @external: {
//   std::stringstream::str: [unsafe, () const -> std::string]
//   std::stringstream::str: [unsafe, (const std::string& s) -> void]
//   std::ostringstream::str: [unsafe, () const -> std::string]
//   std::istringstream::str: [unsafe, (const std::string& s) -> void]
// }

// ============================================================================
// Utilities - Type Traits and Meta-programming
// ============================================================================

// Type traits (compile-time)
// @external: {
//   std::is_same: [unsafe, type_trait -> bool]
//   std::is_integral: [unsafe, type_trait -> bool]
//   std::is_floating_point: [unsafe, type_trait -> bool]
//   std::is_pointer: [unsafe, type_trait -> bool]
//   std::is_reference: [unsafe, type_trait -> bool]
//   std::is_const: [unsafe, type_trait -> bool]
//   std::is_move_constructible: [unsafe, type_trait -> bool]
//   std::is_copy_constructible: [unsafe, type_trait -> bool]
//   std::enable_if: [unsafe, type_trait -> type]
//   std::decay: [unsafe, type_trait -> type]
//   std::remove_reference: [unsafe, type_trait -> type]
//   std::remove_const: [unsafe, type_trait -> type]
// }

// ============================================================================
// Threading - Thread operations
// ============================================================================

// @external: {
//   std::mutex::lock: [unsafe, () -> void]
//   std::mutex::unlock: [unsafe, () -> void]
//   std::mutex::try_lock: [unsafe, () -> bool]
//   std::lock_guard: [unsafe, constructor (std::mutex& m)]
//   std::unique_lock: [unsafe, constructor (std::mutex& m)]
//   std::this_thread::sleep_for: [unsafe, (const std::chrono::duration& d) -> void]
//   std::this_thread::yield: [unsafe, () -> void]
//   std::this_thread::get_id: [unsafe, () -> std::thread::id]
// }

// ============================================================================
// Chrono - Time operations
// ============================================================================

// @external: {
//   std::chrono::system_clock::now: [unsafe, () -> std::chrono::time_point]
//   std::chrono::steady_clock::now: [unsafe, () -> std::chrono::time_point]
//   std::chrono::high_resolution_clock::now: [unsafe, () -> std::chrono::time_point]
//   std::chrono::duration_cast: [unsafe, (Duration d) -> TargetDuration]
// }

// ============================================================================
// Functional - Function objects and lambdas
// ============================================================================

// @external: {
//   std::function: [unsafe, type_wrapper]
//   std::bind: [unsafe, (Func&& func, Args&&... args) -> unspecified]
//   std::ref: [unsafe, (T& t) -> reference_wrapper<T> where t: 'a, return: 'a]
//   std::cref: [unsafe, (const T& t) -> reference_wrapper<const T> where t: 'a, return: 'a]
// }

// ============================================================================
// Memory - Memory operations
// ============================================================================

// @external: {
//   std::align: [unsafe, (size_t alignment, size_t size, void*& ptr, size_t& space) -> void*]
//   std::allocator::allocate: [unsafe, (size_t n) -> T*]
//   std::allocator::deallocate: [unsafe, (T* p, size_t n) -> void]
//   std::allocator::construct: [unsafe, (T* p, Args&&... args) -> void]
//   std::allocator::destroy: [unsafe, (T* p) -> void]
// }

// Shared-from-this
// @external: {
//   std::enable_shared_from_this::shared_from_this: [unsafe, () -> std::shared_ptr<T>]
//   std::enable_shared_from_this::weak_from_this: [unsafe, () -> std::weak_ptr<T>]
// }

// ============================================================================
// Usage Examples
// ============================================================================

// Example 1: Using containers requires @unsafe
// @unsafe
// void container_example() {
//     std::vector<int> vec = {1, 2, 3};
//     vec.push_back(4);
//     std::sort(vec.begin(), vec.end());
//
//     for (int x : vec) {
//         std::cout << x << " ";
//     }
//     std::cout << std::endl;
// }

// Example 2: Using algorithms requires @unsafe
// @unsafe
// void algorithm_example() {
//     std::vector<int> v1 = {1, 2, 3, 4, 5};
//     std::vector<int> v2(5);
//
//     std::copy(v1.begin(), v1.end(), v2.begin());
//     auto it = std::find(v2.begin(), v2.end(), 3);
//
//     if (it != v2.end()) {
//         std::cout << "Found: " << *it << std::endl;
//     }
// }

// Example 3: Using smart pointers requires @unsafe
// @unsafe
// void smart_pointer_example() {
//     auto ptr = std::make_unique<int>(42);
//     std::cout << *ptr << std::endl;
//
//     auto sptr = std::make_shared<int>(100);
//     std::cout << *sptr << std::endl;
// }

#endif // RUSTYCPP_STD_ANNOTATION_HPP
