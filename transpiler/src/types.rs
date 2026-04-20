/// Map a Rust primitive type name to its C++ equivalent.
pub fn map_primitive_type(rust_type: &str) -> Option<&'static str> {
    match rust_type {
        "i8" => Some("int8_t"),
        "i16" => Some("int16_t"),
        "i32" => Some("int32_t"),
        "i64" => Some("int64_t"),
        "i128" => Some("__int128"),
        "u8" => Some("uint8_t"),
        "u16" => Some("uint16_t"),
        "u32" => Some("uint32_t"),
        "u64" => Some("uint64_t"),
        "u128" => Some("unsigned __int128"),
        "f32" => Some("float"),
        "f64" => Some("double"),
        "bool" => Some("bool"),
        "char" => Some("char32_t"),
        "usize" => Some("size_t"),
        "isize" => Some("ptrdiff_t"),
        _ => None,
    }
}

/// Map a Rust standard library type path to its rusty-cpp C++ equivalent.
/// Returns (cpp_type_base, needs_template_args).
pub fn map_std_type(rust_path: &str) -> Option<(&'static str, bool)> {
    match rust_path {
        // Primitive module aliases occasionally appear in expanded code via
        // imports like `use core::usize;`.
        "std::bool" | "core::bool" | "std::primitive::bool" | "core::primitive::bool" => {
            Some(("bool", false))
        }
        "std::char" | "core::char" | "std::primitive::char" | "core::primitive::char" => {
            Some(("char32_t", false))
        }
        "std::i8" | "core::i8" | "std::primitive::i8" | "core::primitive::i8" => {
            Some(("int8_t", false))
        }
        "std::i16" | "core::i16" | "std::primitive::i16" | "core::primitive::i16" => {
            Some(("int16_t", false))
        }
        "std::i32" | "core::i32" | "std::primitive::i32" | "core::primitive::i32" => {
            Some(("int32_t", false))
        }
        "std::i64" | "core::i64" | "std::primitive::i64" | "core::primitive::i64" => {
            Some(("int64_t", false))
        }
        "std::i128" | "core::i128" | "std::primitive::i128" | "core::primitive::i128" => {
            Some(("__int128", false))
        }
        "std::isize" | "core::isize" | "std::primitive::isize" | "core::primitive::isize" => {
            Some(("ptrdiff_t", false))
        }
        "std::u8" | "core::u8" | "std::primitive::u8" | "core::primitive::u8" => {
            Some(("uint8_t", false))
        }
        "std::u16" | "core::u16" | "std::primitive::u16" | "core::primitive::u16" => {
            Some(("uint16_t", false))
        }
        "std::u32" | "core::u32" | "std::primitive::u32" | "core::primitive::u32" => {
            Some(("uint32_t", false))
        }
        "std::u64" | "core::u64" | "std::primitive::u64" | "core::primitive::u64" => {
            Some(("uint64_t", false))
        }
        "std::u128" | "core::u128" | "std::primitive::u128" | "core::primitive::u128" => {
            Some(("unsigned __int128", false))
        }
        "std::usize" | "core::usize" | "std::primitive::usize" | "core::primitive::usize" => {
            Some(("size_t", false))
        }
        "std::f32" | "core::f32" | "std::primitive::f32" | "core::primitive::f32" => {
            Some(("float", false))
        }
        "std::f64" | "core::f64" | "std::primitive::f64" | "core::primitive::f64" => {
            Some(("double", false))
        }

        // Smart pointers
        "Box" | "std::boxed::Box" => Some(("rusty::Box", true)),
        "Rc" | "std::rc::Rc" => Some(("rusty::Rc", true)),
        "Arc" | "std::sync::Arc" => Some(("rusty::Arc", true)),
        "Weak" | "std::rc::Weak" | "std::sync::Weak" => Some(("rusty::Weak", true)),

        // Interior mutability
        "Cell" | "std::cell::Cell" => Some(("rusty::Cell", true)),
        "RefCell" | "std::cell::RefCell" => Some(("rusty::RefCell", true)),
        "UnsafeCell" | "std::cell::UnsafeCell" => Some(("rusty::UnsafeCell", true)),

        // Collections
        "Vec" | "std::vec::Vec" => Some(("rusty::Vec", true)),
        "HashMap" | "std::collections::HashMap" => Some(("rusty::HashMap", true)),
        "HashSet" | "std::collections::HashSet" => Some(("rusty::HashSet", true)),
        "BTreeMap" | "std::collections::BTreeMap" => Some(("rusty::BTreeMap", true)),
        "BTreeSet" | "std::collections::BTreeSet" => Some(("rusty::BTreeSet", true)),
        "VecDeque" | "std::collections::VecDeque" => Some(("rusty::VecDeque", true)),
        "std::collections::hash_map::DefaultHasher" => Some(("DefaultHasher", false)),
        // Fallback aliases for collection families not yet modeled separately.
        // Keep a deterministic compile surface in expanded serde-style targets.
        "BinaryHeap" | "std::collections::BinaryHeap" => Some(("rusty::Vec", true)),
        "LinkedList" | "std::collections::LinkedList" => Some(("rusty::Vec", true)),

        // Strings
        "String" | "std::string::String" => Some(("rusty::String", false)),
        "net::TcpStream" | "std::net::TcpStream" => Some(("rusty::net::TcpStream", false)),
        "net::IpAddr" | "std::net::IpAddr" => Some(("rusty::net::IpAddr", false)),
        "net::Ipv4Addr" | "std::net::Ipv4Addr" => Some(("rusty::net::Ipv4Addr", false)),
        "net::Ipv6Addr" | "std::net::Ipv6Addr" => Some(("rusty::net::Ipv6Addr", false)),
        "net::SocketAddr" | "std::net::SocketAddr" => Some(("rusty::net::SocketAddr", false)),
        "net::SocketAddrV4" | "std::net::SocketAddrV4" => Some(("rusty::net::SocketAddrV4", false)),
        "net::SocketAddrV6" | "std::net::SocketAddrV6" => Some(("rusty::net::SocketAddrV6", false)),

        // Error handling
        "Option" | "std::option::Option" => Some(("rusty::Option", true)),
        "Result" | "std::result::Result" => Some(("rusty::Result", true)),
        "core::option::Option" => Some(("rusty::Option", true)),
        "core::result::Result" => Some(("rusty::Result", true)),

        // Concurrency
        "Mutex" | "std::sync::Mutex" => Some(("rusty::Mutex", true)),
        "RwLock" | "std::sync::RwLock" => Some(("rusty::RwLock", true)),
        "Condvar" | "std::sync::Condvar" => Some(("rusty::Condvar", false)),
        "Barrier" | "std::sync::Barrier" => Some(("rusty::Barrier", false)),
        "Once" | "std::sync::Once" => Some(("rusty::Once", false)),
        "std::thread::Thread" => Some(("rusty::thread::Thread", false)),
        "std::thread::LocalKey" => Some(("rusty::thread::LocalKey", true)),
        "std::sync::atomic::AtomicBool" | "core::sync::atomic::AtomicBool" => {
            Some(("rusty::sync::atomic::AtomicBool", false))
        }
        "std::sync::atomic::AtomicI8" | "core::sync::atomic::AtomicI8" => {
            Some(("rusty::sync::atomic::AtomicI8", false))
        }
        "std::sync::atomic::AtomicI16" | "core::sync::atomic::AtomicI16" => {
            Some(("rusty::sync::atomic::AtomicI16", false))
        }
        "std::sync::atomic::AtomicI32" | "core::sync::atomic::AtomicI32" => {
            Some(("rusty::sync::atomic::AtomicI32", false))
        }
        "std::sync::atomic::AtomicI64" | "core::sync::atomic::AtomicI64" => {
            Some(("rusty::sync::atomic::AtomicI64", false))
        }
        "std::sync::atomic::AtomicIsize" | "core::sync::atomic::AtomicIsize" => {
            Some(("rusty::sync::atomic::AtomicIsize", false))
        }
        "std::sync::atomic::AtomicU8" | "core::sync::atomic::AtomicU8" => {
            Some(("rusty::sync::atomic::AtomicU8", false))
        }
        "std::sync::atomic::AtomicU16" | "core::sync::atomic::AtomicU16" => {
            Some(("rusty::sync::atomic::AtomicU16", false))
        }
        "std::sync::atomic::AtomicU32" | "core::sync::atomic::AtomicU32" => {
            Some(("rusty::sync::atomic::AtomicU32", false))
        }
        "std::sync::atomic::AtomicU64" | "core::sync::atomic::AtomicU64" => {
            Some(("rusty::sync::atomic::AtomicU64", false))
        }
        "std::sync::atomic::AtomicUsize" | "core::sync::atomic::AtomicUsize" => {
            Some(("rusty::sync::atomic::AtomicUsize", false))
        }
        "std::sync::atomic::AtomicPtr" | "core::sync::atomic::AtomicPtr" => {
            Some(("rusty::sync::atomic::AtomicPtr", true))
        }
        "AtomicBool" | "atomic::AtomicBool" => Some(("rusty::sync::atomic::AtomicBool", false)),
        "AtomicI8" | "atomic::AtomicI8" => Some(("rusty::sync::atomic::AtomicI8", false)),
        "AtomicI16" | "atomic::AtomicI16" => Some(("rusty::sync::atomic::AtomicI16", false)),
        "AtomicI32" | "atomic::AtomicI32" => Some(("rusty::sync::atomic::AtomicI32", false)),
        "AtomicI64" | "atomic::AtomicI64" => Some(("rusty::sync::atomic::AtomicI64", false)),
        "AtomicIsize" | "atomic::AtomicIsize" => Some(("rusty::sync::atomic::AtomicIsize", false)),
        "AtomicU8" | "atomic::AtomicU8" => Some(("rusty::sync::atomic::AtomicU8", false)),
        "AtomicU16" | "atomic::AtomicU16" => Some(("rusty::sync::atomic::AtomicU16", false)),
        "AtomicU32" | "atomic::AtomicU32" => Some(("rusty::sync::atomic::AtomicU32", false)),
        "AtomicU64" | "atomic::AtomicU64" => Some(("rusty::sync::atomic::AtomicU64", false)),
        "AtomicUsize" | "atomic::AtomicUsize" => Some(("rusty::sync::atomic::AtomicUsize", false)),
        "AtomicPtr" | "atomic::AtomicPtr" => Some(("rusty::sync::atomic::AtomicPtr", true)),
        "std::sync::atomic::Ordering" | "core::sync::atomic::Ordering" => {
            Some(("rusty::sync::atomic::Ordering", false))
        }
        "core::task::Poll" | "std::task::Poll" => Some(("rusty::Poll", true)),
        "core::task::Context" | "std::task::Context" => Some(("rusty::Context", false)),
        "core::task::Waker" | "std::task::Waker" => Some(("rusty::Waker", false)),

        // Runtime compatibility fallbacks for expanded Rust paths.
        "core::cmp::Ordering" => Some(("rusty::cmp::Ordering", false)),
        "std::cmp::Ordering" => Some(("rusty::cmp::Ordering", false)),
        "Any" | "std::any::Any" | "core::any::Any" => Some(("std::any", false)),
        "TypeId" | "std::any::TypeId" | "core::any::TypeId" => Some(("std::type_index", false)),
        "slice::Iter" | "core::slice::Iter" | "std::slice::Iter" => {
            Some(("rusty::slice_iter::Iter", true))
        }
        "slice::IterMut" | "core::slice::IterMut" | "std::slice::IterMut" => {
            Some(("rusty::slice_iter::Iter", true))
        }
        "iter::Empty" | "core::iter::Empty" | "std::iter::Empty" => {
            Some(("rusty::empty_iter", true))
        }
        "iter::Once" | "core::iter::Once" | "std::iter::Once" => Some(("rusty::once_iter", true)),
        "ops::Range" | "core::ops::Range" | "std::ops::Range" => Some(("rusty::range", true)),
        "ops::RangeInclusive" | "core::ops::RangeInclusive" | "std::ops::RangeInclusive" => {
            Some(("rusty::range_inclusive", true))
        }
        "ops::RangeFrom" | "core::ops::RangeFrom" | "std::ops::RangeFrom" => {
            Some(("rusty::range_from", true))
        }
        "ops::RangeTo" | "core::ops::RangeTo" | "std::ops::RangeTo" => {
            Some(("rusty::range_to", true))
        }
        "ops::RangeToInclusive" | "core::ops::RangeToInclusive" | "std::ops::RangeToInclusive" => {
            Some(("rusty::range_to_inclusive", true))
        }
        "ops::RangeFull" | "core::ops::RangeFull" | "std::ops::RangeFull" => {
            Some(("rusty::range_full", false))
        }
        "core::fmt::Result" | "fmt::Result" => Some(("rusty::fmt::Result", false)),
        "core::fmt::Formatter" | "fmt::Formatter" => Some(("rusty::fmt::Formatter", false)),
        "core::fmt::Arguments" | "fmt::Arguments" => Some(("rusty::fmt::Arguments", false)),
        "core::fmt::Alignment" | "fmt::Alignment" => Some(("rusty::fmt::Alignment", false)),
        "core::fmt::Error" | "std::fmt::Error" | "fmt::Error" => Some(("rusty::fmt::Error", false)),
        "NonNull" | "std::ptr::NonNull" | "core::ptr::NonNull" => {
            Some(("rusty::ptr::NonNull", true))
        }
        "NonZeroUsize"
        | "num::NonZeroUsize"
        | "std::num::NonZeroUsize"
        | "core::num::NonZeroUsize" => Some(("rusty::num::NonZeroUsize", false)),
        "NonZeroU64" | "num::NonZeroU64" | "std::num::NonZeroU64" | "core::num::NonZeroU64" => {
            Some(("rusty::num::NonZeroU64", false))
        }
        "NonZeroI8" | "num::NonZeroI8" | "std::num::NonZeroI8" | "core::num::NonZeroI8" => {
            Some(("rusty::num::NonZeroI8", false))
        }
        "NonZeroI16" | "num::NonZeroI16" | "std::num::NonZeroI16" | "core::num::NonZeroI16" => {
            Some(("rusty::num::NonZeroI16", false))
        }
        "NonZeroI32" | "num::NonZeroI32" | "std::num::NonZeroI32" | "core::num::NonZeroI32" => {
            Some(("rusty::num::NonZeroI32", false))
        }
        "NonZeroI64" | "num::NonZeroI64" | "std::num::NonZeroI64" | "core::num::NonZeroI64" => {
            Some(("rusty::num::NonZeroI64", false))
        }
        "NonZeroI128" | "num::NonZeroI128" | "std::num::NonZeroI128" | "core::num::NonZeroI128" => {
            Some(("rusty::num::NonZeroI128", false))
        }
        "NonZeroIsize"
        | "num::NonZeroIsize"
        | "std::num::NonZeroIsize"
        | "core::num::NonZeroIsize" => Some(("rusty::num::NonZeroIsize", false)),
        "NonZeroU8" | "num::NonZeroU8" | "std::num::NonZeroU8" | "core::num::NonZeroU8" => {
            Some(("rusty::num::NonZeroU8", false))
        }
        "NonZeroU16" | "num::NonZeroU16" | "std::num::NonZeroU16" | "core::num::NonZeroU16" => {
            Some(("rusty::num::NonZeroU16", false))
        }
        "NonZeroU32" | "num::NonZeroU32" | "std::num::NonZeroU32" | "core::num::NonZeroU32" => {
            Some(("rusty::num::NonZeroU32", false))
        }
        "NonZeroU128" | "num::NonZeroU128" | "std::num::NonZeroU128" | "core::num::NonZeroU128" => {
            Some(("rusty::num::NonZeroU128", false))
        }
        "Wrapping" | "num::Wrapping" | "std::num::Wrapping" | "core::num::Wrapping" => {
            Some(("rusty::num::Wrapping", true))
        }
        "Saturating" | "num::Saturating" | "std::num::Saturating" | "core::num::Saturating" => {
            Some(("rusty::num::Saturating", true))
        }
        "Reverse" | "cmp::Reverse" | "std::cmp::Reverse" | "core::cmp::Reverse" => {
            Some(("rusty::cmp::Reverse", true))
        }
        "std::alloc::Layout" | "core::alloc::Layout" | "alloc::alloc::Layout" => {
            Some(("rusty::alloc::Layout", false))
        }
        "std::alloc::LayoutErr"
        | "core::alloc::LayoutErr"
        | "std::alloc::LayoutError"
        | "core::alloc::LayoutError"
        | "alloc::alloc::LayoutErr"
        | "alloc::alloc::LayoutError" => Some(("rusty::alloc::LayoutErr", false)),
        "std::str::Utf8Error" | "core::str::Utf8Error" => {
            Some(("rusty::str_runtime::Utf8Error", false))
        }
        "PhantomData" | "std::marker::PhantomData" | "core::marker::PhantomData" => {
            Some(("rusty::PhantomData", true))
        }
        "Pin" | "std::pin::Pin" | "core::pin::Pin" => Some(("rusty::pin::Pin", true)),
        "std::future::Ready" | "core::future::Ready" => Some(("rusty::future::Ready", true)),
        "std::time::Instant" => Some(("rusty::time::Instant", false)),
        "Duration" | "std::time::Duration" | "core::time::Duration" => {
            Some(("rusty::time::Duration", false))
        }
        "SystemTime" | "std::time::SystemTime" | "core::time::SystemTime" => {
            Some(("rusty::time::SystemTime", false))
        }
        "Path" | "std::path::Path" => Some(("rusty::path::Path", false)),
        "PathBuf" | "std::path::PathBuf" => Some(("rusty::path::PathBuf", false)),
        "Cow" | "std::borrow::Cow" | "core::borrow::Cow" | "alloc::borrow::Cow" => {
            Some(("rusty::Cow", false))
        }
        "OsStr" | "std::ffi::OsStr" => Some(("rusty::ffi::OsStr", false)),
        "CStr" | "std::ffi::CStr" => Some(("rusty::ffi::CStr", false)),
        "CString" | "std::ffi::CString" => Some(("rusty::ffi::CString", false)),
        "OsString" | "std::ffi::OsString" => Some(("rusty::ffi::OsString", false)),
        "std::process::Child" => Some(("rusty::process::Child", false)),
        "std::process::Command" => Some(("rusty::process::Command", false)),

        // MaybeUninit
        "MaybeUninit" | "std::mem::MaybeUninit" => Some(("rusty::MaybeUninit", true)),
        "ManuallyDrop" | "std::mem::ManuallyDrop" => Some(("rusty::mem::ManuallyDrop", true)),

        // I/O types
        "io::Result" | "std::io::Result" => Some(("rusty::io::Result", true)),
        "io::Cursor" | "std::io::Cursor" => Some(("rusty::io::Cursor", true)),
        "io::Error" | "std::io::Error" => Some(("rusty::io::Error", false)),
        "io::SeekFrom" | "std::io::SeekFrom" => Some(("rusty::io::SeekFrom", false)),
        "io::Stdin" | "std::io::Stdin" => Some(("rusty::io::Stdin", false)),
        "io::Stdout" | "std::io::Stdout" => Some(("rusty::io::Stdout", false)),
        "io::Stderr" | "std::io::Stderr" => Some(("rusty::io::Stderr", false)),

        // str (bare type, not &str — &str handled at Type::Reference level)
        "str" | "core::primitive::str" | "std::primitive::str" => {
            Some(("std::string_view", false))
        }

        _ => None,
    }
}

