// Standard Library Annotations for RustyCpp
//
// This header provides safety and lifetime annotations for C++ standard library.
//
// Design principles:
// 1. Non-pointer operations are marked [safe] - they don't expose raw memory
// 2. Pointer-returning/taking operations remain [unsafe] (default, not annotated)
// 3. Lifetime annotations are provided where return value borrows from input
//
// Usage:
//   #include <std_annotation.hpp>
//
//   // @safe - Can use most STL operations directly
//   void my_function() {
//       std::vector<int> vec = {1, 2, 3};
//       vec.push_back(4);
//       if (!vec.empty()) { ... }
//   }

#ifndef RUSTYCPP_STD_ANNOTATION_HPP
#define RUSTYCPP_STD_ANNOTATION_HPP

// ============================================================================
// Utility Functions - Core C++
// ============================================================================

// @external: {
//   std::swap: [safe]
//   std::exchange: [safe]
//   std::move: [safe, (&'a T) -> T&& where return: 'a]
//   std::forward: [safe, (&'a T) -> T&& where return: 'a]
// }

// ============================================================================
// Smart Pointers - Memory Management
// ============================================================================

// std::unique_ptr - pointer operations need lifetime tracking
// @external: {
//   std::make_unique: [safe]
//   std::unique_ptr::operator bool: [safe]
//   std::unique_ptr::get: [safe, (&'a) -> T* where return: 'a]
//   std::unique_ptr::operator*: [safe, (&'a) -> T& where return: 'a]
//   std::unique_ptr::operator->: [safe, (&'a) -> T* where return: 'a]
// }

// std::shared_ptr operations
// @external: {
//   std::make_shared: [safe]
//   std::shared_ptr::operator bool: [safe]
//   std::shared_ptr::use_count: [safe]
//   std::shared_ptr::get: [safe, (&'a) -> T* where return: 'a]
//   std::shared_ptr::operator*: [safe, (&'a) -> T& where return: 'a]
//   std::shared_ptr::operator->: [safe, (&'a) -> T* where return: 'a]
// }

// std::weak_ptr operations
// @external: {
//   std::weak_ptr::lock: [safe]
//   std::weak_ptr::expired: [safe]
//   std::weak_ptr::use_count: [safe]
// }

// ============================================================================
// Containers - std::vector
// ============================================================================

// @external: {
//   std::vector::push_back: [safe]
//   std::vector::emplace_back: [safe]
//   std::vector::pop_back: [safe]
//   std::vector::clear: [safe]
//   std::vector::size: [safe]
//   std::vector::empty: [safe]
//   std::vector::capacity: [safe]
//   std::vector::reserve: [safe]
//   std::vector::resize: [safe]
//   std::vector::shrink_to_fit: [safe]
//   std::vector::operator[]: [safe, (&'a, size_t) -> T& where return: 'a]
//   std::vector::at: [safe, (&'a, size_t) -> T& where return: 'a]
//   std::vector::front: [safe, (&'a) -> T& where return: 'a]
//   std::vector::back: [safe, (&'a) -> T& where return: 'a]
//   std::vector::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::vector::end: [safe, (&'a) -> iterator]
//   std::vector::rbegin: [safe, (&'a) -> reverse_iterator where return: 'a]
//   std::vector::rend: [safe, (&'a) -> reverse_iterator]
//   std::vector::cbegin: [safe, (&'a) -> const_iterator where return: 'a]
//   std::vector::cend: [safe, (&'a) -> const_iterator]
//   std::vector::data: [safe, (&'a) -> T* where return: 'a]
// }

// ============================================================================
// Containers - std::list
// ============================================================================

// @external: {
//   std::list::push_back: [safe]
//   std::list::push_front: [safe]
//   std::list::emplace_back: [safe]
//   std::list::emplace_front: [safe]
//   std::list::pop_back: [safe]
//   std::list::pop_front: [safe]
//   std::list::insert: [safe]
//   std::list::erase: [safe]
//   std::list::clear: [safe]
//   std::list::size: [safe]
//   std::list::empty: [safe]
//   std::list::front: [safe, (&'a) -> T& where return: 'a]
//   std::list::back: [safe, (&'a) -> T& where return: 'a]
//   std::list::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::list::end: [safe, (&'a) -> iterator]
//   std::__cxx11::list::push_back: [safe]
//   std::__cxx11::list::push_front: [safe]
//   std::__cxx11::list::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::__cxx11::list::end: [safe, (&'a) -> iterator]
//   std::__cxx11::list::empty: [safe]
//   std::__cxx11::list::size: [safe]
// }

