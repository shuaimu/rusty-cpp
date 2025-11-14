#ifndef RUSTY_HPP
#define RUSTY_HPP

// Rusty - Rust-inspired safe types for C++
//
// This library provides Rust-like types with proper lifetime annotations
// that work with the Rusty C++ Checker to ensure memory safety.
//
// All types follow Rust's ownership and borrowing principles:
// - Single ownership (Box, Vec)
// - Shared immutable access (Rc, Arc) with built-in polymorphism support
// - Explicit nullability (Option)
// - Explicit error handling (Result)

// #include "rusty/std_minimal.hpp"  // Not needed with standard library
#include "rusty/box.hpp"
#include "rusty/arc.hpp"  // Unified Arc with polymorphism (like std::shared_ptr)
#include "rusty/rc.hpp"   // Unified Rc with polymorphism (like std::shared_ptr)
#include "rusty/weak.hpp"  // Compatibility aliases (Weak<T> for Rc, ArcWeak<T> for Arc)
// TODO: Enable once namespace conflicts are resolved
// #include "rusty/rc/weak.hpp"  // Namespace-organized: rusty::rc_impl::Weak<T>
// #include "rusty/sync/weak.hpp"  // Namespace-organized: rusty::sync_impl::Weak<T>
#include "rusty/vec.hpp"
#include "rusty/option.hpp"
#include "rusty/result.hpp"
#include "rusty/cell.hpp"
#include "rusty/refcell.hpp"
#include "rusty/string.hpp"
#include "rusty/hashmap.hpp"
#include "rusty/hashset.hpp"
#include "rusty/btreemap.hpp"
#include "rusty/btreeset.hpp"

// Synchronization primitives (std::sync equivalent)
#include "rusty/mutex.hpp"
#include "rusty/rwlock.hpp"
#include "rusty/condvar.hpp"
#include "rusty/barrier.hpp"
#include "rusty/once.hpp"

// Convenience aliases in rusty namespace
// @safe
namespace rusty {
    // Common Result types
    template<typename T>
    using ResultVoid = Result<T, void>;
    
    template<typename T>
    using ResultString = Result<T, const char*>;
    
    template<typename T>
    using ResultInt = Result<T, int>;
    
    // Smart pointer conversions (Rust-idiomatic names)
    template<typename T>
    // @lifetime: owned
    Box<T> from_raw(T* ptr) {
        return Box<T>(ptr);
    }
    
    // C++ style alias
    template<typename T>
    // @lifetime: owned
    Box<T> box_from_raw(T* ptr) {
        return from_raw(ptr);
    }
    
    template<typename T>
    // @lifetime: owned
    Arc<T> arc_from_box(Box<T>&& box) {
        T* ptr = box.into_raw();
        Arc<T> result = Arc<T>::new_(std::move(*ptr));
        delete ptr;
        return result;
    }
    
    template<typename T>
    // @lifetime: owned
    Rc<T> rc_from_box(Box<T>&& box) {
        T* ptr = box.into_raw();
        Rc<T> result = Rc<T>::new_(std::move(*ptr));
        delete ptr;
        return result;
    }
    
    // Rust-style type aliases for convenience
    template<typename T>
    using Boxed = Box<T>;
    
    template<typename T>
    using Shared = Arc<T>;
    
    template<typename T>
    using RefCounted = Rc<T>;
}

#endif // RUSTY_HPP