//! Focused parity tests for `Vec` — exercises the core `rusty::Vec` codegen
//! (push/pop, indexing, iteration, sort, drain, retain, dedup, insert/remove,
//! extend, collect) in isolation. Element types are annotated so the tests
//! target Vec's *methods* rather than the `Vec::new()` element-inference leak
//! (tracked separately; the standalone hashbrown/indexmap crates cover that).

#[test]
fn push_pop_len() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..5 {
        v.push(i * i);
    }
    assert_eq!(v.len(), 5);
    assert_eq!(v.pop(), Some(16));
    assert_eq!(v.pop(), Some(9));
    assert_eq!(v.len(), 3);
    assert_eq!(v, vec![0, 1, 4]);
}

#[test]
fn index_and_iterate() {
    let v: Vec<i32> = vec![10, 20, 30, 40];
    assert_eq!(v[0], 10);
    assert_eq!(v[3], 40);
    let mut sum = 0;
    for x in &v {
        sum += *x;
    }
    assert_eq!(sum, 100);
}

#[test]
fn extend_and_collect() {
    let mut v: Vec<i32> = vec![1, 2, 3];
    v.extend(4..=6);
    assert_eq!(v, vec![1, 2, 3, 4, 5, 6]);
    let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
    assert_eq!(doubled, vec![2, 4, 6, 8, 10, 12]);
}

#[test]
fn sort_and_sort_by() {
    let mut v: Vec<i32> = vec![5, 3, 8, 1, 9, 2];
    v.sort();
    assert_eq!(v, vec![1, 2, 3, 5, 8, 9]);
    v.sort_by(|a, b| b.cmp(a));
    assert_eq!(v, vec![9, 8, 5, 3, 2, 1]);
}

#[test]
fn retain_and_dedup() {
    let mut v: Vec<i32> = vec![1, 1, 2, 3, 3, 3, 4, 5, 5];
    v.dedup();
    assert_eq!(v, vec![1, 2, 3, 4, 5]);
    v.retain(|x| x % 2 == 1);
    assert_eq!(v, vec![1, 3, 5]);
}

#[test]
fn drain_range() {
    let mut v: Vec<i32> = vec![0, 1, 2, 3, 4, 5];
    let drained: Vec<i32> = v.drain(1..4).collect();
    assert_eq!(drained, vec![1, 2, 3]);
    assert_eq!(v, vec![0, 4, 5]);
}

#[test]
fn insert_and_remove() {
    let mut v: Vec<i32> = vec![1, 2, 4, 5];
    v.insert(2, 3);
    assert_eq!(v, vec![1, 2, 3, 4, 5]);
    let removed = v.remove(0);
    assert_eq!(removed, 1);
    assert_eq!(v, vec![2, 3, 4, 5]);
}

#[test]
fn reverse_and_contains() {
    let mut v: Vec<i32> = vec![1, 2, 3, 4];
    v.reverse();
    assert_eq!(v, vec![4, 3, 2, 1]);
    assert!(v.contains(&3));
    assert!(!v.contains(&9));
    assert_eq!(v.iter().position(|&x| x == 2), Some(2));
}

#[test]
fn collect_from_range_and_filter() {
    let evens: Vec<i32> = (0..10).filter(|x| x % 2 == 0).collect();
    assert_eq!(evens, vec![0, 2, 4, 6, 8]);
    let total: i32 = evens.iter().sum();
    assert_eq!(total, 20);
}

#[test]
fn truncate_and_clear() {
    let mut v: Vec<i32> = vec![1, 2, 3, 4, 5];
    v.truncate(3);
    assert_eq!(v, vec![1, 2, 3]);
    v.clear();
    assert!(v.is_empty());
}
