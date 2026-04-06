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

        // Strings
        "String" | "std::string::String" => Some(("rusty::String", false)),
        "net::TcpStream" | "std::net::TcpStream" => Some(("rusty::net::TcpStream", false)),

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
        "core::task::Poll" => Some(("rusty::Poll", true)),
        "core::task::Context" => Some(("rusty::Context", false)),

        // Runtime compatibility fallbacks for expanded Rust paths.
        "core::cmp::Ordering" => Some(("rusty::cmp::Ordering", false)),
        "std::cmp::Ordering" => Some(("rusty::cmp::Ordering", false)),
        "Any" | "std::any::Any" | "core::any::Any" => Some(("std::any", false)),
        "TypeId" | "std::any::TypeId" | "core::any::TypeId" => {
            Some(("std::type_index", false))
        }
        "slice::Iter" | "core::slice::Iter" | "std::slice::Iter" => {
            Some(("rusty::slice_iter::Iter", true))
        }
        "slice::IterMut" | "core::slice::IterMut" | "std::slice::IterMut" => {
            Some(("rusty::slice_iter::Iter", true))
        }
        "core::fmt::Result" | "fmt::Result" => Some(("rusty::fmt::Result", false)),
        "core::fmt::Formatter" | "fmt::Formatter" => Some(("rusty::fmt::Formatter", false)),
        "core::fmt::Arguments" | "fmt::Arguments" => Some(("rusty::fmt::Arguments", false)),
        "core::fmt::Error" | "std::fmt::Error" | "fmt::Error" => {
            Some(("rusty::fmt::Error", false))
        }
        "std::str::Utf8Error" | "core::str::Utf8Error" => {
            Some(("rusty::str_runtime::Utf8Error", false))
        }
        "PhantomData" | "std::marker::PhantomData" | "core::marker::PhantomData" => {
            Some(("rusty::PhantomData", true))
        }
        "Pin" | "std::pin::Pin" | "core::pin::Pin" => Some(("rusty::pin::Pin", true)),
        "std::path::Path" => Some(("rusty::path::Path", false)),
        "std::ffi::OsStr" => Some(("rusty::ffi::OsStr", false)),
        "std::ffi::CStr" => Some(("rusty::ffi::CStr", false)),

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
        "str" => Some(("std::string_view", false)),

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
        // Vec::new
        "Vec::new" | "std::vec::Vec::new" | "alloc::vec::Vec::new" => Some("rusty::Vec::new_"),
        "vec::from_elem" | "std::vec::from_elem" | "alloc::vec::from_elem" => {
            Some("rusty::array_repeat")
        }
        "Vec::with_capacity" => Some("rusty::Vec::with_capacity"),
        // thread::spawn
        "thread::spawn" | "std::thread::spawn" => Some("rusty::thread::spawn"),
        // I/O functions
        "io::stdin" | "std::io::stdin" => Some("rusty::io::stdin_"),
        "io::stdout" | "std::io::stdout" => Some("rusty::io::stdout_"),
        "io::stderr" | "std::io::stderr" => Some("rusty::io::stderr_"),
        "io::_print" | "std::io::_print" => Some("rusty::io::_print"),
        "io::Cursor::new" | "std::io::Cursor::new" => Some("rusty::io::Cursor::new_"),

        // Expanded-Rust runtime compatibility shims.
        "core::intrinsics::discriminant_value" => Some("rusty::intrinsics::discriminant_value"),
        "core::intrinsics::unreachable" => Some("rusty::intrinsics::unreachable"),
        "core::panicking::panic_fmt" => Some("rusty::panicking::panic_fmt"),
        "core::panicking::assert_failed" => Some("rusty::panicking::assert_failed"),
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
        "core::ptr::const_ptr::offset"
        | "std::ptr::const_ptr::offset"
        | "ptr::const_ptr::offset" => Some("rusty::ptr::offset"),
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
        "std::mem::replace" | "mem::replace" => Some("rusty::mem::replace"),
        "std::mem::forget" | "mem::forget" => Some("rusty::mem::forget"),
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
        "core::cmp::min" | "std::cmp::min" => Some("core::cmp::min"),
        "core::cmp::max" | "std::cmp::max" => Some("core::cmp::max"),
        "std::str::from_utf8" | "core::str::from_utf8" | "str::from_utf8" => {
            Some("rusty::str_runtime::from_utf8")
        }
        "std::str::from_utf8_unchecked"
        | "core::str::from_utf8_unchecked"
        | "str::from_utf8_unchecked" => Some("rusty::str_runtime::from_utf8_unchecked"),
        "std::str::from_utf8_unchecked_mut"
        | "core::str::from_utf8_unchecked_mut"
        | "str::from_utf8_unchecked_mut" => Some("rusty::str_runtime::from_utf8_unchecked_mut"),
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
        assert_eq!(map_std_type("fmt::Error"), Some(("rusty::fmt::Error", false)));
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
            map_std_type("std::mem::ManuallyDrop"),
            Some(("rusty::mem::ManuallyDrop", true))
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
            map_function_path("thread::spawn"),
            Some("rusty::thread::spawn")
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
            map_function_path("core::hash::Hash::hash"),
            Some("rusty::hash::hash")
        );
        assert_eq!(map_function_path("std::cmp::min"), Some("core::cmp::min"));
        assert_eq!(map_function_path("core::cmp::max"), Some("core::cmp::max"));
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
            map_function_path("mem::replace"),
            Some("rusty::mem::replace")
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
            map_function_path("alloc::boxed::box_new"),
            Some("rusty::boxed::box_new")
        );
        assert_eq!(
            map_function_path("into_vec"),
            Some("rusty::boxed::into_vec")
        );
        assert_eq!(map_function_path("Add::add"), Some("rusty::ops::add_fn"));
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
