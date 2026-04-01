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
        "Condvar" | "std::sync::Condvar" => Some(("rusty::Condvar", false)),
        "Barrier" | "std::sync::Barrier" => Some(("rusty::Barrier", false)),
        "Once" | "std::sync::Once" => Some(("rusty::Once", false)),

        // MaybeUninit
        "MaybeUninit" | "std::mem::MaybeUninit" => Some(("rusty::MaybeUninit", true)),

        // I/O types
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
        // String::from → rusty::String constructor
        "String::from" => Some("rusty::String::from"),
        "String::new" => Some("rusty::String::new_"),
        // Vec::new
        "Vec::new" => Some("rusty::Vec::new_"),
        "Vec::with_capacity" => Some("rusty::Vec::with_capacity"),
        // thread::spawn
        "thread::spawn" | "std::thread::spawn" => Some("rusty::thread::spawn"),
        // I/O functions
        "io::stdin" | "std::io::stdin" => Some("rusty::io::stdin_"),
        "io::stdout" | "std::io::stdout" => Some("rusty::io::stdout_"),
        "io::stderr" | "std::io::stderr" => Some("rusty::io::stderr_"),
        "io::Cursor::new" | "std::io::Cursor::new" => Some("rusty::io::Cursor::new_"),
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
        assert_eq!(map_std_type("UnsafeCell"), Some(("rusty::UnsafeCell", true)));
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
    }

    #[test]
    fn test_function_path_mapping() {
        assert_eq!(map_function_path("Box::new"), Some("rusty::Box::new_"));
        assert_eq!(map_function_path("String::from"), Some("rusty::String::from"));
        assert_eq!(map_function_path("String::new"), Some("rusty::String::new_"));
        assert_eq!(map_function_path("Vec::new"), Some("rusty::Vec::new_"));
        assert_eq!(map_function_path("thread::spawn"), Some("rusty::thread::spawn"));
        assert_eq!(map_function_path("Unknown::method"), None);
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
        assert_eq!(mappings.get("serde::Serialize").map(|s| s.as_str()), Some("serde::Serialize"));
        assert_eq!(mappings.get("serde::Deserialize").map(|s| s.as_str()), Some("serde::Deserialize"));
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
        map.mappings.insert("serde::Serialize".to_string(), "/* Serialize */".to_string());
        assert_eq!(map.lookup("serde::Serialize"), Some("/* Serialize */"));
        assert_eq!(map.lookup("unknown::Type"), None);
    }
}
