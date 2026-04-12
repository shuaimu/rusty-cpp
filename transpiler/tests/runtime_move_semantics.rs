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
fn test_array_eq_supports_as_slice_containers_and_vec() {
    let source = r#"
        #include <array>
        #include <cstddef>
        #include <span>
        #include <rusty/array.hpp>
        #include <rusty/vec.hpp>

        struct SliceLike {
            std::array<std::size_t, 1> data{0};
            auto as_slice() const {
                return std::span<const std::size_t>(data.data(), data.size());
            }
        };

        int main() {
            const SliceLike s{};
            if (!(s == std::array{0})) {
                return 1;
            }
            if (!(std::array{0} == s)) {
                return 2;
            }
            const auto expected_span = std::span<const std::size_t>(s.data.data(), s.data.size());
            if (!(s == expected_span)) {
                return 3;
            }
            if (!(expected_span == s)) {
                return 4;
            }

            auto v = rusty::Vec<unsigned char>::new_();
            v.push(static_cast<unsigned char>(3));
            if (!(v == std::array{3})) {
                return 5;
            }
            if (!(std::array{3} == v)) {
                return 6;
            }
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "array_eq_as_slice_and_vec");
}

#[test]
fn test_vec_eq_supports_cross_numeric_element_types() {
    let source = r#"
        #include <cstdint>
        #include <rusty/vec.hpp>

        int main() {
            auto bytes = rusty::Vec<std::uint8_t>::new_();
            bytes.push(static_cast<std::uint8_t>(0));
            bytes.push(static_cast<std::uint8_t>(1));

            auto ints = rusty::Vec<int>::new_();
            ints.push(0);
            ints.push(1);

            if (!(bytes == ints)) {
                return 1;
            }
            if (!(ints == bytes)) {
                return 2;
            }

            auto mismatch = rusty::Vec<int>::new_();
            mismatch.push(0);
            mismatch.push(2);
            if (bytes == mismatch) {
                return 3;
            }

            return 0;
        }
    "#;

    compile_and_run_cpp(source, "vec_eq_cross_numeric_types");
}

#[test]
fn test_result_ok_supports_cross_numeric_array_literal_conversion() {
    let source = r#"
        #include <array>
        #include <cstdint>
        #include <rusty/result.hpp>

        int main() {
            using R = rusty::Result<std::array<std::uint8_t, 2>, int>;
            auto ok = R::Ok(std::array{0, 1});
            if (!ok.is_ok()) {
                return 1;
            }
            const auto& payload = ok.unwrap();
            if (payload[0] != static_cast<std::uint8_t>(0)) {
                return 2;
            }
            if (payload[1] != static_cast<std::uint8_t>(1)) {
                return 3;
            }
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "result_ok_array_numeric_convert");
}

#[test]
fn test_rc_new_and_static_clone_surface() {
    let source = r#"
        #include <rusty/rc.hpp>

        int main() {
            auto one = rusty::Rc<int>::new_(1);
            auto two = rusty::Rc<int>::clone(one);
            if (one.strong_count() != 2) {
                return 1;
            }
            if (two.strong_count() != 2) {
                return 2;
            }
            if (*one != 1 || *two != 1) {
                return 3;
            }
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "rc_new_static_clone_surface");
}

#[test]
fn test_slice_scan_runtime_adapter_surface() {
    let source = r#"
        #include <array>
        #include <rusty/array.hpp>
        #include <rusty/slice.hpp>

        int main() {
            std::array<int, 4> values{1, 2, 3, 4};
            auto it = rusty::scan(
                rusty::iter(values),
                0,
                [](int& state, int value) -> rusty::Option<int> {
                    state += value;
                    if (state > 5) {
                        return rusty::None;
                    }
                    return rusty::Option<int>(state);
                });

            auto first = it.next();
            if (!first.is_some() || first.unwrap() != 1) {
                return 1;
            }

            auto second = it.next();
            if (!second.is_some() || second.unwrap() != 3) {
                return 2;
            }

            auto third = it.next();
            if (!third.is_none()) {
                return 3;
            }

            auto after_none = it.next();
            if (!after_none.is_none()) {
                return 4;
            }

            return 0;
        }
    "#;

    compile_and_run_cpp(source, "slice_scan_runtime_surface");
}

