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

        // Error handling
        "Option" | "std::option::Option" => Some(("rusty::Option", true)),
        "Result" | "std::result::Result" => Some(("rusty::Result", true)),

        // Concurrency
        "Mutex" | "std::sync::Mutex" => Some(("rusty::Mutex", true)),
        "RwLock" | "std::sync::RwLock" => Some(("rusty::RwLock", true)),

        // MaybeUninit
        "MaybeUninit" | "std::mem::MaybeUninit" => Some(("rusty::MaybeUninit", true)),

        _ => None,
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
        assert_eq!(map_std_type("HashMap"), Some(("rusty::HashMap", true)));
        assert_eq!(map_std_type("Mutex"), Some(("rusty::Mutex", true)));
        assert_eq!(map_std_type("UnknownType"), None);
    }

    #[test]
    fn test_std_types_full_path() {
        assert_eq!(
            map_std_type("std::vec::Vec"),
            Some(("rusty::Vec", true))
        );
        assert_eq!(
            map_std_type("std::sync::Arc"),
            Some(("rusty::Arc", true))
        );
        assert_eq!(
            map_std_type("std::collections::HashMap"),
            Some(("rusty::HashMap", true))
        );
    }
}
