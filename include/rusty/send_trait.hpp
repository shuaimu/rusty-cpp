#pragma once

#include <type_traits>

// Rust's Send trait equivalent for C++
// Conservative approach: Types are NOT Send unless explicitly marked
//
// This solves the compositional problem:
// - struct { Rc<T> } is NOT Send (default)
// - Must explicitly mark types as Send
// - Much safer than assuming movable = Send

namespace rusty {

// ==================================================================
// EXPLICIT OPT-IN SEND TRAIT SYSTEM
// ==================================================================

// Step 1: Check for static marker (preferred method)
template<typename T, typename = void>
struct has_send_marker : std::false_type {};

template<typename T>
struct has_send_marker<T, std::void_t<decltype(T::is_send)>> : std::true_type {};

// Step 2: Explicit opt-in via specialization
template<typename T>
struct is_explicitly_send : std::false_type {};

// Step 3: Main is_send trait - CONSERVATIVE DEFAULT
template<typename T>
struct is_send {
    static constexpr bool value = []() {
        // Priority 1: Check static marker
        if constexpr (has_send_marker<T>::value) {
            return T::is_send;
        }
        // Priority 2: Check explicit specialization
        else if constexpr (is_explicitly_send<T>::value) {
            return true;
        }
        // Priority 3: CONSERVATIVE - default to NOT Send
        else {
            return false;
        }
    }();
};

// ==================================================================
// BUILT-IN SEND TYPES (primitives and standard types)
// ==================================================================

// Primitive types are Send
template<> struct is_explicitly_send<bool> : std::true_type {};
template<> struct is_explicitly_send<char> : std::true_type {};
template<> struct is_explicitly_send<signed char> : std::true_type {};
template<> struct is_explicitly_send<unsigned char> : std::true_type {};
template<> struct is_explicitly_send<wchar_t> : std::true_type {};
template<> struct is_explicitly_send<char16_t> : std::true_type {};
template<> struct is_explicitly_send<char32_t> : std::true_type {};
template<> struct is_explicitly_send<short> : std::true_type {};
template<> struct is_explicitly_send<unsigned short> : std::true_type {};
template<> struct is_explicitly_send<int> : std::true_type {};
template<> struct is_explicitly_send<unsigned int> : std::true_type {};
template<> struct is_explicitly_send<long> : std::true_type {};
template<> struct is_explicitly_send<unsigned long> : std::true_type {};
template<> struct is_explicitly_send<long long> : std::true_type {};
template<> struct is_explicitly_send<unsigned long long> : std::true_type {};
template<> struct is_explicitly_send<float> : std::true_type {};
template<> struct is_explicitly_send<double> : std::true_type {};
template<> struct is_explicitly_send<long double> : std::true_type {};

// Pointers are Send (but unsafe to use - like Rust)
template<typename T>
struct is_explicitly_send<T*> : std::true_type {};

// References are NOT Send (tied to their referent's lifetime)
// No need to specialize - default is false

// ==================================================================
// HELPER MACRO FOR USER TYPES
// ==================================================================

// Convenience macro to mark types as Send
#define RUSTY_MARK_SEND(Type) \
    namespace rusty { \
        template<> struct is_explicitly_send<Type> : std::true_type {}; \
    }

// Template version
#define RUSTY_MARK_SEND_TEMPLATE(Template, T) \
    namespace rusty { \
        template<typename T> \
        struct is_explicitly_send<Template<T>> : is_send<T> {}; \
    }

} // namespace rusty