/// Map Rust method/function paths that need renaming in C++.
/// Returns the C++ replacement if the path should be rewritten.
pub fn map_function_path(rust_path: &str) -> Option<&'static str> {
    match rust_path {
        // Box::new → rusty::Box<T>::new_ (new is a C++ keyword, _ suffix for consistency)
        "Box::new" => Some("rusty::Box::new_"),
        "alloc::boxed::box_new" | "std::boxed::box_new" | "boxed::box_new" => {
            Some("rusty::boxed::box_new")
        }
        "into_vec" => Some("rusty::boxed::into_vec"),
        // String::from → rusty::String constructor
        "String::from" => Some("rusty::String::from"),
        "String::new" => Some("rusty::String::new_"),
        "String::from_utf8"
        | "std::string::String::from_utf8"
        | "alloc::string::String::from_utf8" => Some("rusty::String::from_utf8"),
        "String::from_utf8_lossy"
        | "std::string::String::from_utf8_lossy"
        | "alloc::string::String::from_utf8_lossy" => Some("rusty::String::from_utf8_lossy"),
        "CString::new" | "CString::new_" | "std::ffi::CString::new" | "std::ffi::CString::new_" => {
            Some("rusty::ffi::cstring_new")
        }
        // Vec::new
        "Vec::new" | "std::vec::Vec::new" | "alloc::vec::Vec::new" => Some("rusty::Vec::new_"),
        "vec::from_elem" | "std::vec::from_elem" | "alloc::vec::from_elem" => {
            Some("rusty::array_repeat")
        }
        "repeat" | "iter::repeat" | "core::iter::repeat" | "std::iter::repeat" => {
            Some("rusty::repeat")
        }
        "iter::once" | "core::iter::once" | "std::iter::once" => Some("rusty::once"),
        "iter::empty" | "core::iter::empty" | "std::iter::empty" => Some("rusty::empty"),
        "iter::repeat_with" | "core::iter::repeat_with" | "std::iter::repeat_with" => {
            Some("rusty::repeat_with")
        }
        "std::future::ready" | "core::future::ready" | "future::ready" => {
            Some("rusty::future::ready")
        }
        "futures_timer::Delay::new" | "futures_timer::Delay::new_" => {
            Some("rusty::future::Delay::new_")
        }
        "Vec::with_capacity" => Some("rusty::Vec::with_capacity"),
        "DefaultHasher::new"
        | "DefaultHasher::new_"
        | "std::collections::hash_map::DefaultHasher::new"
        | "std::collections::hash_map::DefaultHasher::new_" => Some("DefaultHasher::new_"),
        "DefaultHasher::default" | "std::collections::hash_map::DefaultHasher::default" => {
            Some("DefaultHasher::new_")
        }
        "Vec::extend_from_slice"
        | "std::vec::Vec::extend_from_slice"
        | "alloc::vec::Vec::extend_from_slice" => Some("rusty::vec_extend_from_slice"),
        // thread::spawn
        "thread::spawn" | "std::thread::spawn" => Some("rusty::thread::spawn"),
        "thread::current" | "std::thread::current" => Some("rusty::thread::current"),
        "thread::park" | "std::thread::park" => Some("rusty::thread::park"),
        "thread::yield_now" | "std::thread::yield_now" => Some("rusty::thread::yield_now"),
        "std::sync::atomic::fence" | "core::sync::atomic::fence" => {
            Some("rusty::sync::atomic::fence")
        }
        // I/O functions
        "io::stdin" | "std::io::stdin" => Some("rusty::io::stdin_"),
        "io::stdout" | "std::io::stdout" => Some("rusty::io::stdout_"),
        "io::stderr" | "std::io::stderr" => Some("rusty::io::stderr_"),
        "io::_print" | "std::io::_print" => Some("rusty::io::_print"),
        "io::_eprint" | "std::io::_eprint" => Some("rusty::io::_eprint"),
        "io::Cursor::new" | "std::io::Cursor::new" => Some("rusty::io::Cursor::new_"),

        // Expanded-Rust runtime compatibility shims.
        "core::intrinsics::discriminant_value" => Some("rusty::intrinsics::discriminant_value"),
        "core::intrinsics::unreachable" => Some("rusty::intrinsics::unreachable"),
        "core::panicking::panic_fmt" => Some("rusty::panicking::panic_fmt"),
        "core::panicking::assert_failed" => Some("rusty::panicking::assert_failed"),
        "core::panicking::unreachable_display" => Some("rusty::panicking::unreachable_display"),
        "std::ptr::null_mut" | "core::ptr::null_mut" | "ptr::null_mut" => {
            Some("rusty::ptr::null_mut")
        }
        "std::ptr::read" | "ptr::read" => Some("rusty::ptr::read"),
        "std::ptr::write" | "ptr::write" => Some("rusty::ptr::write"),
        "core::ptr::mut_ptr::add" | "std::ptr::mut_ptr::add" | "ptr::mut_ptr::add" => {
            Some("rusty::ptr::add")
        }
        "core::ptr::const_ptr::add" | "std::ptr::const_ptr::add" | "ptr::const_ptr::add" => {
            Some("rusty::ptr::add")
        }
        "core::ptr::mut_ptr::offset" | "std::ptr::mut_ptr::offset" | "ptr::mut_ptr::offset" => {
            Some("rusty::ptr::offset")
        }
        "core::ptr::mut_ptr::wrapping_offset"
        | "std::ptr::mut_ptr::wrapping_offset"
        | "ptr::mut_ptr::wrapping_offset" => Some("rusty::ptr::offset"),
        "core::ptr::const_ptr::offset"
        | "std::ptr::const_ptr::offset"
        | "ptr::const_ptr::offset" => Some("rusty::ptr::offset"),
        "core::ptr::const_ptr::wrapping_offset"
        | "std::ptr::const_ptr::wrapping_offset"
        | "ptr::const_ptr::wrapping_offset" => Some("rusty::ptr::offset"),
        "std::ptr::copy" | "ptr::copy" => Some("rusty::ptr::copy"),
        "std::ptr::copy_nonoverlapping" | "ptr::copy_nonoverlapping" => {
            Some("rusty::ptr::copy_nonoverlapping")
        }
        "std::ptr::drop_in_place" | "ptr::drop_in_place" => Some("rusty::ptr::drop_in_place"),
        "std::slice::from_raw_parts" | "core::slice::from_raw_parts" | "slice::from_raw_parts" => {
            Some("rusty::from_raw_parts")
        }
        "std::slice::from_raw_parts_mut"
        | "core::slice::from_raw_parts_mut"
        | "slice::from_raw_parts_mut" => Some("rusty::from_raw_parts_mut"),
        "drop" | "std::mem::drop" | "mem::drop" => Some("rusty::mem::drop"),
        "std::mem::size_of" | "mem::size_of" => Some("rusty::mem::size_of"),
        "std::mem::align_of" | "core::mem::align_of" | "mem::align_of" => {
            Some("rusty::mem::align_of")
        }
        "std::mem::replace" | "mem::replace" => Some("rusty::mem::replace"),
        "std::mem::swap" | "core::mem::swap" | "mem::swap" => Some("rusty::mem::swap"),
        "std::mem::forget" | "mem::forget" => Some("rusty::mem::forget"),
        "usize::checked_next_power_of_two"
        | "std::usize::checked_next_power_of_two"
        | "core::usize::checked_next_power_of_two" => {
            Some("rusty::checked_next_power_of_two_usize")
        }
        "unreachable_unchecked"
        | "hint::unreachable_unchecked"
        | "std::hint::unreachable_unchecked"
        | "core::hint::unreachable_unchecked" => Some("rusty::intrinsics::unreachable"),
        "alloc::alloc" | "std::alloc::alloc" | "core::alloc::alloc" => Some("rusty::alloc::alloc"),
        "alloc::dealloc" | "std::alloc::dealloc" | "core::alloc::dealloc" => {
            Some("rusty::alloc::dealloc")
        }
        "alloc::handle_alloc_error"
        | "std::alloc::handle_alloc_error"
        | "core::alloc::handle_alloc_error" => Some("rusty::alloc::handle_alloc_error"),
        "alloc::fmt::format" | "std::alloc::fmt::format" | "core::alloc::fmt::format" => {
            Some("rusty::alloc::fmt::format")
        }
        "alloc::__export::must_use"
        | "std::alloc::__export::must_use"
        | "core::alloc::__export::must_use" => Some("rusty::alloc::__export::must_use"),
        "ManuallyDrop::new" | "std::mem::ManuallyDrop::new" | "mem::ManuallyDrop::new" => {
            Some("rusty::mem::manually_drop_new")
        }
        "std::panic::catch_unwind" | "panic::catch_unwind" => Some("rusty::panic::catch_unwind"),
        "std::panic::resume_unwind" | "panic::resume_unwind" => Some("rusty::panic::resume_unwind"),
        "std::panic::AssertUnwindSafe" | "panic::AssertUnwindSafe" => {
            Some("rusty::panic::AssertUnwindSafe")
        }
        "std::rt::begin_panic" | "rt::begin_panic" => Some("rusty::panic::begin_panic"),
        "std::rt::panic_fmt" | "rt::panic_fmt" => Some("rusty::panicking::panic_fmt"),
        "std::process::abort" => Some("std::abort"),
        "core::hash::Hash::hash" => Some("rusty::hash::hash"),
        "Add::add" | "core::ops::Add::add" | "std::ops::Add::add" => Some("rusty::ops::add_fn"),
        "cmp::min" | "core::cmp::min" | "std::cmp::min" => Some("rusty::cmp::min"),
        "cmp::max" | "core::cmp::max" | "std::cmp::max" => Some("rusty::cmp::max"),
        "std::str::from_utf8" | "core::str::from_utf8" | "str::from_utf8" => {
            Some("rusty::str_runtime::from_utf8")
        }
        "std::string_view::from_utf8" => Some("rusty::str_runtime::from_utf8"),
        "std::str::from_utf8_unchecked"
        | "core::str::from_utf8_unchecked"
        | "str::from_utf8_unchecked"
        | "std::string_view::from_utf8_unchecked" => {
            Some("rusty::str_runtime::from_utf8_unchecked")
        }
        "std::str::from_utf8_unchecked_mut"
        | "core::str::from_utf8_unchecked_mut"
        | "str::from_utf8_unchecked_mut"
        | "std::string_view::from_utf8_unchecked_mut" => {
            Some("rusty::str_runtime::from_utf8_unchecked_mut")
        }
        "std::char::from_u32" | "core::char::from_u32" | "char::from_u32" => {
            Some("rusty::char_runtime::from_u32")
        }
        "core::fmt::Formatter::debug_tuple_field1_finish" => {
            Some("rusty::fmt::Formatter::debug_tuple_field1_finish")
        }
        "core::fmt::Formatter::debug_struct_field1_finish" => {
            Some("rusty::fmt::Formatter::debug_struct_field1_finish")
        }
        "Pin::new_unchecked" | "std::pin::Pin::new_unchecked" | "core::pin::Pin::new_unchecked" => {
            Some("rusty::pin::new_unchecked")
        }
        "Pin::get_ref" | "std::pin::Pin::get_ref" | "core::pin::Pin::get_ref" => {
            Some("rusty::pin::get_ref")
        }
        "Pin::get_unchecked_mut"
        | "std::pin::Pin::get_unchecked_mut"
        | "core::pin::Pin::get_unchecked_mut" => Some("rusty::pin::get_unchecked_mut"),
        _ => None,
    }
}

