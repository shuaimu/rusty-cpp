#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WORK_DIR="${REPO_ROOT}/.rusty-cpp-abi-adapter"

if [[ "${1:-}" == "--work-dir" ]]; then
    [[ $# -eq 2 ]] || { echo "usage: $0 [--work-dir DIR]" >&2; exit 2; }
    WORK_DIR="$2"
elif [[ $# -ne 0 ]]; then
    echo "usage: $0 [--work-dir DIR]" >&2
    exit 2
fi

SOURCE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_core.rs"
SIBLING_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_sibling_crate/Cargo.toml"
ASSERT_EXTERNAL_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_assert_external_crate/Cargo.toml"
ASSERT_BINDING_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_assert_binding_crate/Cargo.toml"
EXTERN_SELF_ALIAS_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_extern_self_alias_crate/Cargo.toml"
EDITION_2015_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_edition_2015_crate/Cargo.toml"
EDITION_OMITTED_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_edition_omitted_crate/Cargo.toml"
EDITION_WORKSPACE_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_edition_workspace_crate/member/Cargo.toml"
MARKER_FREE_2015_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_marker_free_2015_crate/Cargo.toml"
VALID_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_valid_crate/Cargo.toml"
CUSTOM_ROOT_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_custom_root_crate/Cargo.toml"
LIB_BIN_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_lib_bin_crate/Cargo.toml"
SCOPED_SIBLING_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_scoped_sibling_crate/Cargo.toml"
MISSING_MODULE_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_missing_module_crate/Cargo.toml"
MODULE_FILE_CFG_CRATE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_module_file_cfg_crate/Cargo.toml"
BAD_DEP_ROOT="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_dependency_preflight/root/Cargo.toml"
CLOSURE_FIXTURES="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_dependency_closure"
REJECT_FIXTURES="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_reject"
GENERATED="${WORK_DIR}/cpp_abi_core.cppm"
RUST_LIB="${WORK_DIR}/libcpp_abi_core.rlib"
BUILD_DIR="${WORK_DIR}/build"

mkdir -p "${WORK_DIR}"
cargo build -p rusty-cpp-transpiler

for fixture in \
    local_item_fn_shadow \
    local_const_shadow \
    local_static_shadow \
    local_foreign_shadow \
    local_owner_shadow \
    escaped_free_collision \
    escaped_method_collision \
    escaped_owner_collision \
    escaped_alias_collision \
    assert_import_alias \
    assert_macro_use_broad \
    assert_macro_use_selective \
    assert_macro_rules \
    assert_nested_macro \
    file_cfg_false
do
    rustc --edition=2024 --crate-type=lib \
        "${REJECT_FIXTURES}/${fixture}.rs" \
        -o "${WORK_DIR}/${fixture}.rlib"
    REJECT_OUTPUT="${WORK_DIR}/${fixture}.cppm"
    REJECT_LOG="${WORK_DIR}/${fixture}.log"
    if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
        "${REJECT_FIXTURES}/${fixture}.rs" \
        -m "cpp_abi_reject_${fixture}" -o "${REJECT_OUTPUT}" \
        >"${REJECT_LOG}" 2>&1
    then
        echo "cpp_abi reviewer rejection fixture unexpectedly passed: ${fixture}" >&2
        exit 1
    fi
    [[ ! -e "${REJECT_OUTPUT}" ]]
done

ASSERT_BINDING_PARENT="$(mktemp -d "${WORK_DIR}/assert-binding.XXXXXX")"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${ASSERT_BINDING_CRATE}" --output-dir "${ASSERT_BINDING_PARENT}/out" \
    >"${ASSERT_BINDING_PARENT}/failure.log" 2>&1
then
    echo "imported assert macro binding unexpectedly passed crate preflight" >&2
    exit 1
fi
grep -Fq 'binding `assert` is reserved' "${ASSERT_BINDING_PARENT}/failure.log"
[[ ! -e "${ASSERT_BINDING_PARENT}/out" ]]

EXTERN_ALIAS_PARENT="$(mktemp -d "${WORK_DIR}/extern-self-alias.XXXXXX")"
rustc --edition=2024 --crate-type=lib \
    "$(dirname "${EXTERN_SELF_ALIAS_CRATE}")/src/lib.rs" \
    -o "${EXTERN_ALIAS_PARENT}/libextern_self_alias.rlib"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${EXTERN_SELF_ALIAS_CRATE}" --output-dir "${EXTERN_ALIAS_PARENT}/out" \
    >"${EXTERN_ALIAS_PARENT}/failure.log" 2>&1
then
    echo "extern crate self alias unexpectedly passed crate preflight" >&2
    exit 1
fi
grep -Fq 'extern crate' "${EXTERN_ALIAS_PARENT}/failure.log"
[[ ! -e "${EXTERN_ALIAS_PARENT}/out" ]]

for edition_case in \
    "explicit:${EDITION_2015_CRATE}" \
    "omitted:${EDITION_OMITTED_CRATE}" \
    "workspace:${EDITION_WORKSPACE_CRATE}"
do
    case_name="${edition_case%%:*}"
    manifest="${edition_case#*:}"
    EDITION_PARENT="$(mktemp -d "${WORK_DIR}/edition-${case_name}.XXXXXX")"
    rustc --edition=2015 --crate-type=lib \
        "$(dirname "${manifest}")/src/lib.rs" \
        -o "${EDITION_PARENT}/libedition_2015.rlib"
    if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
        --crate "${manifest}" --output-dir "${EDITION_PARENT}/out" \
        >"${EDITION_PARENT}/failure.log" 2>&1
    then
        echo "unsupported cpp_abi edition unexpectedly passed: ${case_name}" >&2
        exit 1
    fi
    grep -Eq 'explicit(ly resolved)? Rust 2018, 2021, or 2024 package.edition' \
        "${EDITION_PARENT}/failure.log"
    [[ ! -e "${EDITION_PARENT}/out" ]]
done

MARKER_FREE_2015_PARENT="$(mktemp -d "${WORK_DIR}/marker-free-2015.XXXXXX")"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${MARKER_FREE_2015_CRATE}" \
    --output-dir "${MARKER_FREE_2015_PARENT}/out"
test -f "${MARKER_FREE_2015_PARENT}/out/cppabi_marker_free_2015_repro.cppm"

for jobs in 1 4; do
    NEGATIVE_PARENT="$(mktemp -d "${WORK_DIR}/sibling-negative-${jobs}.XXXXXX")"
    NEGATIVE_OUTPUT="${NEGATIVE_PARENT}/out"
    NEGATIVE_LOG="${WORK_DIR}/sibling-negative-${jobs}.log"
    if RUSTY_CPP_TRANSPILE_JOBS="${jobs}" \
        "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
        --crate "${SIBLING_CRATE}" --output-dir "${NEGATIVE_OUTPUT}" \
        >"${NEGATIVE_LOG}" 2>&1
    then
        echo "sibling cpp_abi use unexpectedly passed crate preflight" >&2
        exit 1
    fi
    grep -Fq 'cpp_abi crate preflight found a sibling-file reference' "${NEGATIVE_LOG}"
    [[ ! -e "${NEGATIVE_OUTPUT}" ]]
done
cmp -s "${WORK_DIR}/sibling-negative-1.log" "${WORK_DIR}/sibling-negative-4.log"

ASSERT_EXTERNAL_PARENT="$(mktemp -d "${WORK_DIR}/assert-external.XXXXXX")"
rustc --edition=2024 --crate-type=lib \
    "$(dirname "${ASSERT_EXTERNAL_CRATE}")/src/lib.rs" \
    -o "${ASSERT_EXTERNAL_PARENT}/libassert_external.rlib"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${ASSERT_EXTERNAL_CRATE}" --output-dir "${ASSERT_EXTERNAL_PARENT}/out" \
    >"${ASSERT_EXTERNAL_PARENT}/failure.log" 2>&1
then
    echo "assert containing an external cpp_abi call unexpectedly passed" >&2
    exit 1
fi
grep -Fq 'cpp_abi crate preflight found a sibling-file reference' \
    "${ASSERT_EXTERNAL_PARENT}/failure.log"
[[ ! -e "${ASSERT_EXTERNAL_PARENT}/out" ]]

for case_name in missing malformed cycle two_level_invalid; do
    case_root="root"
    [[ "${case_name}" == "cycle" ]] && case_root="a"
    manifest="${CLOSURE_FIXTURES}/${case_name}/${case_root}/Cargo.toml"
    CLOSURE_PARENT="$(mktemp -d "${WORK_DIR}/closure-${case_name}.XXXXXX")"
    CLOSURE_OUTPUT="${CLOSURE_PARENT}/out"
    CLOSURE_LOG="${CLOSURE_PARENT}/failure.log"
    if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
        --crate "${manifest}" --output-dir "${CLOSURE_OUTPUT}" \
        >"${CLOSURE_LOG}" 2>&1
    then
        echo "cpp_abi closure preflight unexpectedly passed: ${case_name}" >&2
        exit 1
    fi
    grep -Fq 'cpp_abi whole local-dependency closure preflight failed before output' \
        "${CLOSURE_LOG}"
    [[ ! -e "${CLOSURE_OUTPUT}" ]]
done

CROSS_CRATE_PARENT="$(mktemp -d "${WORK_DIR}/cross-crate-adapter.XXXXXX")"
CROSS_CRATE_MANIFEST="${CLOSURE_FIXTURES}/cross_crate_adapter/root/Cargo.toml"
CROSS_CRATE_CHECK="${CROSS_CRATE_PARENT}/cargo-check-fixture"
mkdir -p "${CROSS_CRATE_CHECK}"
cp -R "${CLOSURE_FIXTURES}/cross_crate_adapter/." "${CROSS_CRATE_CHECK}/"
CARGO_TARGET_DIR="${CROSS_CRATE_PARENT}/cargo-target" cargo check --quiet \
    --manifest-path "${CROSS_CRATE_CHECK}/root/Cargo.toml"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${CROSS_CRATE_MANIFEST}" --output-dir "${CROSS_CRATE_PARENT}/out" \
    >"${CROSS_CRATE_PARENT}/failure.log" 2>&1
then
    echo "adapter in local dependency unexpectedly passed closure preflight" >&2
    exit 1
fi
grep -Fq 'cross-crate adapter calls are unsupported' "${CROSS_CRATE_PARENT}/failure.log"
[[ ! -e "${CROSS_CRATE_PARENT}/out" ]]

for jobs in 1 4; do
    CLOSURE_PARENT="$(mktemp -d "${WORK_DIR}/closure-jobs-${jobs}.XXXXXX")"
    if RUSTY_CPP_TRANSPILE_JOBS="${jobs}" \
        "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
        --crate "${CLOSURE_FIXTURES}/missing/root/Cargo.toml" \
        --output-dir "${CLOSURE_PARENT}/out" \
        >"${WORK_DIR}/closure-jobs-${jobs}.log" 2>&1
    then
        echo "missing dependency unexpectedly passed with jobs=${jobs}" >&2
        exit 1
    fi
    [[ ! -e "${CLOSURE_PARENT}/out" ]]
done
cmp -s "${WORK_DIR}/closure-jobs-1.log" "${WORK_DIR}/closure-jobs-4.log"

UNREADABLE_PARENT="$(mktemp -d "${WORK_DIR}/closure-unreadable.XXXXXX")"
cp -R "${CLOSURE_FIXTURES}/unreadable/root" "${UNREADABLE_PARENT}/root"
cp -R "${CLOSURE_FIXTURES}/unreadable/bad" "${UNREADABLE_PARENT}/bad"
chmod 000 "${UNREADABLE_PARENT}/bad/Cargo.toml"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${UNREADABLE_PARENT}/root/Cargo.toml" \
    --output-dir "${UNREADABLE_PARENT}/out" \
    >"${UNREADABLE_PARENT}/failure.log" 2>&1
then
    chmod 600 "${UNREADABLE_PARENT}/bad/Cargo.toml"
    echo "unreadable cpp_abi dependency manifest unexpectedly passed" >&2
    exit 1
fi
chmod 600 "${UNREADABLE_PARENT}/bad/Cargo.toml"
grep -Fq 'could not read local dependency manifest' "${UNREADABLE_PARENT}/failure.log"
[[ ! -e "${UNREADABLE_PARENT}/out" ]]

VALID_CLOSURE_PARENT="$(mktemp -d "${WORK_DIR}/closure-valid.XXXXXX")"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${CLOSURE_FIXTURES}/two_level_valid/root/Cargo.toml" \
    --output-dir "${VALID_CLOSURE_PARENT}/out"
test -f "${VALID_CLOSURE_PARENT}/out/cppabi_closure_two_level_valid.cppm"
test -f "${VALID_CLOSURE_PARENT}/out/mid/cppabi_closure_two_level_valid_mid.cppm"
test -f "${VALID_CLOSURE_PARENT}/out/mid/leaf/cppabi_closure_two_level_valid_leaf.cppm"

for case_name in marker_free_missing marker_free_malformed; do
    LEGACY_PARENT="$(mktemp -d "${WORK_DIR}/closure-${case_name}.XXXXXX")"
    "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
        --crate "${CLOSURE_FIXTURES}/${case_name}/root/Cargo.toml" \
        --output-dir "${LEGACY_PARENT}/out" \
        >"${LEGACY_PARENT}/legacy.log" 2>&1
    test -f "${LEGACY_PARENT}/out/cppabi_closure_${case_name}.cppm"
done

SCOPED_PARENT="$(mktemp -d "${WORK_DIR}/scoped-sibling.XXXXXX")"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${SCOPED_SIBLING_CRATE}" --output-dir "${SCOPED_PARENT}/out"
test -f "${SCOPED_PARENT}/out/cppabi_scoped_sibling_repro.sibling.cppm"

for manifest in "${MISSING_MODULE_CRATE}" "${MODULE_FILE_CFG_CRATE}" "${BAD_DEP_ROOT}"
do
    name="$(basename "$(dirname "${manifest}")")"
    NEGATIVE_PARENT="$(mktemp -d "${WORK_DIR}/${name}.XXXXXX")"
    NEGATIVE_OUTPUT="${NEGATIVE_PARENT}/out"
    NEGATIVE_LOG="${WORK_DIR}/${name}.log"
    if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
        --crate "${manifest}" --output-dir "${NEGATIVE_OUTPUT}" \
        >"${NEGATIVE_LOG}" 2>&1
    then
        echo "cpp_abi crate preflight unexpectedly passed: ${manifest}" >&2
        exit 1
    fi
    grep -Fq 'cpp_abi' "${NEGATIVE_LOG}"
    [[ ! -e "${NEGATIVE_OUTPUT}" ]]
done

VALID_PARENT="$(mktemp -d "${WORK_DIR}/valid-crate.XXXXXX")"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${VALID_CRATE}" --output-dir "${VALID_PARENT}/out"
test -f "${VALID_PARENT}/out/cppabi_valid_repro.api.cppm"

EXPAND_PARENT="$(mktemp -d "${WORK_DIR}/expand-negative.XXXXXX")"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${VALID_CRATE}" --expand --output-dir "${EXPAND_PARENT}/out" \
    >"${WORK_DIR}/expand-negative.log" 2>&1
then
    echo "marked --crate --expand unexpectedly passed" >&2
    exit 1
fi
grep -Fq 'does not support --expand' "${WORK_DIR}/expand-negative.log"
[[ ! -e "${EXPAND_PARENT}/out" ]]

CUSTOM_PARENT="$(mktemp -d "${WORK_DIR}/custom-root-negative.XXXXXX")"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${CUSTOM_ROOT_CRATE}" --output-dir "${CUSTOM_PARENT}/out" \
    >"${WORK_DIR}/custom-root-negative.log" 2>&1
then
    echo "marked custom lib root unexpectedly passed" >&2
    exit 1
fi
grep -Fq 'declared target rust/lib.rs' "${WORK_DIR}/custom-root-negative.log"
[[ ! -e "${CUSTOM_PARENT}/out" ]]

LIB_BIN_PARENT="$(mktemp -d "${WORK_DIR}/lib-bin-negative.XXXXXX")"
if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --crate "${LIB_BIN_CRATE}" --output-dir "${LIB_BIN_PARENT}/out" \
    >"${WORK_DIR}/lib-bin-negative.log" 2>&1
then
    echo "marked lib+bin crate unexpectedly passed" >&2
    exit 1
fi
grep -Fq 'cpp_abi requires one conventional source file' "${WORK_DIR}/lib-bin-negative.log"
[[ ! -e "${LIB_BIN_PARENT}/out" ]]

"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    "${SOURCE}" -m cpp_abi_core -o "${GENERATED}"
rustc --edition=2024 --crate-type=lib "${SOURCE}" -o "${RUST_LIB}"

for expected in \
    'export using Weights = std::vector<double>;' \
    'export std::string roundtrip(std::string bytes);' \
    'static std::string encode(uint8_t value);' \
    'std::string Codec::encode(uint8_t value) {' \
    'static uint32_t choose(const Weights& weights);' \
    'uint32_t Picker::choose(const Weights& weights) {' \
    'namespace private_ {' \
    'export using static_ = std::vector<double>;' \
    'export std::string class_(std::string bytes);' \
    'static uint32_t pause(const static_& values);' \
    'uint32_t struct_::pause(const static_& values) {'
do
    grep -Fq "${expected}" "${GENERATED}"
done

cmake -S "${REPO_ROOT}/transpiler/tests/cpp_abi_core" -B "${BUILD_DIR}" -G Ninja \
    -DCMAKE_BUILD_TYPE=Debug \
    -DCMAKE_CXX_COMPILER="${CXX:-clang++}" \
    -DRUSTY_CPP_SOURCE_DIR="${REPO_ROOT}" \
    -DCPP_ABI_GENERATED_MODULE="${GENERATED}"
cmake --build "${BUILD_DIR}" --target cpp_abi_core_runtime -j "${JOBS:-2}"
ctest --test-dir "${BUILD_DIR}" --output-on-failure -R '^cpp_abi_core_runtime$'

MODULE_OBJECT="$(find "${BUILD_DIR}" -path '*cpp_abi_core.cppm.o' -type f -print -quit)"
[[ -n "${MODULE_OBJECT}" ]]
STRONG_SYMBOLS="$(nm -C "${MODULE_OBJECT}" | awk '$2 ~ /^[TDB]$/ { print $0 }')"
[[ "$(printf '%s\n' "${STRONG_SYMBOLS}" | sed '/^$/d' | wc -l)" -eq 6 ]]
printf '%s\n' "${STRONG_SYMBOLS}" | grep -Fq 'initializer for module cpp_abi_core'
printf '%s\n' "${STRONG_SYMBOLS}" | grep -Fq 'roundtrip@cpp_abi_core'
printf '%s\n' "${STRONG_SYMBOLS}" | grep -Fq 'Codec@cpp_abi_core::encode'
printf '%s\n' "${STRONG_SYMBOLS}" | grep -Fq 'Picker@cpp_abi_core::choose'
printf '%s\n' "${STRONG_SYMBOLS}" | grep -Fq 'private_::class_'
printf '%s\n' "${STRONG_SYMBOLS}" | grep -Fq 'private_::struct_@cpp_abi_core::pause'
! printf '%s\n' "${STRONG_SYMBOLS}" | grep -Fq 'rusty_cpp_abi_sem_'

LOCAL_HELPERS="$(nm -C "${MODULE_OBJECT}" | awk '$2 == "t" && /rusty_cpp_abi_sem_/ { print $0 }')"
[[ "$(printf '%s\n' "${LOCAL_HELPERS}" | sed '/^$/d' | wc -l)" -eq 5 ]]

echo "cpp_abi adapter compile/runtime/symbol gate passed"