#[test]
fn test_slice_filter_runtime_adapter_surface() {
    let source = r#"
        #include <rusty/array.hpp>
        #include <rusty/slice.hpp>

        int main() {
            auto iter = rusty::filter(
                rusty::range(0, 5),
                [](int n) { return n % 2 == 0; });

            const auto hint = iter.size_hint();
            if (hint._0 != 0) {
                return 1;
            }

            auto first = iter.next();
            if (!first.has_value() || *first != 0) {
                return 2;
            }

            auto second = iter.next();
            if (!second.has_value() || *second != 2) {
                return 3;
            }

            auto third = iter.next();
            if (!third.has_value() || *third != 4) {
                return 4;
            }

            auto done = iter.next();
            if (done.has_value()) {
                return 5;
            }

            return 0;
        }
    "#;

    compile_and_run_cpp(source, "slice_filter_runtime_surface");
}

#[test]
fn test_slice_get_runtime_helper_surface() {
    let source = r#"
        #include <array>
        #include <rusty/array.hpp>

        int main() {
            std::array<int, 3> values{4, 5, 6};

            auto mid = rusty::get(values, 1);
            if (!mid.is_some() || mid.unwrap() != 5) {
                return 1;
            }

            auto miss = rusty::get(values, 99);
            if (!miss.is_none()) {
                return 2;
            }

            return 0;
        }
    "#;

    compile_and_run_cpp(source, "slice_get_runtime_surface");
}

#[test]
fn test_array_type_level_size_helper_surface() {
    let source = r#"
        #include <array>
        #include <rusty/array.hpp>

        int main() {
            constexpr size_t n = rusty::detail::type_level_size<std::array<int, 3>>();
            static_assert(n == 3, "type_level_size should use tuple_size for std::array");
            if (n != 3) {
                return 1;
            }
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "array_type_level_size_surface");
}

#[test]
fn test_mem_forgotten_address_tracking_counts_repeated_marks() {
    let source = r#"
        #include <rusty/mem.hpp>

        int main() {
            int value = 0;
            const void* addr = &value;

            rusty::mem::mark_forgotten_address(addr);
            rusty::mem::mark_forgotten_address(addr);

            if (!rusty::mem::consume_forgotten_address(addr)) {
                return 1;
            }
            if (!rusty::mem::consume_forgotten_address(addr)) {
                return 2;
            }
            if (rusty::mem::consume_forgotten_address(addr)) {
                return 3;
            }
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "mem_forgotten_address_refcount");
}

#[test]
fn test_mem_forgotten_address_storage_survives_global_destructor_calls() {
    let source = r#"
        #include <cstdlib>
        #include <rusty/mem.hpp>

        struct ExitProbe {
            ~ExitProbe() noexcept {
                rusty::mem::mark_forgotten_address(this);
                if (!rusty::mem::consume_forgotten_address(this)) {
                    std::abort();
                }
            }
        };

        static ExitProbe PROBE;

        int main() {
            int value = 0;
            const void* addr = &value;
            rusty::mem::mark_forgotten_address(addr);
            rusty::mem::consume_forgotten_address(addr);
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "mem_forgotten_address_static_exit");
}

#[test]
fn test_mem_drop_allows_unwind_catch_for_panicking_destructors() {
    let source = r#"
        #include <rusty/mem.hpp>
        #include <rusty/panic.hpp>

        struct PanicOnDrop {
            PanicOnDrop() = default;
            PanicOnDrop(const PanicOnDrop&) = default;
            PanicOnDrop(PanicOnDrop&& other) noexcept {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    rusty::mem::mark_forgotten_address(this);
                    rusty::mem::mark_forgotten_address(&other);
                } else {
                    rusty::mem::mark_forgotten_address(&other);
                }
            }

            void rusty_mark_forgotten() noexcept {
                rusty::mem::mark_forgotten_address(this);
            }

            ~PanicOnDrop() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) {
                    return;
                }
                rusty::panic::begin_panic("drop");
            }
        };

        int main() {
            PanicOnDrop value{};
            auto res = rusty::panic::catch_unwind(rusty::panic::AssertUnwindSafe([&]() {
                rusty::mem::drop(std::move(value));
            }));
            return res.is_err() ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "mem_drop_unwind_catch");
}

#[test]
fn test_vec_drop_panic_is_catchable_via_catch_unwind() {
    let source = r#"
        #include <rusty/rusty.hpp>

        struct Bump {
            const rusty::Cell<int>& flag;
            explicit Bump(const rusty::Cell<int>& flag_ref) : flag(flag_ref) {}
            Bump(const Bump&) = default;
            Bump(Bump&& other) noexcept : flag(other.flag) {
                if (rusty::mem::consume_forgotten_address(&other)) {
                    rusty::mem::mark_forgotten_address(this);
                    rusty::mem::mark_forgotten_address(&other);
                } else {
                    rusty::mem::mark_forgotten_address(&other);
                }
            }

            void rusty_mark_forgotten() noexcept {
                rusty::mem::mark_forgotten_address(this);
            }

            ~Bump() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) {
                    return;
                }
                const auto n = flag.get();
                flag.set(n + 1);
                if (n == 0) {
                    rusty::panic::begin_panic("drop");
                }
            }
        };

        int main() {
            const auto& flag = rusty::Cell<int>::new_(0);
            auto v = rusty::Vec<Bump>::new_();
            v.push(Bump(flag));
            v.push(Bump(flag));

            auto res = rusty::panic::catch_unwind(rusty::panic::AssertUnwindSafe([&]() {
                rusty::mem::drop(std::move(v));
            }));
            return res.is_err() ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "vec_drop_panic_catch_unwind");
}