/// User-provided type mappings loaded from a TOML file.
/// Format:
/// ```toml
/// # Map crate::Type to C++ type
/// [serde]
/// Serialize = "serde::Serialize"
/// Deserialize = "serde::Deserialize"
///
/// [tokio.runtime]
/// Runtime = "tokio::Runtime"
/// ```
///
/// Each section is a crate/module name, and each key-value maps a Rust type to a C++ type.
/// The Rust path "crate::Type" maps to the value string.
#[derive(Default, Clone)]
pub struct UserTypeMap {
    /// Maps Rust type path (e.g., "serde::Serialize") to C++ type string.
    pub mappings: std::collections::HashMap<String, String>,
}

impl UserTypeMap {
    /// Load type mappings from a TOML file.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read type map file '{}': {}", path.display(), e))?;

        let table: toml::value::Table = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse type map file: {}", e))?;

        let mut mappings = std::collections::HashMap::new();
        Self::flatten_table("", &table, &mut mappings);
        Ok(Self { mappings })
    }

    /// Recursively flatten nested TOML tables into dotted paths.
    /// [serde] Serialize = "..." → "serde::Serialize" = "..."
    fn flatten_table(
        prefix: &str,
        table: &toml::value::Table,
        result: &mut std::collections::HashMap<String, String>,
    ) {
        for (key, value) in table {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}::{}", prefix, key)
            };

            match value {
                toml::Value::String(s) => {
                    result.insert(path, s.clone());
                }
                toml::Value::Table(t) => {
                    Self::flatten_table(&path, t, result);
                }
                _ => {}
            }
        }
    }

    /// Look up a type path in the user mappings.
    pub fn lookup(&self, rust_path: &str) -> Option<&str> {
        self.mappings.get(rust_path).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(map_primitive_type("i32"), Some("int32_t"));
        assert_eq!(map_primitive_type("u64"), Some("uint64_t"));
        assert_eq!(map_primitive_type("f64"), Some("double"));
        assert_eq!(map_primitive_type("bool"), Some("bool"));
        assert_eq!(map_primitive_type("char"), Some("char32_t"));
        assert_eq!(map_primitive_type("usize"), Some("size_t"));
        assert_eq!(map_primitive_type("isize"), Some("ptrdiff_t"));
        assert_eq!(map_primitive_type("unknown"), None);
    }

    #[test]
    fn test_std_types() {
        assert_eq!(map_std_type("Vec"), Some(("rusty::Vec", true)));
        assert_eq!(map_std_type("Box"), Some(("rusty::Box", true)));
        assert_eq!(map_std_type("String"), Some(("rusty::String", false)));
        assert_eq!(map_std_type("Option"), Some(("rusty::Option", true)));
        assert_eq!(map_std_type("Result"), Some(("rusty::Result", true)));
        assert_eq!(
            map_std_type("io::Result"),
            Some(("rusty::io::Result", true))
        );
        assert_eq!(
            map_std_type("slice::Iter"),
            Some(("rusty::slice_iter::Iter", true))
        );
        assert_eq!(
            map_std_type("std::marker::PhantomData"),
            Some(("rusty::PhantomData", true))
        );
        assert_eq!(map_std_type("HashMap"), Some(("rusty::HashMap", true)));
        assert_eq!(map_std_type("Mutex"), Some(("rusty::Mutex", true)));
        assert_eq!(map_std_type("UnknownType"), None);
    }

    #[test]
    fn test_leaf42_runtime_type_fallback_mappings() {
        assert_eq!(
            map_std_type("core::option::Option"),
            Some(("rusty::Option", true))
        );
        assert_eq!(
            map_std_type("std::cmp::Ordering"),
            Some(("rusty::cmp::Ordering", false))
        );
        assert_eq!(
            map_std_type("core::task::Poll"),
            Some(("rusty::Poll", true))
        );
        assert_eq!(map_std_type("std::task::Poll"), Some(("rusty::Poll", true)));
        assert_eq!(
            map_std_type("std::task::Context"),
            Some(("rusty::Context", false))
        );
        assert_eq!(
            map_std_type("std::task::Waker"),
            Some(("rusty::Waker", false))
        );
        assert_eq!(
            map_std_type("core::slice::Iter"),
            Some(("rusty::slice_iter::Iter", true))
        );
        assert_eq!(
            map_std_type("core::fmt::Formatter"),
            Some(("rusty::fmt::Formatter", false))
        );
        assert_eq!(map_std_type("std::any::Any"), Some(("std::any", false)));
        assert_eq!(
            map_std_type("core::any::TypeId"),
            Some(("std::type_index", false))
        );
        assert_eq!(
            map_std_type("fmt::Arguments"),
            Some(("rusty::fmt::Arguments", false))
        );
        assert_eq!(
            map_std_type("fmt::Error"),
            Some(("rusty::fmt::Error", false))
        );
        assert_eq!(map_std_type("Pin"), Some(("rusty::pin::Pin", true)));
        assert_eq!(
            map_std_type("std::path::Path"),
            Some(("rusty::path::Path", false))
        );
        assert_eq!(
            map_std_type("std::ffi::CStr"),
            Some(("rusty::ffi::CStr", false))
        );
        assert_eq!(
            map_std_type("std::time::Instant"),
            Some(("rusty::time::Instant", false))
        );
        assert_eq!(
            map_std_type("std::future::Ready"),
            Some(("rusty::future::Ready", true))
        );
        assert_eq!(
            map_std_type("std::mem::ManuallyDrop"),
            Some(("rusty::mem::ManuallyDrop", true))
        );
        assert_eq!(
            map_std_type("core::fmt::Alignment"),
            Some(("rusty::fmt::Alignment", false))
        );
        assert_eq!(
            map_std_type("std::ptr::NonNull"),
            Some(("rusty::ptr::NonNull", true))
        );
        assert_eq!(
            map_std_type("core::num::NonZeroUsize"),
            Some(("rusty::num::NonZeroUsize", false))
        );
        assert_eq!(
            map_std_type("std::num::NonZeroU64"),
            Some(("rusty::num::NonZeroU64", false))
        );
        assert_eq!(
            map_std_type("core::alloc::Layout"),
            Some(("rusty::alloc::Layout", false))
        );
        assert_eq!(
            map_std_type("std::alloc::LayoutErr"),
            Some(("rusty::alloc::LayoutErr", false))
        );
        assert_eq!(
            map_std_type("core::alloc::LayoutError"),
            Some(("rusty::alloc::LayoutErr", false))
        );
    }

    #[test]
    fn test_std_types_full_path() {
        assert_eq!(map_std_type("std::vec::Vec"), Some(("rusty::Vec", true)));
        assert_eq!(map_std_type("std::sync::Arc"), Some(("rusty::Arc", true)));
        assert_eq!(
            map_std_type("std::io::Result"),
            Some(("rusty::io::Result", true))
        );
        assert_eq!(
            map_std_type("std::slice::Iter"),
            Some(("rusty::slice_iter::Iter", true))
        );
        assert_eq!(
            map_std_type("std::collections::HashMap"),
            Some(("rusty::HashMap", true))
        );
        assert_eq!(
            map_std_type("std::net::TcpStream"),
            Some(("rusty::net::TcpStream", false))
        );
        assert_eq!(
            map_std_type("std::collections::hash_map::DefaultHasher"),
            Some(("DefaultHasher", false))
        );
    }

    #[test]
    fn test_smart_pointers() {
        assert_eq!(map_std_type("Box"), Some(("rusty::Box", true)));
        assert_eq!(map_std_type("Rc"), Some(("rusty::Rc", true)));
        assert_eq!(map_std_type("Arc"), Some(("rusty::Arc", true)));
        assert_eq!(map_std_type("Weak"), Some(("rusty::Weak", true)));
    }

    #[test]
    fn test_interior_mutability() {
        assert_eq!(map_std_type("Cell"), Some(("rusty::Cell", true)));
        assert_eq!(map_std_type("RefCell"), Some(("rusty::RefCell", true)));
        assert_eq!(
            map_std_type("UnsafeCell"),
            Some(("rusty::UnsafeCell", true))
        );
    }

    #[test]
    fn test_collections() {
        assert_eq!(map_std_type("Vec"), Some(("rusty::Vec", true)));
        assert_eq!(map_std_type("HashMap"), Some(("rusty::HashMap", true)));
        assert_eq!(map_std_type("HashSet"), Some(("rusty::HashSet", true)));
        assert_eq!(map_std_type("BTreeMap"), Some(("rusty::BTreeMap", true)));
        assert_eq!(map_std_type("BTreeSet"), Some(("rusty::BTreeSet", true)));
        assert_eq!(map_std_type("VecDeque"), Some(("rusty::VecDeque", true)));
    }

    #[test]
    fn test_concurrency() {
        assert_eq!(map_std_type("Mutex"), Some(("rusty::Mutex", true)));
        assert_eq!(map_std_type("RwLock"), Some(("rusty::RwLock", true)));
        assert_eq!(map_std_type("Condvar"), Some(("rusty::Condvar", false)));
        assert_eq!(map_std_type("Barrier"), Some(("rusty::Barrier", false)));
        assert_eq!(map_std_type("Once"), Some(("rusty::Once", false)));
        assert_eq!(
            map_std_type("std::sync::atomic::AtomicPtr"),
            Some(("rusty::sync::atomic::AtomicPtr", true))
        );
        assert_eq!(
            map_std_type("AtomicPtr"),
            Some(("rusty::sync::atomic::AtomicPtr", true))
        );
        assert_eq!(
            map_std_type("std::sync::atomic::AtomicUsize"),
            Some(("rusty::sync::atomic::AtomicUsize", false))
        );
        assert_eq!(
            map_std_type("AtomicUsize"),
            Some(("rusty::sync::atomic::AtomicUsize", false))
        );
        assert_eq!(
            map_std_type("std::sync::atomic::Ordering"),
            Some(("rusty::sync::atomic::Ordering", false))
        );
        assert_eq!(
            map_std_type("std::thread::Thread"),
            Some(("rusty::thread::Thread", false))
        );
    }

    #[test]
    fn test_str_type() {
        assert_eq!(map_std_type("str"), Some(("std::string_view", false)));
        assert_eq!(
            map_std_type("std::str::Utf8Error"),
            Some(("rusty::str_runtime::Utf8Error", false))
        );
    }

    #[test]
    fn test_function_path_mapping() {
        assert_eq!(map_function_path("Box::new"), Some("rusty::Box::new_"));
        assert_eq!(
            map_function_path("String::from"),
            Some("rusty::String::from")
        );
        assert_eq!(
            map_function_path("String::new"),
            Some("rusty::String::new_")
        );
        assert_eq!(
            map_function_path("String::from_utf8"),
            Some("rusty::String::from_utf8")
        );
        assert_eq!(
            map_function_path("alloc::string::String::from_utf8"),
            Some("rusty::String::from_utf8")
        );
        assert_eq!(
            map_function_path("String::from_utf8_lossy"),
            Some("rusty::String::from_utf8_lossy")
        );
        assert_eq!(map_function_path("Vec::new"), Some("rusty::Vec::new_"));
        assert_eq!(
            map_function_path("alloc::vec::from_elem"),
            Some("rusty::array_repeat")
        );
        assert_eq!(
            map_function_path("std::vec::Vec::new"),
            Some("rusty::Vec::new_")
        );
        assert_eq!(
            map_function_path("alloc::vec::Vec::new"),
            Some("rusty::Vec::new_")
        );
        assert_eq!(
            map_function_path("Vec::extend_from_slice"),
            Some("rusty::vec_extend_from_slice")
        );
        assert_eq!(
            map_function_path("thread::spawn"),
            Some("rusty::thread::spawn")
        );
        assert_eq!(
            map_function_path("std::thread::current"),
            Some("rusty::thread::current")
        );
        assert_eq!(
            map_function_path("std::thread::park"),
            Some("rusty::thread::park")
        );
        assert_eq!(
            map_function_path("std::sync::atomic::fence"),
            Some("rusty::sync::atomic::fence")
        );
        assert_eq!(
            map_function_path("DefaultHasher::new"),
            Some("DefaultHasher::new_")
        );
        assert_eq!(
            map_function_path("std::collections::hash_map::DefaultHasher::new"),
            Some("DefaultHasher::new_")
        );
        assert_eq!(
            map_function_path("std::collections::hash_map::DefaultHasher::default"),
            Some("DefaultHasher::new_")
        );
        assert_eq!(map_function_path("Unknown::method"), None);
    }

    #[test]
    fn test_leaf42_runtime_function_path_mappings() {
        assert_eq!(
            map_function_path("core::intrinsics::discriminant_value"),
            Some("rusty::intrinsics::discriminant_value")
        );
        assert_eq!(
            map_function_path("core::panicking::panic_fmt"),
            Some("rusty::panicking::panic_fmt")
        );
        assert_eq!(
            map_function_path("core::panicking::assert_failed"),
            Some("rusty::panicking::assert_failed")
        );
        assert_eq!(
            map_function_path("core::panicking::unreachable_display"),
            Some("rusty::panicking::unreachable_display")
        );
        assert_eq!(
            map_function_path("core::hash::Hash::hash"),
            Some("rusty::hash::hash")
        );
        assert_eq!(map_function_path("std::cmp::min"), Some("rusty::cmp::min"));
        assert_eq!(map_function_path("core::cmp::max"), Some("rusty::cmp::max"));
        assert_eq!(map_function_path("cmp::min"), Some("rusty::cmp::min"));
        assert_eq!(map_function_path("cmp::max"), Some("rusty::cmp::max"));
        assert_eq!(
            map_function_path("std::str::from_utf8"),
            Some("rusty::str_runtime::from_utf8")
        );
        assert_eq!(
            map_function_path("core::str::from_utf8"),
            Some("rusty::str_runtime::from_utf8")
        );
        assert_eq!(
            map_function_path("str::from_utf8_unchecked"),
            Some("rusty::str_runtime::from_utf8_unchecked")
        );
        assert_eq!(
            map_function_path("core::str::from_utf8_unchecked_mut"),
            Some("rusty::str_runtime::from_utf8_unchecked_mut")
        );
        assert_eq!(
            map_function_path("std::char::from_u32"),
            Some("rusty::char_runtime::from_u32")
        );
        assert_eq!(
            map_function_path("Pin::new_unchecked"),
            Some("rusty::pin::new_unchecked")
        );
        assert_eq!(
            map_function_path("Pin::get_unchecked_mut"),
            Some("rusty::pin::get_unchecked_mut")
        );
        assert_eq!(
            map_function_path("std::panic::catch_unwind"),
            Some("rusty::panic::catch_unwind")
        );
        assert_eq!(
            map_function_path("std::ptr::read"),
            Some("rusty::ptr::read")
        );
        assert_eq!(
            map_function_path("core::ptr::null_mut"),
            Some("rusty::ptr::null_mut")
        );
        assert_eq!(
            map_function_path("ptr::null_mut"),
            Some("rusty::ptr::null_mut")
        );
        assert_eq!(map_function_path("ptr::write"), Some("rusty::ptr::write"));
        assert_eq!(
            map_function_path("core::ptr::mut_ptr::add"),
            Some("rusty::ptr::add")
        );
        assert_eq!(
            map_function_path("std::ptr::const_ptr::add"),
            Some("rusty::ptr::add")
        );
        assert_eq!(
            map_function_path("ptr::mut_ptr::offset"),
            Some("rusty::ptr::offset")
        );
        assert_eq!(
            map_function_path("core::ptr::const_ptr::offset"),
            Some("rusty::ptr::offset")
        );
        assert_eq!(
            map_function_path("std::ptr::mut_ptr::wrapping_offset"),
            Some("rusty::ptr::offset")
        );
        assert_eq!(
            map_function_path("ptr::const_ptr::wrapping_offset"),
            Some("rusty::ptr::offset")
        );
        assert_eq!(map_function_path("ptr::copy"), Some("rusty::ptr::copy"));
        assert_eq!(
            map_function_path("std::ptr::copy_nonoverlapping"),
            Some("rusty::ptr::copy_nonoverlapping")
        );
        assert_eq!(
            map_function_path("std::slice::from_raw_parts"),
            Some("rusty::from_raw_parts")
        );
        assert_eq!(
            map_function_path("slice::from_raw_parts_mut"),
            Some("rusty::from_raw_parts_mut")
        );
        assert_eq!(
            map_function_path("ptr::drop_in_place"),
            Some("rusty::ptr::drop_in_place")
        );
        assert_eq!(
            map_function_path("std::mem::forget"),
            Some("rusty::mem::forget")
        );
        assert_eq!(map_function_path("drop"), Some("rusty::mem::drop"));
        assert_eq!(
            map_function_path("std::mem::size_of"),
            Some("rusty::mem::size_of")
        );
        assert_eq!(
            map_function_path("core::mem::align_of"),
            Some("rusty::mem::align_of")
        );
        assert_eq!(
            map_function_path("mem::replace"),
            Some("rusty::mem::replace")
        );
        assert_eq!(
            map_function_path("core::mem::swap"),
            Some("rusty::mem::swap")
        );
        assert_eq!(
            map_function_path("usize::checked_next_power_of_two"),
            Some("rusty::checked_next_power_of_two_usize")
        );
        assert_eq!(
            map_function_path("core::alloc::alloc"),
            Some("rusty::alloc::alloc")
        );
        assert_eq!(
            map_function_path("std::alloc::dealloc"),
            Some("rusty::alloc::dealloc")
        );
        assert_eq!(
            map_function_path("alloc::handle_alloc_error"),
            Some("rusty::alloc::handle_alloc_error")
        );
        assert_eq!(
            map_function_path("alloc::fmt::format"),
            Some("rusty::alloc::fmt::format")
        );
        assert_eq!(
            map_function_path("alloc::__export::must_use"),
            Some("rusty::alloc::__export::must_use")
        );
        assert_eq!(
            map_function_path("std::mem::ManuallyDrop::new"),
            Some("rusty::mem::manually_drop_new")
        );
        assert_eq!(
            map_function_path("ManuallyDrop::new"),
            Some("rusty::mem::manually_drop_new")
        );
        assert_eq!(
            map_function_path("panic::resume_unwind"),
            Some("rusty::panic::resume_unwind")
        );
        assert_eq!(
            map_function_path("std::rt::begin_panic"),
            Some("rusty::panic::begin_panic")
        );
        assert_eq!(
            map_function_path("rt::panic_fmt"),
            Some("rusty::panicking::panic_fmt")
        );
        assert_eq!(map_function_path("std::process::abort"), Some("std::abort"));
        assert_eq!(
            map_function_path("std::io::_print"),
            Some("rusty::io::_print")
        );
        assert_eq!(
            map_function_path("std::future::ready"),
            Some("rusty::future::ready")
        );
        assert_eq!(
            map_function_path("std::string_view::from_utf8"),
            Some("rusty::str_runtime::from_utf8")
        );
        assert_eq!(
            map_function_path("std::string_view::from_utf8_unchecked"),
            Some("rusty::str_runtime::from_utf8_unchecked")
        );
        assert_eq!(
            map_function_path("std::string_view::from_utf8_unchecked_mut"),
            Some("rusty::str_runtime::from_utf8_unchecked_mut")
        );
        assert_eq!(
            map_function_path("futures_timer::Delay::new"),
            Some("rusty::future::Delay::new_")
        );
        assert_eq!(map_function_path("repeat"), Some("rusty::repeat"));
        assert_eq!(
            map_function_path("core::iter::repeat"),
            Some("rusty::repeat")
        );
        assert_eq!(
            map_function_path("alloc::boxed::box_new"),
            Some("rusty::boxed::box_new")
        );
        assert_eq!(
            map_function_path("into_vec"),
            Some("rusty::boxed::into_vec")
        );
        assert_eq!(map_function_path("Add::add"), Some("rusty::ops::add_fn"));
        assert_eq!(
            map_function_path("std::hint::unreachable_unchecked"),
            Some("rusty::intrinsics::unreachable")
        );
        assert_eq!(
            map_function_path("unreachable_unchecked"),
            Some("rusty::intrinsics::unreachable")
        );
        assert_eq!(
            map_function_path("core::ops::Add::add"),
            Some("rusty::ops::add_fn")
        );
    }

    #[test]
    fn test_user_type_map_flat() {
        let toml_str = r#"
            [serde]
            Serialize = "serde::Serialize"
            Deserialize = "serde::Deserialize"
        "#;
        let table: toml::value::Table = toml::from_str(toml_str).unwrap();
        let mut mappings = std::collections::HashMap::new();
        UserTypeMap::flatten_table("", &table, &mut mappings);
        assert_eq!(
            mappings.get("serde::Serialize").map(|s| s.as_str()),
            Some("serde::Serialize")
        );
        assert_eq!(
            mappings.get("serde::Deserialize").map(|s| s.as_str()),
            Some("serde::Deserialize")
        );
    }

    #[test]
    fn test_user_type_map_nested() {
        let toml_str = r#"
            [tokio]
            [tokio.runtime]
            Runtime = "tokio::runtime::Runtime"
        "#;
        let table: toml::value::Table = toml::from_str(toml_str).unwrap();
        let mut mappings = std::collections::HashMap::new();
        UserTypeMap::flatten_table("", &table, &mut mappings);
        assert_eq!(
            mappings.get("tokio::runtime::Runtime").map(|s| s.as_str()),
            Some("tokio::runtime::Runtime")
        );
    }

    #[test]
    fn test_user_type_map_lookup() {
        let mut map = UserTypeMap::default();
        map.mappings.insert(
            "serde::Serialize".to_string(),
            "/* Serialize */".to_string(),
        );
        assert_eq!(map.lookup("serde::Serialize"), Some("/* Serialize */"));
        assert_eq!(map.lookup("unknown::Type"), None);
    }
}
