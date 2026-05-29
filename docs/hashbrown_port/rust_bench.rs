// Baseline: Rust std::collections::HashMap<i32, i32>
// Matches the workload shape in docs/hashbrown_port/bench.cpp

use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

const N: usize = 200;
const ROUNDS: usize = 1000;

fn make_workload(n: usize, seed: u32) -> (Vec<i32>, Vec<i32>) {
    use std::num::Wrapping;
    let mut s = Wrapping(seed as u64);
    let mut keys: Vec<i32> = (0..n).map(|i| i as i32).collect();
    let mut vals: Vec<i32> = (0..n).map(|_| {
        s = (s * Wrapping(6364136223846793005)) + Wrapping(1442695040888963407);
        ((s.0 >> 32) as u32) as i32
    }).collect();
    // shuffle keys (Fisher–Yates)
    for i in (1..n).rev() {
        s = (s * Wrapping(6364136223846793005)) + Wrapping(1442695040888963407);
        let j = (s.0 as usize) % (i + 1);
        keys.swap(i, j);
    }
    for i in (1..n).rev() {
        s = (s * Wrapping(6364136223846793005)) + Wrapping(1442695040888963407);
        let j = (s.0 as usize) % (i + 1);
        vals.swap(i, j);
    }
    (keys, vals)
}

fn main() {
    let (keys, vals) = make_workload(N, 42);

    // INSERT
    {
        let mut guard: i64 = 0;
        let t0 = Instant::now();
        for _ in 0..ROUNDS {
            let mut m: HashMap<i32, i32> = HashMap::with_capacity(N * 4);
            for i in 0..N {
                m.insert(keys[i], vals[i]);
            }
            guard += m.len() as i64;
        }
        let avg = t0.elapsed().as_nanos() as f64 / ROUNDS as f64;
        println!("INSERT n={} x {} rounds", N, ROUNDS);
        println!("  Rust std::HashMap    : {:>8.0} ns/iter", avg);
        black_box(guard);
    }

    // LOOKUP
    {
        let mut m: HashMap<i32, i32> = HashMap::with_capacity(N * 4);
        for i in 0..N {
            m.insert(keys[i], vals[i]);
        }
        let mut guard: i64 = 0;
        let t0 = Instant::now();
        for _ in 0..ROUNDS {
            let mut sum: i64 = 0;
            for i in 0..N {
                if let Some(&v) = m.get(&keys[i]) {
                    sum += v as i64;
                }
            }
            guard += sum;
        }
        let avg = t0.elapsed().as_nanos() as f64 / ROUNDS as f64;
        println!("LOOKUP n={} x {} rounds", N, ROUNDS);
        println!("  Rust std::HashMap    : {:>8.0} ns/iter", avg);
        black_box(guard);
    }
}
