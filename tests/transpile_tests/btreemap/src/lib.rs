// Minimal end-to-end fixture for the BTreeMap insert/get write-path.
//
// Goal: the transpiled C++ output must pass `rusty-cpp-checker` with no
// `@unsafe` escapes. See team-todo.md TODO-001-btreemap-e2e-parity-impl.

use std::collections::BTreeMap;

pub fn insert_then_get_present() -> i32 {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(1, 10);
    m.insert(2, 20);
    match m.get(&1) {
        Some(v) => *v,
        None => -1,
    }
}

pub fn insert_then_get_missing() -> i32 {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(1, 10);
    match m.get(&2) {
        Some(v) => *v,
        None => -1,
    }
}

pub fn insert_returns_old() -> i32 {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(1, 10);
    match m.insert(1, 99) {
        Some(old) => old,
        None => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_present() {
        assert_eq!(insert_then_get_present(), 10);
    }

    #[test]
    fn get_missing() {
        assert_eq!(insert_then_get_missing(), -1);
    }

    #[test]
    fn insert_overwrite() {
        assert_eq!(insert_returns_old(), 10);
    }
}
