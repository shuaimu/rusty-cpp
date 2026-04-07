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

fn compile_and_run_cpp(source: &str, test_name: &str) {
    let compiler = find_cpp_compiler().expect("no C++ compiler found in PATH or CXX");
    let temp = tempfile::tempdir().expect("create temp dir");
    let source_path = temp.path().join(format!("{test_name}.cpp"));
    let bin_path = temp.path().join(format!("{test_name}.bin"));

    std::fs::write(&source_path, source).expect("write C++ source");

    let include_dir = project_include_dir();
    let compile = Command::new(&compiler)
        .arg("-std=c++20")
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

    let run = Command::new(&bin_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "C++ binary failed for {test_name}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}

#[test]
fn test_ptr_read_const_pointer_supports_move_only_payloads() {
    let source = r#"
        #include <rusty/ptr.hpp>
        #include <utility>

        struct MoveOnly {
            int value;
            explicit MoveOnly(int v) : value(v) {}
            MoveOnly(const MoveOnly&) = delete;
            MoveOnly& operator=(const MoveOnly&) = delete;
            MoveOnly(MoveOnly&&) noexcept = default;
            MoveOnly& operator=(MoveOnly&&) noexcept = default;
        };

        int main() {
            MoveOnly payload(7);
            const MoveOnly* ptr = &payload;
            auto moved = rusty::ptr::read(ptr);
            return moved.value == 7 ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "ptr_read_move_only");
}

#[test]
fn test_mem_replace_supports_non_assignable_move_only_payloads() {
    let source = r#"
        #include <rusty/mem.hpp>

        struct NonAssignable {
            int value;
            explicit NonAssignable(int v) : value(v) {}
            NonAssignable(const NonAssignable&) = delete;
            NonAssignable& operator=(const NonAssignable&) = delete;
            NonAssignable(NonAssignable&&) noexcept = default;
            NonAssignable& operator=(NonAssignable&&) = delete;
        };

        int main() {
            NonAssignable dst(1);
            auto old = rusty::mem::replace(dst, NonAssignable(2));
            return (old.value == 1 && dst.value == 2) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "mem_replace_non_assignable");
}

#[test]
fn test_ptr_write_supports_non_assignable_move_only_payloads() {
    let source = r#"
        #include <rusty/ptr.hpp>

        struct NonAssignable {
            int value;
            explicit NonAssignable(int v) : value(v) {}
            NonAssignable() = delete;
            NonAssignable(const NonAssignable&) = delete;
            NonAssignable& operator=(const NonAssignable&) = delete;
            NonAssignable(NonAssignable&&) noexcept = default;
            NonAssignable& operator=(NonAssignable&&) = delete;
        };

        int main() {
            alignas(NonAssignable) unsigned char storage[sizeof(NonAssignable)];
            auto* dst = reinterpret_cast<NonAssignable*>(storage);
            rusty::ptr::write(dst, NonAssignable(11));
            const bool ok = dst->value == 11;
            dst->~NonAssignable();
            return ok ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "ptr_write_non_assignable");
}

#[test]
fn test_result_err_supports_non_default_constructible_error_payloads() {
    let source = r#"
        #include <rusty/result.hpp>

        struct NonDefaultErr {
            int value;
            NonDefaultErr() = delete;
            explicit NonDefaultErr(int v) : value(v) {}
            NonDefaultErr(const NonDefaultErr&) = delete;
            NonDefaultErr& operator=(const NonDefaultErr&) = delete;
            NonDefaultErr(NonDefaultErr&&) noexcept = default;
            NonDefaultErr& operator=(NonDefaultErr&&) noexcept = default;
        };

        int main() {
            auto res = rusty::Result<int, NonDefaultErr>::Err(NonDefaultErr(7));
            if (!res.is_err()) {
                return 1;
            }
            auto err = res.unwrap_err();
            return err.value == 7 ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "result_err_non_default");
}

#[test]
fn test_slice_cloned_iter_supports_move_only_cloneable_payloads() {
    let source = r#"
        #include <array>
        #include <rusty/slice.hpp>

        struct MoveOnlyCloneable {
            int value;
            explicit MoveOnlyCloneable(int v) : value(v) {}
            MoveOnlyCloneable(const MoveOnlyCloneable&) = delete;
            MoveOnlyCloneable& operator=(const MoveOnlyCloneable&) = delete;
            MoveOnlyCloneable(MoveOnlyCloneable&&) noexcept = default;
            MoveOnlyCloneable& operator=(MoveOnlyCloneable&&) noexcept = default;

            MoveOnlyCloneable clone() const { return MoveOnlyCloneable(value); }
        };

        int main() {
            std::array<MoveOnlyCloneable, 2> data{
                MoveOnlyCloneable(7),
                MoveOnlyCloneable(9),
            };
            auto iter = rusty::slice_iter::Iter<const MoveOnlyCloneable>(
                std::span<const MoveOnlyCloneable>(data)
            ).cloned();

            auto first = iter.next();
            if (first.is_none()) {
                return 1;
            }
            auto v = first.unwrap();
            return v.value == 7 ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "slice_cloned_move_only_cloneable");
}

#[test]
fn test_slice_full_vec_of_vec_uses_element_pointer_not_container_pointer() {
    let source = r#"
        #include <rusty/rusty.hpp>
        #include <type_traits>

        int main() {
            auto xs = rusty::Vec<rusty::Vec<int>>::new_();
            xs.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{1, 2})));
            xs.push(rusty::boxed::into_vec(rusty::boxed::box_new(std::array{3})));

            auto span = rusty::slice_full(xs);
            static_assert(
                std::is_same_v<
                    decltype(span),
                    std::span<rusty::Vec<int>>
                >
            );
            return span.size() == 2 ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "slice_full_vec_of_vec_pointer_shape");
}

#[test]
fn test_for_in_zip_temporary_preserves_rvalue_storage_lifetime() {
    let source = r#"
        #include <array>
        #include <rusty/rusty.hpp>

        int main() {
            const auto chars = std::array{U'a', U'α', U'�', U'𐍈'};
            size_t count = 0;
            const auto utf8_len = [](char32_t ch) -> size_t {
                const auto code = static_cast<uint32_t>(ch);
                if (code < 0x80u) {
                    return 1;
                }
                if (code < 0x800u) {
                    return 2;
                }
                if (code < 0x10000u) {
                    return 3;
                }
                return 4;
            };

            for (auto&& [len, ch] : rusty::for_in(rusty::zip((rusty::range_inclusive(1, 4)), chars))) {
                if (static_cast<size_t>(len) != utf8_len(ch)) {
                    return 1;
                }
                ++count;
            }

            return count == 4 ? 0 : 2;
        }
    "#;

    compile_and_run_cpp(source, "for_in_zip_temporary_lifetime");
}
