//! The guard-context matrix — the systematic closure of issues #32/#34/#35.
//!
//! Those issues were all the SAME defect surfacing one syntactic context at a
//! time: the type model says `borrow_mut()` yields `&mut T` (true of Rust),
//! but the C++ runtime yields a guard object, and every consumption context
//! that trusted the model dropped the deref. Fixing them one report at a time
//! left the next context as the next issue.
//!
//! This test instead enumerates the contexts Rust lets a guard-yielding value
//! flow into — inline call, autoderef call on a bound guard, deref
//! assign/read/compound-assign, function argument, comparison, arithmetic,
//! if-let, for loop, return position, chained calls, `lock().unwrap()`,
//! indexing, repeated use — transpiles them all through the real binary, and
//! compiles AND RUNS the result, asserting the values. A new guard bug gets a
//! new row here, and rows cannot regress.
//!
//! clang-only by policy (gcc is not a target); skips when clang is absent.

use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const MATRIX_SOURCE: &str = r####"#include <rusty/rusty.hpp>
namespace demo {
struct Holder {
    rusty::RefCell<rusty::VecDeque<int>> q;     // container in a cell
    rusty::RefCell<int> n;                      // scalar in a cell
    rusty::RefCell<rusty::Option<int>> o;       // enum in a cell
    rusty::Mutex<rusty::VecDeque<int>> mq;      // container in a mutex
};
void sink(const rusty::VecDeque<int>& v);
#if RUSTYCPP_RUST
// C1: inline method call (issue #35, fixed)
fn c1_inline_call(h: &Holder) { h.q.borrow_mut().push_back(1); }
// C2: bound guard + explicit deref method (issue #35, fixed)
fn c2_bound_star_call(h: &Holder) { let mut g = h.q.borrow_mut(); (*g).push_back(2); }
// C3: bound guard + AUTODEREF method call (no star) — Rust allows this
fn c3_bound_autoderef_call(h: &Holder) { let mut g = h.q.borrow_mut(); g.push_back(3); }
// C4: inline deref-assign (issue #34 shape, concrete receiver)
fn c4_inline_deref_assign(h: &Holder) { *h.n.borrow_mut() = 4; }
// C5: bound guard + deref-assign
fn c5_bound_deref_assign(h: &Holder) { let mut g = h.n.borrow_mut(); *g = 5; }
// C6: bound guard + compound assign
fn c6_bound_compound_assign(h: &Holder) { let mut g = h.n.borrow_mut(); *g += 6; }
// C7: deref-read into a value
fn c7_deref_read(h: &Holder) -> i32 { let g = h.n.borrow(); *g }
// C8: guard reborrowed as function argument
fn c8_arg_reborrow(h: &Holder) { let g = h.q.borrow(); sink(&*g); }
// C9: comparison through the guard
fn c9_compare(h: &Holder) -> bool { let g = h.n.borrow(); *g == 7 }
// C10: arithmetic through the guard
fn c10_arith(h: &Holder) -> i32 { let g = h.n.borrow(); *g + 1 }
// C11: if-let over a dereffed guard
fn c11_iflet(h: &Holder) -> i32 { let g = h.o.borrow(); if let Some(x) = *g { x } else { 0 } }
// C12: for loop over iter() through the guard
fn c12_for(h: &Holder) -> i32 { let g = h.q.borrow(); let mut s = 0; for x in g.iter() { s += *x; } s }
// C13: return-position inline guard call
fn c13_return_len(h: &Holder) -> usize { h.q.borrow().len() }
// C14: chained call CONTINUING off a guard-method result
fn c14_chain(h: &Holder) -> bool { h.o.borrow_mut().take().is_some() }
// C15: Mutex lock (same shape, different guard)
fn c15_mutex_inline(h: &Holder) { h.mq.lock().unwrap().push_back(15); }
// C16: bound Mutex guard + autoderef
fn c16_mutex_bound(h: &Holder) { let mut g = h.mq.lock().unwrap(); g.push_back(16); }
// C17: index through the guard
fn c17_index(h: &Holder) -> i32 { let g = h.q.borrow(); (*g)[0] }
// C18: guard held across a statement then reused twice
fn c18_two_uses(h: &Holder) { let mut g = h.q.borrow_mut(); (*g).push_back(18); (*g).push_back(19); }
// C19: guard behind an if-expression initializer (flow, not shape)
fn c19_if_init(h: &Holder, cond: bool) { let mut g = if cond { h.q.borrow_mut() } else { h.q.borrow_mut() }; g.push_back(19); }
// C20: guard behind a match-arm initializer
fn c20_match_init(h: &Holder, k: i32) { let mut g = match k { _ => h.q.borrow_mut() }; g.push_back(20); }
// C21: guard behind a block-tail initializer
fn c21_block_init(h: &Holder) { let mut g = { h.q.borrow_mut() }; g.push_back(21); }
// C22: free-helper family on a bound guard (len/is_empty/first/last/get/contains)
fn c22_helpers(h: &Holder) -> usize {
    let g = h.q.borrow();
    let mut n = g.len();
    if !g.is_empty() { n += 1; }
    if g.front().is_some() { n += 1; }
    if g.back().is_some() { n += 1; }
    if g.get(0).is_some() { n += 1; }
    if g.contains(&19) { n += 1; }
    n
}
// C23: generic receiver, bound guard, autoderef call
fn c23_generic<H>(h: &H) { let mut g = h.q.borrow_mut(); g.push_back(23); }
// C24: drop the guard early, then re-borrow (runtime borrow counter must clear)
fn c24_drop_reborrow(h: &Holder) { let g = h.q.borrow_mut(); drop(g); let g2 = h.q.borrow(); let _ = g2.len(); }
#endif
}
"####;

