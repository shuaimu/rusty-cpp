// Baseline: Rust std::collections::BinaryHeap<i32>
// Matches the workload shape in docs/binary_heap_port/binary_heap_port_bench.cpp:
//   PUSH n=10000, POP n=10000, MIX HALF=5000 push/pop pairs; ROUNDS=200.
//
// Reproduce:
//   rustc -O docs/binary_heap_port/rust_bench.rs \
//       -o /tmp/rust_binary_heap_bench
//   /tmp/rust_binary_heap_bench

use std::collections::BinaryHeap;
use std::hint::black_box;
use std::time::Instant;

const N: usize = 10_000;
const ROUNDS: usize = 200;

fn make_workload(n: usize, seed: u32) -> Vec<i32> {
    use std::num::Wrapping;
    let mut s = Wrapping(seed as u64);
    let mut values: Vec<i32> = Vec::with_capacity(n);
    for _ in 0..n {
        s = (s * Wrapping(6364136223846793005)) + Wrapping(1442695040888963407);
        values.push(((s.0 >> 32) as u32) as i32);
    }
    values
}

fn main() {
    let values = make_workload(N, 42);

    // -------- PUSH --------
    {
        let mut guard: i64 = 0;
        let t0 = Instant::now();
        for _ in 0..ROUNDS {
            let mut h: BinaryHeap<i32> = BinaryHeap::with_capacity(N);
            for &v in &values {
                h.push(v);
            }
            guard += h.len() as i64;
        }
        let avg = t0.elapsed().as_nanos() as f64 / ROUNDS as f64;
        println!("PUSH n={} x {} rounds", N, ROUNDS);
        println!("  Rust std::BinaryHeap  : {:>9.0} ns/iter", avg);
        println!("  guard={}\n", guard);
        black_box(guard);
    }

    // -------- POP --------
    {
        let mut guard: i64 = 0;
        let t0 = Instant::now();
        for _ in 0..ROUNDS {
            // Rebuild per round so we time only the pop loop.
            let mut h: BinaryHeap<i32> = BinaryHeap::with_capacity(N);
            for &v in &values {
                h.push(v);
            }
            // Stop the clock segment cleanly: time only the pop loop.
            let t1 = Instant::now();
            let mut sum: i32 = 0;
            while let Some(x) = h.pop() {
                sum = sum.wrapping_add(x);
            }
            let _ = t1; // suppress unused warning; per-round split not needed for the avg
            guard += sum as i64;
        }
        let avg = t0.elapsed().as_nanos() as f64 / ROUNDS as f64;
        // Note: `avg` here also includes the rebuild cost; we report the
        // pop-only timing in a 2nd pass below for fairness with the C++
        // bench (which only times the pop loop, not the rebuild).
        let _ = avg;

        // Second pass: time ONLY the pop loop (rebuild outside the timer).
        let mut guard2: i64 = 0;
        let mut tot_ns = 0u128;
        for _ in 0..ROUNDS {
            let mut h: BinaryHeap<i32> = BinaryHeap::with_capacity(N);
            for &v in &values {
                h.push(v);
            }
            let t1 = Instant::now();
            let mut sum: i32 = 0;
            while let Some(x) = h.pop() {
                sum = sum.wrapping_add(x);
            }
            tot_ns += t1.elapsed().as_nanos();
            guard2 += sum as i64;
        }
        let avg_pop = tot_ns as f64 / ROUNDS as f64;
        println!("POP  n={} x {} rounds", N, ROUNDS);
        println!("  Rust std::BinaryHeap  : {:>9.0} ns/iter", avg_pop);
        println!("  guard={}\n", guard2);
        black_box(guard);
        black_box(guard2);
    }

    // -------- MIX --------
    {
        let half = N / 2;
        let mut guard: i64 = 0;
        let mut tot_ns = 0u128;
        for _ in 0..ROUNDS {
            let mut h: BinaryHeap<i32> = BinaryHeap::with_capacity(N);
            for i in 0..half {
                h.push(values[i]);
            }
            let t1 = Instant::now();
            let mut sum: i32 = 0;
            for i in 0..half {
                h.push(values[half + i]);
                if let Some(x) = h.pop() {
                    sum = sum.wrapping_add(x);
                }
            }
            tot_ns += t1.elapsed().as_nanos();
            guard += sum as i64;
        }
        let avg = tot_ns as f64 / ROUNDS as f64;
        println!("MIX  n={} x {} rounds (HALF={} push/pop pairs)", N, ROUNDS, half);
        println!("  Rust std::BinaryHeap  : {:>9.0} ns/iter", avg);
        println!("  guard={}\n", guard);
        black_box(guard);
    }
}