// ============================================================================
// Containers - std::deque
// ============================================================================

// @external: {
//   std::deque::push_back: [safe]
//   std::deque::push_front: [safe]
//   std::deque::emplace_back: [safe]
//   std::deque::emplace_front: [safe]
//   std::deque::pop_back: [safe]
//   std::deque::pop_front: [safe]
//   std::deque::clear: [safe]
//   std::deque::size: [safe]
//   std::deque::empty: [safe]
//   std::deque::operator[]: [safe, (&'a, size_t) -> T& where return: 'a]
//   std::deque::at: [safe, (&'a, size_t) -> T& where return: 'a]
//   std::deque::front: [safe, (&'a) -> T& where return: 'a]
//   std::deque::back: [safe, (&'a) -> T& where return: 'a]
//   std::deque::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::deque::end: [safe, (&'a) -> iterator]
// }

// ============================================================================
// Containers - std::string
// ============================================================================

// @external: {
//   std::string::size: [safe]
//   std::string::length: [safe]
//   std::string::empty: [safe]
//   std::string::clear: [safe]
//   std::string::reserve: [safe]
//   std::string::shrink_to_fit: [safe]
//   std::string::push_back: [safe]
//   std::string::pop_back: [safe]
//   std::string::append: [safe]
//   std::string::operator+=: [safe]
//   std::string::operator+: [safe]
//   std::string::substr: [safe]
//   std::string::find: [safe]
//   std::string::rfind: [safe]
//   std::string::compare: [safe]
//   std::string::operator[]: [safe, (&'a, size_t) -> char& where return: 'a]
//   std::string::at: [safe, (&'a, size_t) -> char& where return: 'a]
//   std::string::front: [safe, (&'a) -> char& where return: 'a]
//   std::string::back: [safe, (&'a) -> char& where return: 'a]
//   std::string::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::string::end: [safe, (&'a) -> iterator]
//   std::string::c_str: [safe, (&'a) -> const char* where return: 'a]
//   std::string::data: [safe, (&'a) -> char* where return: 'a]
// }

// ============================================================================
// Containers - std::map
// ============================================================================

// @external: {
//   std::map::insert: [safe]
//   std::map::insert_or_assign: [safe]
//   std::map::emplace: [safe]
//   std::map::erase: [safe]
//   std::map::clear: [safe]
//   std::map::size: [safe]
//   std::map::empty: [safe]
//   std::map::count: [safe]
//   std::map::contains: [safe]
//   std::map::operator[]: [safe, (&'a mut, const K&) -> V& where return: 'a]
//   std::map::at: [safe, (&'a, const K&) -> V& where return: 'a]
//   std::map::find: [safe, (&'a, const K&) -> iterator where return: 'a]
//   std::map::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::map::end: [safe, (&'a) -> iterator]
// }

// ============================================================================
// Containers - std::unordered_map
// ============================================================================

// @external: {
//   std::unordered_map::insert: [safe]
//   std::unordered_map::insert_or_assign: [safe]
//   std::unordered_map::emplace: [safe]
//   std::unordered_map::erase: [safe]
//   std::unordered_map::clear: [safe]
//   std::unordered_map::size: [safe]
//   std::unordered_map::empty: [safe]
//   std::unordered_map::count: [safe]
//   std::unordered_map::contains: [safe]
//   std::unordered_map::operator[]: [safe, (&'a mut, const K&) -> V& where return: 'a]
//   std::unordered_map::at: [safe, (&'a, const K&) -> V& where return: 'a]
//   std::unordered_map::find: [safe, (&'a, const K&) -> iterator where return: 'a]
//   std::unordered_map::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::unordered_map::end: [safe, (&'a) -> iterator]
// }

// ============================================================================
// Containers - std::set
// ============================================================================