const RUNTIME_MAIN: &str = r####"
namespace demo { void sink(const rusty::VecDeque<int>& v) { (void)v; } }
#include <cassert>
int main() {
    using namespace demo;
    auto mk = []{ return Holder{
        rusty::RefCell<rusty::VecDeque<int>>(rusty::VecDeque<int>()),
        rusty::RefCell<int>(0),
        rusty::RefCell<rusty::Option<int>>(rusty::Option<int>(7)),
        rusty::Mutex<rusty::VecDeque<int>>(rusty::VecDeque<int>())}; };
    { auto h = mk(); c1_inline_call(h); c2_bound_star_call(h); c3_bound_autoderef_call(h);
      auto g = h.q.borrow(); assert((*g).len() == 3); assert((*g)[0]==1 && (*g)[1]==2 && (*g)[2]==3); }
    { auto h = mk(); c4_inline_deref_assign(h); assert(*h.n.borrow() == 4);
      c5_bound_deref_assign(h); assert(*h.n.borrow() == 5);
      c6_bound_compound_assign(h); assert(*h.n.borrow() == 11);
      assert(c7_deref_read(h) == 11); assert(!c9_compare(h)); assert(c10_arith(h) == 12); }
    { auto h = mk(); c8_arg_reborrow(h); assert(c11_iflet(h) == 7);
      c1_inline_call(h); c2_bound_star_call(h);
      assert(c12_for(h) == 3); assert(c13_return_len(h) == 2);
      assert(c14_chain(h)); assert(!c14_chain(h)); // take() empties the cell
      assert(c17_index(h) == 1); c18_two_uses(h); assert(c13_return_len(h) == 4); }
    { auto h = mk(); c15_mutex_inline(h); c16_mutex_bound(h);
      auto g = h.mq.lock().unwrap(); assert((*g).len() == 2); assert((*g)[0]==15 && (*g)[1]==16); }
    // Flow rows: guard behind if/match/block initializers, generic receiver,
    // the helper family on a bound guard, and drop-then-reborrow.
    { auto h = mk(); c19_if_init(h, true); c20_match_init(h, 0); c21_block_init(h); c23_generic(h);
      assert(c13_return_len(h) == 4);          // 19, 20, 21, 23
      assert(c22_helpers(h) == 9);             // len(4) + 5 positive probes
      c24_drop_reborrow(h);                    // must not double-borrow panic
      assert(c13_return_len(h) == 4); }
    return 0;
}
"####;

fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    for candidate in ["clang++", "clang++-22", "clang++-21"] {
        if Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some(candidate.to_string());
        }
    }
    None
}

#[test]
fn every_guard_consumption_context_transpiles_compiles_and_runs() {
    let Some(clang) = find_clang() else {
        eprintln!("skipping guard context matrix: no clang++ in PATH or CXX");
        return;
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let cpp = temp.path().join("guard_matrix.cpp");
    std::fs::write(&cpp, MATRIX_SOURCE).expect("write matrix");

    // Rewrite through the actual built binary — the emission under test.
    let transpiler = env!("CARGO_BIN_EXE_rusty-cpp-transpiler");
    let rw = Command::new(transpiler)
        .args(["inline-rust", "--rewrite", "--files"])
        .arg(&cpp)
        .output()
        .expect("run transpiler");
    assert!(
        rw.status.success(),
        "inline-rust rewrite failed:\n{}\n{}",
        String::from_utf8_lossy(&rw.stdout),
        String::from_utf8_lossy(&rw.stderr)
    );

    let mut source = std::fs::read_to_string(&cpp).expect("read rewritten");
    source.push_str(RUNTIME_MAIN);
    std::fs::write(&cpp, source).expect("write runnable");

    let include = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("include");
    let bin = temp.path().join("guard_matrix.bin");
    let compile = Command::new(&clang)
        .arg("-std=c++23")
        .arg("-DRUSTY_PORTABLE_INTRINSICS=1")
        .arg("-I")
        .arg(&include)
        .arg(&cpp)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("invoke clang");
    assert!(
        compile.status.success(),
        "guard matrix failed to compile — a consumption context dropped its deref:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin).output().expect("run matrix binary");
    assert!(
        run.status.success(),
        "guard matrix compiled but produced wrong values:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
