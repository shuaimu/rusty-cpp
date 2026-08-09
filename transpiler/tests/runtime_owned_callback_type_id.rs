use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_clang() -> Option<String> {
    if let Ok(cxx) = env::var("CXX") {
        if !cxx.trim().is_empty() {
            return Some(cxx);
        }
    }
    for candidate in ["clang++", "clang++-22", "clang++-21"] {
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

#[test]
fn direct_fnmut_erasure_and_type_id_hash_key_compile_and_run() {
    let Some(compiler) = find_clang() else {
        eprintln!("skipping direct callback runtime regression: no clang++ found");
        return;
    };
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_path = temp.path().join("owned_callback_type_id.cpp");
    let bin_path = temp.path().join("owned_callback_type_id.bin");
    let source = r#"
#include <rusty/function.hpp>
#include <rusty/arc.hpp>
#include <rusty/dispatch.hpp>

#include <cstddef>
#include <cstdlib>
#include <memory>
#include <new>
#include <typeindex>
#include <utility>

static std::size_t allocations = 0;

void* operator new(std::size_t size) {
    ++allocations;
    if (void* storage = std::malloc(size)) return storage;
    throw std::bad_alloc();
}

void operator delete(void* storage) noexcept { std::free(storage); }
void operator delete(void* storage, std::size_t) noexcept { std::free(storage); }

struct RegisteredPayload {};

struct DivergentImmovableDefault {
    int value = 17;

    DivergentImmovableDefault() = default;
    explicit DivergentImmovableDefault(int initial) : value(initial) {}
    DivergentImmovableDefault(const DivergentImmovableDefault&) = delete;
    DivergentImmovableDefault(DivergentImmovableDefault&&) = delete;
    DivergentImmovableDefault& operator=(const DivergentImmovableDefault&) = delete;
    DivergentImmovableDefault& operator=(DivergentImmovableDefault&&) = delete;

    static DivergentImmovableDefault default_() {
        return DivergentImmovableDefault(91);
    }
};

template <typename Factory>
struct GetOnlyRegistry {
    Factory stored;

    Factory& get(std::size_t) { return stored; }
    const Factory& get(std::size_t) const { return stored; }
};

int main() {
    auto payload = std::make_unique<int>(40);
    const auto before_factory = allocations;

    using Factory = rusty::Function<int()>;
    Factory factory = Factory(
        [payload = std::move(payload)]() mutable -> int {
            return ++*payload;
        });

    // The direct generated shape keeps this small move-only closure in the
    // Function SBO. A Box<std::function<...>>::new_(lambda) adapter would
    // necessarily allocate before Function sees it.
    if (allocations != before_factory || !factory.is_inline()) return 1;
    if (factory() != 41 || factory() != 42) return 2;

    // rusty::HashMap spells Rust's mutable lookup as the non-const overload of
    // `get`; there intentionally is no C++ `get_mut` member. Keep an FnMut in
    // that get-only surface so this also proves the selected reference remains
    // mutable and callable after lookup.
    GetOnlyRegistry<Factory> by_name{std::move(factory)};
    auto& looked_up = by_name.get(7);
    if (looked_up() != 43 || looked_up() != 44) return 3;

    const std::type_index type_id(typeid(RegisteredPayload));
    const std::size_t registry_key = (type_id).hash_code();
    if (registry_key != std::type_index(typeid(RegisteredPayload)).hash_code()) return 4;

    // The reserved intrinsic emits this exact zero-argument call. It must
    // construct T in place (17), not call T::default_() (91), and must compile
    // even though T cannot be copied or moved.
    auto default_payload = rusty::Arc<DivergentImmovableDefault>::make();
    if (default_payload->value != 17) return 5;
    if (rusty::default_like<DivergentImmovableDefault>().value != 91) return 6;
    return 0;
}
"#;
    std::fs::write(&source_path, source).expect("write C++ source");

    let compile = Command::new(&compiler)
        .arg("-std=c++23")
        .arg("-I")
        .arg(project_include_dir())
        .arg(&source_path)
        .arg("-o")
        .arg(&bin_path)
        .output()
        .expect("invoke clang++");
    assert!(
        compile.status.success(),
        "C++ compile failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&bin_path)
        .output()
        .expect("run C++ regression binary");
    assert!(
        run.status.success(),
        "C++ runtime regression failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        run.status.code(),
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