#[test]
fn test_catch_unwind_accepts_plain_callable_without_assert_wrapper() {
    let source = r#"
        #include <rusty/panic.hpp>

        int main() {
            int value = 0;
            auto res = rusty::panic::catch_unwind([=]() mutable {
                value += 1;
                rusty::panic::begin_panic("boom");
            });
            return res.is_err() ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "catch_unwind_plain_callable");
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
fn test_ptr_copy_nonoverlapping_supports_char_to_u8_surface() {
    let source = r#"
        #include <array>
        #include <rusty/ptr.hpp>

        int main() {
            const char src[4] = {'a', 'b', 'c', '\0'};
            std::array<uint8_t, 4> dst{};
            rusty::ptr::copy_nonoverlapping(src, dst.data(), 4);
            return (dst[0] == static_cast<uint8_t>('a') && dst[2] == static_cast<uint8_t>('c')) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "ptr_copy_nonoverlapping_char_u8");
}

#[test]
fn test_ptr_nonnull_supports_equality_comparison() {
    let source = r#"
        #include <rusty/ptr.hpp>

        int main() {
            uint8_t data = 7;
            auto a = rusty::ptr::NonNull<uint8_t>::new_unchecked(&data);
            auto b = rusty::ptr::NonNull<uint8_t>::new_unchecked(&data);
            return (a == b) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "ptr_nonnull_equality");
}

#[test]
fn test_as_ptr_const_value_supports_nonconst_as_ptr_const_pointer_surface() {
    let source = r#"
        #include <rusty/array.hpp>

        struct Wrapper {
            int value;
            const int* as_ptr() { return &value; }
        };

        int main() {
            const Wrapper wrapped{7};
            auto ptr = rusty::as_ptr(wrapped);
            return (*ptr == 7) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "as_ptr_const_nonconst_method_const_pointer");
}

#[test]
fn test_as_ref_ptr_balances_pointer_wrappers_and_string_like_values() {
    let source = r#"
        #include <rusty/array.hpp>
        #include <rusty/string.hpp>

        struct Wrapper {
            int value;
            const int* as_ptr() { return &value; }
        };

        int main() {
            const Wrapper wrapped{7};
            auto wrapped_ptr = rusty::as_ref_ptr(wrapped);
            if (*wrapped_ptr != 7) {
                return 1;
            }

            rusty::String text = rusty::String::from("hi");
            auto text_ptr = rusty::as_ref_ptr(text);
            if (!(*text_ptr == "hi")) {
                return 2;
            }
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "as_ref_ptr_wrapper_and_string_balance");
}

#[test]
fn test_mem_transmute_supports_equal_size_byte_reinterpretation() {
    let source = r#"
        #include <array>
        #include <cstdint>
        #include <rusty/mem.hpp>

        struct Pair {
            uint8_t a;
            uint8_t b;
        };

        int main() {
            std::array<uint8_t, 2> bytes{5, 9};
            auto pair = rusty::mem::transmute<std::array<uint8_t, 2>, Pair>(bytes);
            return (pair.a == 5 && pair.b == 9) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "mem_transmute_equal_size");
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

#[test]
fn test_to_string_view_prefers_deref_over_recursive_as_str() {
    let source = r#"
        #include <rusty/rusty.hpp>
        #include <string_view>

        struct RecursiveAsStr {
            std::string_view as_str() const {
                return rusty::to_string_view(*this);
            }

            std::string_view operator*() const {
                return std::string_view("ok");
            }
        };

        int main() {
            const RecursiveAsStr value{};
            const auto view = rusty::to_string_view(value);
            return view == "ok" ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "to_string_view_recursive_as_str");
}

#[test]
fn test_mem_size_of_uses_rust_layout_for_arrayvec_like_zero_capacity_storage() {
    let source = r#"
        #include <array>
        #include <cstdint>
        #include <rusty/maybe_uninit.hpp>
        #include <rusty/mem.hpp>

        template<size_t CAP>
        struct ArrayVecLike {
            static constexpr size_t CAPACITY = CAP;
            uint32_t len_field;
            std::array<rusty::MaybeUninit<uint8_t>, CAP> xs;
        };

        struct Plain {
            uint32_t a;
            uint8_t b;
        };

        int main() {
            const bool zero_capacity_matches_rust =
                rusty::mem::size_of<ArrayVecLike<0>>() == sizeof(uint32_t);
            const bool non_zero_capacity_matches_len_plus_elements =
                rusty::mem::size_of<ArrayVecLike<4>>() == sizeof(uint32_t) + 4 * sizeof(uint8_t);
            const bool fallback_uses_native_size =
                rusty::mem::size_of<Plain>() == sizeof(Plain);
            return (zero_capacity_matches_rust &&
                    non_zero_capacity_matches_len_plus_elements &&
                    fallback_uses_native_size) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "mem_size_of_rust_layout_override");
}

#[test]
fn test_string_repeat_supports_zero_and_overflow_guard() {
    let source = r#"
        #include <limits>
        #include <rusty/string.hpp>
        #include <stdexcept>

        int main() {
            const auto seed = rusty::String::from("ab");

            const auto repeated = seed.repeat(3);
            if (!(repeated == "ababab")) {
                return 1;
            }
            if (!(seed == "ab")) {
                return 2;
            }

            const auto zero = seed.repeat(0);
            if (!zero.is_empty()) {
                return 3;
            }

            bool overflow_guard_triggered = false;
            try {
                (void)seed.repeat(std::numeric_limits<size_t>::max());
            } catch (const std::length_error&) {
                overflow_guard_triggered = true;
            }
            return overflow_guard_triggered ? 0 : 4;
        }
    "#;

    compile_and_run_cpp(source, "string_repeat_zero_and_overflow_guard");
}

#[test]
fn test_default_value_prefers_empty_for_non_default_constructible_types() {
    let source = r#"
        #include <rusty/rusty.hpp>

        struct NonDefaultWithEmpty {
            int value;
            explicit NonDefaultWithEmpty(int v) : value(v) {}
            NonDefaultWithEmpty() = delete;
            static NonDefaultWithEmpty empty() { return NonDefaultWithEmpty(41); }
        };

        int main() {
            const auto v = rusty::default_value<NonDefaultWithEmpty>();
            return v.value == 41 ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(
        source,
        "default_value_non_default_constructible_empty_fallback",
    );
}

#[test]
fn test_len_supports_as_str_wrappers_without_size_surface() {
    let source = r#"
        #include <rusty/rusty.hpp>
        #include <string_view>

        struct AsStrOnly {
            std::string_view text;
            std::string_view as_str() const { return text; }
        };

        int main() {
            const AsStrOnly pre{"alpha"};
            const AsStrOnly empty{""};
            return (rusty::len(pre) == 5 && rusty::len(empty) == 0) ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "len_as_str_wrapper_fallback");
}

#[test]
fn test_mem_forget_marks_const_values_with_rusty_drop_guard() {
    let source = r#"
        #include <rusty/mem.hpp>

        struct GuardedDrop {
            static inline int drop_count = 0;
            void rusty_mark_forgotten() noexcept {
                rusty::mem::mark_forgotten_address(this);
            }
            ~GuardedDrop() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) {
                    return;
                }
                ++drop_count;
            }
        };

        int main() {
            {
                const auto value = GuardedDrop{};
                rusty::mem::forget(std::move(value));
            }
            return GuardedDrop::drop_count == 0 ? 0 : 1;
        }
    "#;

    compile_and_run_cpp(source, "mem_forget_const_guarded_drop");
}

#[test]
fn test_mem_forget_const_prevents_is_empty_destructor_recursion_shape() {
    let source = r#"
        #include <rusty/mem.hpp>

        struct RecursiveDrop {
            int tag;
            explicit RecursiveDrop(int v) : tag(v) {}

            static RecursiveDrop empty() { return RecursiveDrop(-1); }

            bool is_empty() const {
                const auto empty_value = RecursiveDrop::empty();
                const bool eq = (tag == empty_value.tag);
                rusty::mem::forget(std::move(empty_value));
                return eq;
            }

            void rusty_mark_forgotten() noexcept {
                rusty::mem::mark_forgotten_address(this);
            }

            ~RecursiveDrop() noexcept(false) {
                if (rusty::mem::consume_forgotten_address(this)) {
                    return;
                }
                if (is_empty()) {
                    return;
                }
            }
        };

        int main() {
            RecursiveDrop value(5);
            (void)value.tag;
            return 0;
        }
    "#;

    compile_and_run_cpp(source, "mem_forget_const_recursion_shape");
}
