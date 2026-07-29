use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_cpp_compiler() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    for candidate in ["c++", "g++", "clang++"] {
        let status = Command::new(candidate)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

fn project_include_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("include")
}

fn transpiler_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_BIN_EXE_rusty-cpp-transpiler"));
    if !p.exists() {
        p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/release/rusty-cpp-transpiler");
    }
    p
}

/// Transpile a Rust source and compile+run the emitted C++.
///
/// Single-file mode emits PLAIN C++ with the runtime helper preamble
/// inlined, so this exercises the real consumption path — the one place
/// `rusty::time` is defined. A header-level test cannot reach it.
fn transpile_compile_run(rust_source: &str, test_name: &str) {
    let temp = tempfile::tempdir().expect("create temp dir");
    let rs_path = temp.path().join(format!("{test_name}.rs"));
    let cpp_path = temp.path().join(format!("{test_name}.cpp"));
    std::fs::write(&rs_path, rust_source).expect("write rust source");

    let out = Command::new(transpiler_bin())
        .arg(rs_path.to_str().unwrap())
        .arg("-o")
        .arg(cpp_path.to_str().unwrap())
        .output()
        .expect("run transpiler");
    assert!(
        out.status.success(),
        "transpile failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let cpp = std::fs::read_to_string(&cpp_path).expect("read emitted C++");
    compile_and_run_cpp(&cpp, test_name);
}

fn transpile_compile_run_with_main(rust_source: &str, test_name: &str) {
    let temp = tempfile::tempdir().expect("create temp dir");
    let rs_path = temp.path().join(format!("{test_name}.rs"));
    let cpp_path = temp.path().join(format!("{test_name}.cpp"));
    std::fs::write(&rs_path, rust_source).expect("write rust source");

    let out = Command::new(transpiler_bin())
        .arg(rs_path.to_str().unwrap())
        .arg("-o")
        .arg(cpp_path.to_str().unwrap())
        .output()
        .expect("run transpiler");
    assert!(
        out.status.success(),
        "transpile failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let mut cpp = std::fs::read_to_string(&cpp_path).expect("read emitted C++");
    cpp.push_str("\nint main() { return check_time(); }\n");
    compile_and_run_cpp(&cpp, test_name);
}

fn compile_and_run_cpp(source: &str, test_name: &str) {
    let compiler = find_cpp_compiler().expect("no C++ compiler found in PATH or CXX");
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_path = temp.path().join(format!("{test_name}.cpp"));
    let bin_path = temp.path().join(format!("{test_name}.bin"));

    std::fs::write(&source_path, source).expect("write C++ source");

    let include_dir = project_include_dir();
    let compile = Command::new(&compiler)
        .arg("-std=c++23")
        .arg("-I")
        .arg(&include_dir)
        .arg(&source_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke C++ compiler");

    assert!(
        compile.status.success(),
        "C++ compile failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin_path)
        .output()
        .expect("run compiled binary");
    assert!(
        run.status.success(),
        "C++ binary failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}


/// `rusty::time` — the std::time surface transpiled crates actually use.
///
/// The runtime helper block shipped a STUB (from_secs/millis/nanos,
/// as_secs, subsec_nanos, comparisons; Instant::now/duration_since):
/// no `elapsed()`, no as_micros/as_millis, no arithmetic, no
/// checked/saturating forms — so ordinary timeout/TTL/deadline code
/// could not translate at all. This pins the completed surface by
/// running it.
#[test]
fn test_std_time_surface_translates_and_runs() {
    // A non-zero return names the failing group, so a regression says
    // WHICH part of the surface broke rather than just "exit 1".
    let source = r#"
pub fn check_time() -> i32 {
    let mut fails: i32 = 0;

    // constructors / accessors
    if std::time::Duration::from_secs(2).as_secs() != 2 { fails += 1; }
    if std::time::Duration::from_millis(1500).as_millis() != 1500 { fails += 2; }
    if std::time::Duration::from_micros(1500).as_micros() != 1500 { fails += 4; }
    if std::time::Duration::from_nanos(42).as_nanos() != 42 { fails += 8; }
    if std::time::Duration::from_millis(1500).subsec_millis() != 500 { fails += 16; }

    // arithmetic
    let a = std::time::Duration::from_millis(300);
    let b = std::time::Duration::from_millis(200);
    if (a + b).as_millis() != 500 { fails += 32; }
    if (a - b).as_millis() != 100 { fails += 64; }

    // Duration is UNSIGNED in Rust: underflow saturates, never wraps.
    if !b.saturating_sub(a).is_zero() { fails += 128; }
    if a.saturating_sub(b).as_millis() != 100 { fails += 256; }
    if !(a > b) { fails += 512; }

    // Instant: elapsed / ordering / backwards handling
    let t0 = std::time::Instant::now();
    let t1 = std::time::Instant::now();
    if !t0.saturating_duration_since(t1).is_zero() { fails += 1024; }
    if t1.duration_since(t0).as_nanos() >= 1000000000 { fails += 2048; }
    if t0.elapsed().as_nanos() >= 1000000000 { fails += 4096; }
    if ((t0 + std::time::Duration::from_secs(1)) - t0).as_secs() != 1 { fails += 8192; }

    // SystemTime against the epoch
    let now = std::time::SystemTime::now();
    if !now.duration_since(std::time::UNIX_EPOCH).is_ok() { fails += 16384; }

    fails
}
"#;

    // Single-file mode emits a plain-C++ translation unit with no entry
    // point; give it one that propagates the failure bitmask.
    transpile_compile_run_with_main(source, "std_time_surface");
}

/// The member-dispatch overloads added to the free saturating_* helpers
/// must not disturb the integral kernels they were guarding.
#[test]
fn test_integral_saturating_helpers_still_saturate() {
    let source = r#"
        #include <rusty/array.hpp>
        #include <cstdint>

        int main() {
            if (rusty::saturating_sub<std::uint8_t, std::uint8_t>(0, 1) != 0) return 1;
            if (rusty::saturating_add<std::uint8_t, std::uint8_t>(255, 1) != 255) return 2;
            if (rusty::saturating_sub(5, 3) != 2) return 3;
            if (rusty::saturating_mul<std::uint8_t, std::uint8_t>(200, 2) != 255) return 4;
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "integral_saturating");
}

/// `fetch_min` / `fetch_max` / `fetch_update` — std atomics with no
/// `std::atomic` counterpart, so each is a CAS loop. srpc's connection
/// metrics need all three: min/max latency, and a saturating decrement
/// of the in-flight gauge.
#[test]
fn test_atomic_fetch_min_max_update() {
    let source = r#"
        #include <rusty/sync/atomic.hpp>
        #include <rusty/option.hpp>
        #include <cstdint>

        using rusty::sync::atomic::Ordering;

        int main() {
            rusty::sync::atomic::AtomicU64 a(10);
            if (a.fetch_min(3, Ordering::Relaxed) != 10) return 1;   // returns PREVIOUS
            if (a.load(Ordering::Relaxed) != 3) return 2;
            if (a.fetch_min(7, Ordering::Relaxed) != 3) return 3;    // larger: no store
            if (a.load(Ordering::Relaxed) != 3) return 4;
            if (a.fetch_max(9, Ordering::Relaxed) != 3) return 5;
            if (a.load(Ordering::Relaxed) != 9) return 6;
            if (a.fetch_max(1, Ordering::Relaxed) != 9) return 7;    // smaller: no store
            if (a.load(Ordering::Relaxed) != 9) return 8;

            auto ok = a.fetch_update(Ordering::Relaxed, Ordering::Relaxed,
                [](std::uint64_t v) { return rusty::Option<std::uint64_t>(v * 2); });
            if (!ok.is_ok() || a.load(Ordering::Relaxed) != 18) return 9;

            // None declines the update and reports Err.
            auto declined = a.fetch_update(Ordering::Relaxed, Ordering::Relaxed,
                [](std::uint64_t) { return rusty::Option<std::uint64_t>(rusty::None); });
            if (!declined.is_err() || a.load(Ordering::Relaxed) != 18) return 10;

            // The saturating-decrement shape: must not wrap at zero.
            rusty::sync::atomic::AtomicU64 gauge(0);
            (void)gauge.fetch_update(Ordering::Relaxed, Ordering::Relaxed,
                [](std::uint64_t v) {
                    return rusty::Option<std::uint64_t>(v == 0 ? 0 : v - 1);
                });
            if (gauge.load(Ordering::Relaxed) != 0) return 11;
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "atomic_fetch_min_max_update");
}