// @external: {
//   std::set::insert: [safe]
//   std::set::emplace: [safe]
//   std::set::erase: [safe]
//   std::set::clear: [safe]
//   std::set::size: [safe]
//   std::set::empty: [safe]
//   std::set::count: [safe]
//   std::set::contains: [safe]
//   std::set::find: [safe, (&'a, const T&) -> iterator where return: 'a]
//   std::set::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::set::end: [safe, (&'a) -> iterator]
// }

// ============================================================================
// Containers - std::unordered_set
// ============================================================================

// @external: {
//   std::unordered_set::insert: [safe]
//   std::unordered_set::emplace: [safe]
//   std::unordered_set::erase: [safe]
//   std::unordered_set::clear: [safe]
//   std::unordered_set::size: [safe]
//   std::unordered_set::empty: [safe]
//   std::unordered_set::count: [safe]
//   std::unordered_set::contains: [safe]
//   std::unordered_set::find: [safe, (&'a, const T&) -> iterator where return: 'a]
//   std::unordered_set::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::unordered_set::end: [safe, (&'a) -> iterator]
// }

// ============================================================================
// Containers - std::array
// ============================================================================

// @external: {
//   std::array::size: [safe]
//   std::array::empty: [safe]
//   std::array::fill: [safe]
//   std::array::operator[]: [safe, (&'a, size_t) -> T& where return: 'a]
//   std::array::at: [safe, (&'a, size_t) -> T& where return: 'a]
//   std::array::front: [safe, (&'a) -> T& where return: 'a]
//   std::array::back: [safe, (&'a) -> T& where return: 'a]
//   std::array::begin: [safe, (&'a) -> iterator where return: 'a]
//   std::array::end: [safe, (&'a) -> iterator]
//   std::array::data: [safe, (&'a) -> T* where return: 'a]
// }

// ============================================================================
// Utility Types - std::pair, std::tuple, std::optional, std::variant
// ============================================================================

// @external: {
//   std::make_pair: [safe]
//   std::make_tuple: [safe]
//   std::get: [safe, (&'a tuple) -> T& where return: 'a]
//   std::tie: [safe]
// }

// @external: {
//   std::make_optional: [safe]
//   std::optional::has_value: [safe]
//   std::optional::operator bool: [safe]
//   std::optional::reset: [safe]
//   std::optional::value: [safe, (&'a) -> T& where return: 'a]
//   std::optional::value_or: [safe]
//   std::optional::operator*: [safe, (&'a) -> T& where return: 'a]
//   std::optional::operator->: [safe, (&'a) -> T* where return: 'a]
// }

// @external: {
//   std::holds_alternative: [safe]
//   std::get: [safe, (&'a variant) -> T& where return: 'a]
//   std::get_if: [safe, (&'a variant) -> T* where return: 'a]
//   std::visit: [safe]
// }

// ============================================================================
// Algorithms - Safe operations
// ============================================================================

// @external: {
//   std::sort: [safe]
//   std::stable_sort: [safe]
//   std::partial_sort: [safe]
//   std::is_sorted: [safe]
//   std::reverse: [safe]
//   std::rotate: [safe]
//   std::shuffle: [safe]
//   std::unique: [safe]
//   std::fill: [safe]
//   std::fill_n: [safe]
//   std::copy: [safe]
//   std::copy_n: [safe]
//   std::copy_if: [safe]
//   std::transform: [safe]
//   std::replace: [safe]
//   std::replace_if: [safe]
//   std::swap_ranges: [safe]
//   std::count: [safe]
//   std::count_if: [safe]
//   std::all_of: [safe]
//   std::any_of: [safe]
//   std::none_of: [safe]
//   std::for_each: [safe]
//   std::binary_search: [safe]
//   std::accumulate: [safe]
//   std::reduce: [safe]
//   std::inner_product: [safe]
// }

// Algorithms returning iterators (need lifetime tracking)
// @external: {
//   std::find: [safe, (It first, It last, const T&) -> It where first: 'a, return: 'a]
//   std::find_if: [safe, (It first, It last, Pred) -> It where first: 'a, return: 'a]
//   std::find_if_not: [safe, (It first, It last, Pred) -> It where first: 'a, return: 'a]
//   std::lower_bound: [safe, (It first, It last, const T&) -> It where first: 'a, return: 'a]
//   std::upper_bound: [safe, (It first, It last, const T&) -> It where first: 'a, return: 'a]
//   std::min_element: [safe, (It first, It last) -> It where first: 'a, return: 'a]
//   std::max_element: [safe, (It first, It last) -> It where first: 'a, return: 'a]
//   std::remove: [safe, (It first, It last, const T&) -> It where first: 'a, return: 'a]
//   std::remove_if: [safe, (It first, It last, Pred) -> It where first: 'a, return: 'a]
// }

