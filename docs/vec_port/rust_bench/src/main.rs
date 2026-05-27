// Native Rust std::Vec bench. Mirror of vec_bench_4way.cpp for the
// std::Vec runtime — same N, same trial count, same workloads.

use std::time::Instant;

const N: usize = 10_000_000;
const REPEATS: usize = 5;

#[inline(always)]
fn ms(elapsed: std::time::Duration) -> f64 {
    elapsed.as_secs_f64() * 1000.0
}

fn bench_push_grow() -> f64 {
    let t0 = Instant::now();
    let mut v: Vec<i32> = Vec::new();
    for i in 0..N {
        v.push(i as i32);
    }
    std::hint::black_box(&v);
    ms(t0.elapsed())
}

fn bench_push_reserved() -> f64 {
    let t0 = Instant::now();
    let mut v: Vec<i32> = Vec::with_capacity(N);
    for i in 0..N {
        v.push(i as i32);
    }
    std::hint::black_box(&v);
    ms(t0.elapsed())
}

fn bench_iter() -> (f64, i64) {
    let mut v: Vec<i32> = Vec::with_capacity(N);
    for i in 0..N {
        v.push(i as i32);
    }
    let t0 = Instant::now();
    let mut s: i64 = 0;
    for x in v.iter() {
        s += *x as i64;
    }
    let elapsed = t0.elapsed();
    (ms(elapsed), std::hint::black_box(s))
}

fn bench_index() -> (f64, i64) {
    let mut v: Vec<i32> = Vec::with_capacity(N);
    for i in 0..N {
        v.push(i as i32);
    }
    let t0 = Instant::now();
    let mut s: i64 = 0;
    for i in 0..N {
        s += v[i] as i64;
    }
    let elapsed = t0.elapsed();
    (ms(elapsed), std::hint::black_box(s))
}

fn main() {
    println!(
        "Rust std::Vec bench: N={} i32 pushes, {} trials each\n",
        N, REPEATS
    );

    let mut t_pg = 0.0;
    let mut t_pr = 0.0;
    let mut t_it = 0.0;
    let mut t_ix = 0.0;
    let mut sink: i64 = 0;
    for _ in 0..REPEATS {
        t_pg += bench_push_grow();
        t_pr += bench_push_reserved();
        let (m, s) = bench_iter();
        t_it += m;
        sink ^= s;
        let (m, s) = bench_index();
        t_ix += m;
        sink ^= s;
    }
    let avg = |t: f64| t / REPEATS as f64;
    println!("                                  push-grow  push-reserved   iterate     index");
    println!(
        "std::Vec         (Rust)           {:8.2} ms   {:8.2} ms {:8.2} ms {:8.2} ms",
        avg(t_pg),
        avg(t_pr),
        avg(t_it),
        avg(t_ix)
    );
    println!("\n(sink={} — defeats DCE)", sink);
}
