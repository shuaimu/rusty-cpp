use super::*;

fn generate(source: &str) -> String {
    let mut codegen = CodeGen::new();
    codegen.emit_file(&syn::parse_file(source).unwrap(), None);
    codegen.into_output()
}

const ENTRY: &str = r#"
    use std::sync::Arc;
    pub struct Entry { pub value: i32 }
    impl Entry {
        pub fn read(&self) -> i32 { self.value }
        pub fn retain(&self, other: Arc<Entry>) -> i32 { other.value }
    }
"#;

#[test]
fn owned_map_iteration_field_preserves_arc_value_type() {
    let output = generate(&format!(
        r#"{ENTRY}
        use std::collections::HashMap;
        pub struct Batch {{ entries: HashMap<i64, Arc<Entry>> }}
        pub fn consume(batch: Batch) -> i32 {{
            let mut sum = 0;
            for (_key, entry) in batch.entries {{
                sum += entry.retain(entry.clone());
            }}
            sum
        }}
    "#
    ));
    assert!(output.contains("entry->retain("), "{output}");
    assert!(!output.contains("entry.retain("), "{output}");
}

#[test]
fn owned_map_iteration_resolves_renamed_import_and_generic_alias() {
    let output = generate(&format!(
        r#"{ENTRY}
        use std::collections::HashMap as Entries;
        type ById<T> = Entries<i64, T>;
        pub fn consume(entries: ById<Arc<Entry>>) -> i32 {{
            let mut sum = 0;
            for (_key, entry) in entries {{ sum += entry.read(); }}
            sum
        }}
    "#
    ));
    assert!(output.contains("entry->read("), "{output}");
    assert!(!output.contains("entry.read("), "{output}");
}

#[test]
fn owned_map_iteration_types_nested_tuple_bindings() {
    let output = generate(&format!(
        r#"{ENTRY}
        pub fn consume(entries: std::collections::HashMap<i64, (Arc<Entry>, Arc<Entry>)>) -> i32 {{
            let mut sum = 0;
            for (_key, (first, second)) in entries {{
                sum += first.read() + second.read();
            }}
            sum
        }}
    "#
    ));
    assert!(output.contains("first->read("), "{output}");
    assert!(output.contains("second->read("), "{output}");
}

#[test]
fn owned_map_iteration_does_not_invent_items_for_local_namesake() {
    let output = generate(&format!(
        r#"{ENTRY}
        pub struct HashMap<K, V> {{
            rows: Vec<(K, Entry)>,
            marker: std::marker::PhantomData<V>,
        }}
        impl<K, V> IntoIterator for HashMap<K, V> {{
            type Item = (K, Entry);
            type IntoIter = std::vec::IntoIter<Self::Item>;
            fn into_iter(self) -> Self::IntoIter {{ self.rows.into_iter() }}
        }}
        pub fn consume(entries: HashMap<i64, Arc<Entry>>) -> i32 {{
            let mut sum = 0;
            for (_key, entry) in entries {{ sum += entry.read(); }}
            sum
        }}
    "#
    ));
    assert!(output.contains("entry.read("), "{output}");
    assert!(!output.contains("entry->read("), "{output}");
}

#[test]
fn owned_map_iteration_rejects_local_std_module_identity() {
    let output = generate(
        &format!(
            r#"{ENTRY}
        mod std {{
            pub mod collections {{
                pub struct HashMap<K, V> {{
                    pub rows: Vec<(K, super::super::Entry)>,
                    pub marker: ::std::marker::PhantomData<V>,
                }}
                impl<K, V> IntoIterator for HashMap<K, V> {{
                    type Item = (K, super::super::Entry);
                    type IntoIter = ::std::vec::IntoIter<Self::Item>;
                    fn into_iter(self) -> Self::IntoIter {{ self.rows.into_iter() }}
                }}
            }}
        }}
        pub fn consume(entries: std::collections::HashMap<i64, Arc<Entry>>) -> i32 {{
            let mut sum = 0;
            for (_key, entry) in entries {{ sum += entry.read(); }}
            sum
        }}
    "#
        )
        .replace("use std::sync::Arc;", "use ::std::sync::Arc;"),
    );
    assert!(output.contains("entry.read("), "{output}");
    assert!(!output.contains("entry->read("), "{output}");
}
