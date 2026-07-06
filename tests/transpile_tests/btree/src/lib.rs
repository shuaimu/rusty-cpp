//! Focused parity tests for `BTreeMap` / `BTreeSet` — exercises the ordered-map
//! and ordered-set codegen surface (insert/get/remove, sorted iteration, range
//! queries, first/last, entry API, keys/values, set algebra, retain) in
//! isolation. Like the `vec` crate, bindings are type-annotated so the tests
//! target the collections' *methods* rather than constructor element
//! inference (covered by the standalone hashbrown/indexmap crates).

use std::collections::{BTreeMap, BTreeSet};

// ───────────────────────────── BTreeMap ─────────────────────────────

#[test]
fn map_insert_get_len() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    assert!(m.is_empty());
    assert_eq!(m.insert(2, 20), None);
    assert_eq!(m.insert(1, 10), None);
    assert_eq!(m.insert(3, 30), None);
    assert_eq!(m.insert(2, 22), Some(20));
    assert_eq!(m.len(), 3);
    assert_eq!(m.get(&2), Some(&22));
    assert_eq!(m.get(&9), None);
    assert!(m.contains_key(&1));
    assert!(!m.contains_key(&9));
}

#[test]
fn map_remove() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    for k in 0..6 {
        m.insert(k, k * 100);
    }
    assert_eq!(m.remove(&3), Some(300));
    assert_eq!(m.remove(&3), None);
    assert_eq!(m.len(), 5);
    assert!(!m.contains_key(&3));
}

#[test]
fn map_ordered_iteration() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(30, 3);
    m.insert(10, 1);
    m.insert(20, 2);
    m.insert(5, 0);
    let mut keys: Vec<i32> = Vec::new();
    let mut vals: Vec<i32> = Vec::new();
    for (k, v) in &m {
        keys.push(*k);
        vals.push(*v);
    }
    assert_eq!(keys, vec![5, 10, 20, 30]);
    assert_eq!(vals, vec![0, 1, 2, 3]);
}

#[test]
fn map_first_last() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(7, 70);
    m.insert(2, 20);
    m.insert(9, 90);
    assert_eq!(m.first_key_value(), Some((&2, &20)));
    assert_eq!(m.last_key_value(), Some((&9, &90)));
}

#[test]
fn map_range_query() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    for k in 0..10 {
        m.insert(k, k * k);
    }
    let mut in_range: Vec<i32> = Vec::new();
    for (k, _v) in m.range(3..7) {
        in_range.push(*k);
    }
    assert_eq!(in_range, vec![3, 4, 5, 6]);
}

#[test]
fn map_get_mut_updates() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(1, 100);
    if let Some(v) = m.get_mut(&1) {
        *v += 5;
    }
    assert_eq!(m.get(&1), Some(&105));
}

#[test]
fn map_entry_or_insert() {
    let mut counts: BTreeMap<i32, i32> = BTreeMap::new();
    let data: Vec<i32> = vec![1, 2, 2, 3, 3, 3];
    for x in &data {
        *counts.entry(*x).or_insert(0) += 1;
    }
    assert_eq!(counts.get(&1), Some(&1));
    assert_eq!(counts.get(&2), Some(&2));
    assert_eq!(counts.get(&3), Some(&3));
}

#[test]
fn map_keys_values_collect() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(4, 40);
    m.insert(1, 10);
    m.insert(3, 30);
    let keys: Vec<i32> = m.keys().copied().collect();
    let values: Vec<i32> = m.values().copied().collect();
    assert_eq!(keys, vec![1, 3, 4]);
    assert_eq!(values, vec![10, 30, 40]);
}

#[test]
fn map_string_values() {
    let mut m: BTreeMap<i32, String> = BTreeMap::new();
    m.insert(2, String::from("two"));
    m.insert(1, String::from("one"));
    assert_eq!(m.get(&1).map(|s| s.as_str()), Some("one"));
    let joined: Vec<String> = m.values().cloned().collect();
    assert_eq!(joined.len(), 2);
    assert_eq!(joined[0], "one");
    assert_eq!(joined[1], "two");
}

#[test]
fn map_retain() {
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    for k in 0..8 {
        m.insert(k, k);
    }
    m.retain(|k, _v| k % 2 == 0);
    let keys: Vec<i32> = m.keys().copied().collect();
    assert_eq!(keys, vec![0, 2, 4, 6]);
}

// ───────────────────────────── BTreeSet ─────────────────────────────

#[test]
fn set_insert_contains_remove() {
    let mut s: BTreeSet<i32> = BTreeSet::new();
    assert!(s.insert(5));
    assert!(s.insert(3));
    assert!(!s.insert(5));
    assert_eq!(s.len(), 2);
    assert!(s.contains(&3));
    assert!(!s.contains(&4));
    assert!(s.remove(&3));
    assert!(!s.remove(&3));
    assert_eq!(s.len(), 1);
}

#[test]
fn set_ordered_iteration() {
    let mut s: BTreeSet<i32> = BTreeSet::new();
    s.insert(40);
    s.insert(10);
    s.insert(30);
    s.insert(20);
    let ordered: Vec<i32> = s.iter().copied().collect();
    assert_eq!(ordered, vec![10, 20, 30, 40]);
}

#[test]
fn set_first_last_range() {
    let mut s: BTreeSet<i32> = BTreeSet::new();
    for x in [9, 1, 7, 3, 5] {
        s.insert(x);
    }
    assert_eq!(s.first(), Some(&1));
    assert_eq!(s.last(), Some(&9));
    let mid: Vec<i32> = s.range(3..8).copied().collect();
    assert_eq!(mid, vec![3, 5, 7]);
}

#[test]
fn set_union_intersection_difference() {
    let mut a: BTreeSet<i32> = BTreeSet::new();
    let mut b: BTreeSet<i32> = BTreeSet::new();
    for x in 1..6 {
        a.insert(x);
    }
    for x in 4..9 {
        b.insert(x);
    }
    let union: Vec<i32> = a.union(&b).copied().collect();
    let inter: Vec<i32> = a.intersection(&b).copied().collect();
    let diff: Vec<i32> = a.difference(&b).copied().collect();
    assert_eq!(union, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(inter, vec![4, 5]);
    assert_eq!(diff, vec![1, 2, 3]);
}

#[test]
fn set_subset_superset() {
    let mut small: BTreeSet<i32> = BTreeSet::new();
    let mut big: BTreeSet<i32> = BTreeSet::new();
    for x in [2, 4] {
        small.insert(x);
    }
    for x in [1, 2, 3, 4, 5] {
        big.insert(x);
    }
    assert!(small.is_subset(&big));
    assert!(big.is_superset(&small));
    assert!(!big.is_subset(&small));
}

#[test]
fn set_retain() {
    let mut s: BTreeSet<i32> = BTreeSet::new();
    for x in 0..10 {
        s.insert(x);
    }
    s.retain(|x| x % 3 == 0);
    let kept: Vec<i32> = s.iter().copied().collect();
    assert_eq!(kept, vec![0, 3, 6, 9]);
}