// @external: {
//   std::min: [safe, (const T& a, const T& b) -> const T& where a: 'a, b: 'a, return: 'a]
//   std::max: [safe, (const T& a, const T& b) -> const T& where a: 'a, b: 'a, return: 'a]
//   std::clamp: [safe, (const T& v, const T& lo, const T& hi) -> const T& where v: 'a, lo: 'a, hi: 'a, return: 'a]
// }

// ============================================================================
// I/O Operations - Safe
// ============================================================================

// @external: {
//   std::cout.operator<<: [safe]
//   std::cerr.operator<<: [safe]
//   std::clog.operator<<: [safe]
//   std::cin.operator>>: [safe]
//   std::endl: [safe]
//   std::flush: [safe]
//   std::getline: [safe]
// }

// @external: {
//   std::ifstream::is_open: [safe]
//   std::ifstream::good: [safe]
//   std::ifstream::eof: [safe]
//   std::ifstream::fail: [safe]
//   std::ifstream::bad: [safe]
//   std::ofstream::is_open: [safe]
//   std::ofstream::good: [safe]
//   std::ofstream::fail: [safe]
//   std::ofstream::bad: [safe]
//   std::stringstream::str: [safe]
//   std::ostringstream::str: [safe]
//   std::istringstream::str: [safe]
// }

// ============================================================================
// Threading - Safe operations
// ============================================================================

// @external: {
//   std::mutex::lock: [safe]
//   std::mutex::unlock: [safe]
//   std::mutex::try_lock: [safe]
//   std::lock_guard: [safe]
//   std::unique_lock: [safe]
//   std::scoped_lock: [safe]
//   std::this_thread::sleep_for: [safe]
//   std::this_thread::sleep_until: [safe]
//   std::this_thread::yield: [safe]
//   std::this_thread::get_id: [safe]
// }

// ============================================================================
// Chrono - Safe operations
// ============================================================================

// @external: {
//   std::chrono::system_clock::now: [safe]
//   std::chrono::steady_clock::now: [safe]
//   std::chrono::high_resolution_clock::now: [safe]
//   std::chrono::duration_cast: [safe]
//   std::chrono::time_point_cast: [safe]
// }

// ============================================================================
// Functional - Safe operations
// ============================================================================

// @external: {
//   std::function::operator(): [safe]
//   std::function::operator bool: [safe]
//   std::function::operator=: [safe]
//   std::bind: [safe]
//   std::ref: [safe, (&'a T) -> reference_wrapper<T> where return: 'a]
//   std::cref: [safe, (const &'a T) -> reference_wrapper<const T> where return: 'a]
// }

// ============================================================================
// Comparison Operators - Safe
// ============================================================================

// @external: {
//   operator==: [safe]
//   operator!=: [safe]
//   operator<: [safe]
//   operator>: [safe]
//   operator<=: [safe]
//   operator>=: [safe]
//   operator<=>: [safe]
//   std::__detail::operator==: [safe]
//   std::__detail::operator!=: [safe]
// }

// ============================================================================
// Numeric - Safe operations
// ============================================================================

// @external: {
//   std::abs: [safe]
//   std::fabs: [safe]
//   std::sqrt: [safe]
//   std::pow: [safe]
//   std::exp: [safe]
//   std::log: [safe]
//   std::log10: [safe]
//   std::sin: [safe]
//   std::cos: [safe]
//   std::tan: [safe]
//   std::floor: [safe]
//   std::ceil: [safe]
//   std::round: [safe]
// }

// ============================================================================
// Type Utilities with Lifetime - Safe with annotations
// ============================================================================

// @external: {
//   std::as_const: [safe, (&'a T) -> const T& where return: 'a]
//   std::addressof: [safe, (&'a T) -> T* where return: 'a]
// }

#endif // RUSTYCPP_STD_ANNOTATION_HPP
