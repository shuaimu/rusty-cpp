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
INLINE_SOURCE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_abi_inline.cppm"
FLAT_IMPORT_INLINE_SOURCE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_import_namespace_inline.cppm"
FLAT_IMPORT_CRATE_SOURCE="${REPO_ROOT}/transpiler/tests/fixtures/cpp_import_namespace_crate"
MARKER_FREE_INLINE_SOURCE="${REPO_ROOT}/transpiler/tests/fixtures/inline_rust_marker_free.cppm"
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
INLINE_GENERATED="${WORK_DIR}/cpp_abi_inline.cppm"
INLINE_RUST="${WORK_DIR}/cpp_abi_inline.rs"
FLAT_IMPORT_INLINE_OUTPUT="${WORK_DIR}/cpp_import_namespace_inline"
FLAT_IMPORT_INLINE_GENERATED="${FLAT_IMPORT_INLINE_OUTPUT}/rrr.inline_consumer.cppm"
FLAT_IMPORT_INLINE_RUST="${FLAT_IMPORT_INLINE_OUTPUT}/rrr.inline_consumer.rs"
FLAT_IMPORT_INLINE_SELECTED_RUST="${FLAT_IMPORT_INLINE_OUTPUT}/rrr.inline_unrelated.rs"
FLAT_IMPORT_CRATE_OUTPUT="${WORK_DIR}/cpp_import_namespace_crate"
FLAT_IMPORT_CRATE_MODULE="${FLAT_IMPORT_CRATE_OUTPUT}/rrr.request_options.cppm"
MARKER_FREE_INLINE_GENERATED="${WORK_DIR}/inline_rust_marker_free.cppm"
RUST_LIB="${WORK_DIR}/libcpp_abi_core.rlib"
BUILD_DIR="${WORK_DIR}/build"

mkdir -p "${WORK_DIR}" "${FLAT_IMPORT_INLINE_OUTPUT}"
FLAT_IMPORT_CRATE_INPUT="$(mktemp -d "${WORK_DIR}/cpp-import-namespace-input.XXXXXX")"
cp -R "${FLAT_IMPORT_CRATE_SOURCE}/." "${FLAT_IMPORT_CRATE_INPUT}/"
FLAT_IMPORT_CRATE="${FLAT_IMPORT_CRATE_INPUT}/Cargo.toml"
cargo build -p rusty-cpp-transpiler

cp "${INLINE_SOURCE}" "${INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --rewrite --files "${INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --check --files "${INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --emit-rust "${INLINE_RUST}" --files "${INLINE_GENERATED}"
rustc --edition=2024 --crate-type=lib "${INLINE_RUST}" \
    -o "${WORK_DIR}/libcpp_abi_inline.rlib"

CARGO_TARGET_DIR="${WORK_DIR}/flat-import-cargo-target" \
    cargo check --quiet --manifest-path "${FLAT_IMPORT_CRATE}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" \
    --cxx-namespace rrr \
    --crate "${FLAT_IMPORT_CRATE}" \
    --output-dir "${FLAT_IMPORT_CRATE_OUTPUT}"
test -f "${FLAT_IMPORT_CRATE_MODULE}"
grep -Fq 'import rrr.rand;' "${FLAT_IMPORT_CRATE_MODULE}"
grep -Fq 'namespace rrr {' "${FLAT_IMPORT_CRATE_MODULE}"
grep -Fq 'randgen_rand_raw()' "${FLAT_IMPORT_CRATE_MODULE}"
! grep -Fq 'using ::rrr::' "${FLAT_IMPORT_CRATE_MODULE}"
! grep -Fq 'namespace rand =' "${FLAT_IMPORT_CRATE_MODULE}"
! grep -Fq '::rrr::rand::' "${FLAT_IMPORT_CRATE_MODULE}"
FLAT_CRATE_MODULE_LINE="$(grep -nF 'export module rrr.request_options;' \
    "${FLAT_IMPORT_CRATE_MODULE}" | cut -d: -f1)"
FLAT_CRATE_IMPORT_LINE="$(grep -nF 'import rrr.rand;' \
    "${FLAT_IMPORT_CRATE_MODULE}" | cut -d: -f1)"
FLAT_CRATE_NAMESPACE_LINE="$(grep -nF 'namespace rrr {' \
    "${FLAT_IMPORT_CRATE_MODULE}" | head -n1 | cut -d: -f1)"
(( FLAT_CRATE_MODULE_LINE < FLAT_CRATE_IMPORT_LINE &&
   FLAT_CRATE_IMPORT_LINE < FLAT_CRATE_NAMESPACE_LINE ))
grep -Fq '0 slot(s)' "${FLAT_IMPORT_CRATE_OUTPUT}/rusty_hand_slots.md"

cp "${FLAT_IMPORT_INLINE_SOURCE}" "${FLAT_IMPORT_INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --rewrite --files "${FLAT_IMPORT_INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --check --files "${FLAT_IMPORT_INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --emit-rust "${FLAT_IMPORT_INLINE_RUST}" \
    --files "${FLAT_IMPORT_INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --emit-rust "${FLAT_IMPORT_INLINE_SELECTED_RUST}" \
    --files "${FLAT_IMPORT_INLINE_GENERATED}" \
    --block-id rrr.inline_unrelated
! grep -Fq 'cpp_import_namespace' "${FLAT_IMPORT_INLINE_SELECTED_RUST}"
! grep -Fq 'randgen_rand_' "${FLAT_IMPORT_INLINE_SELECTED_RUST}"
rustc --edition=2024 --crate-type=lib --crate-name rrr_inline_unrelated -Dwarnings \
    "${FLAT_IMPORT_INLINE_SELECTED_RUST}" \
    -o "${FLAT_IMPORT_INLINE_OUTPUT}/librrr_inline_unrelated.rlib"
grep -Fq 'import rrr.rand;' "${FLAT_IMPORT_INLINE_GENERATED}"
grep -Fq 'randgen_rand_raw()' "${FLAT_IMPORT_INLINE_GENERATED}"
! grep -Fq 'using ::rrr::' "${FLAT_IMPORT_INLINE_GENERATED}"
! grep -Fq 'namespace rand =' "${FLAT_IMPORT_INLINE_GENERATED}"
! grep -Fq 'rusty_cpp_abi_' "${FLAT_IMPORT_INLINE_GENERATED}"
FLAT_INLINE_MODULE_LINE="$(grep -nF 'export module rrr.inline_consumer;' \
    "${FLAT_IMPORT_INLINE_GENERATED}" | cut -d: -f1)"
FLAT_INLINE_IMPORT_LINE="$(grep -nF 'import rrr.rand;' \
    "${FLAT_IMPORT_INLINE_GENERATED}" | cut -d: -f1)"
FLAT_INLINE_NAMESPACE_LINE="$(grep -nF 'export namespace rrr {' \
    "${FLAT_IMPORT_INLINE_GENERATED}" | cut -d: -f1)"
(( FLAT_INLINE_MODULE_LINE < FLAT_INLINE_IMPORT_LINE &&
   FLAT_INLINE_IMPORT_LINE < FLAT_INLINE_NAMESPACE_LINE ))

FLAT_IMPORT_HOST_NEGATIVE_DIR="${WORK_DIR}/cpp_import_namespace_host_negative"
mkdir -p "${FLAT_IMPORT_HOST_NEGATIVE_DIR}"
awk '
    NR == 1 { print "#define REEXPORT export import rrr.rand;" }
    { print }
    $0 == "import rrr.rand;" { print "REEXPORT" }
' "${FLAT_IMPORT_INLINE_SOURCE}" \
    >"${FLAT_IMPORT_HOST_NEGATIVE_DIR}/reexport.cppm"
awk '
    NR == 1 {
        print "#define CAT_(a, b) a ## b"
        print "#define CAT(a, b) CAT_(a, b)"
    }
    { print }
    $0 == "import rrr.rand;" { print "CAT(ex, port) CAT(im, port) rrr.rand;" }
' "${FLAT_IMPORT_INLINE_SOURCE}" \
    >"${FLAT_IMPORT_HOST_NEGATIVE_DIR}/token_paste.cppm"
awk '
    NR == 1 { print "#define PROVIDER rrr.rand" }
    { print }
    $0 == "import rrr.rand;" { print "export import PROVIDER;" }
' "${FLAT_IMPORT_INLINE_SOURCE}" \
    >"${FLAT_IMPORT_HOST_NEGATIVE_DIR}/provider_alias.cppm"
for host_case in reexport token_paste provider_alias; do
    HOST_NEGATIVE="${FLAT_IMPORT_HOST_NEGATIVE_DIR}/${host_case}.cppm"
    HOST_NEGATIVE_BEFORE="${HOST_NEGATIVE}.before"
    HOST_NEGATIVE_LOG="${FLAT_IMPORT_HOST_NEGATIVE_DIR}/${host_case}.log"
    cp "${HOST_NEGATIVE}" "${HOST_NEGATIVE_BEFORE}"
    if "${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
        --rewrite --files "${HOST_NEGATIVE}" >"${HOST_NEGATIVE_LOG}" 2>&1
    then
        echo "cpp_import_namespace host re-export assembly unexpectedly passed: ${host_case}" >&2
        exit 1
    fi
    cmp -s "${HOST_NEGATIVE_BEFORE}" "${HOST_NEGATIVE}"
    grep -Eq 'module re-export|top-level module-import zone' "${HOST_NEGATIVE_LOG}"
done

[[ "$(grep -Ec '^namespace rusty_cpp_abi_detail_m_[0-9a-f]{64} \{' \
    "${INLINE_GENERATED}")" -eq 1 ]]
grep -Eq 'return rusty_cpp_abi_sem_m_[0-9a-f]{64}_echo_bytes\(' \
    "${INLINE_GENERATED}"
grep -Fq 'std::string echo_bytes(std::string bytes) {' "${INLINE_GENERATED}"
grep -Fq 'std::string InlineCodec::via_earlier(std::string bytes) {' \
    "${INLINE_GENERATED}"
! grep -Fq 'inline std::string echo_bytes(std::string bytes) {' \
    "${INLINE_GENERATED}"
! grep -Fq 'inline std::string InlineCodec::via_earlier(std::string bytes) {' \
    "${INLINE_GENERATED}"

cp "${MARKER_FREE_INLINE_SOURCE}" "${MARKER_FREE_INLINE_GENERATED}"
"${REPO_ROOT}/target/debug/rusty-cpp-transpiler" inline-rust \
    --rewrite --files "${MARKER_FREE_INLINE_GENERATED}"
cmp -s "${MARKER_FREE_INLINE_SOURCE}" "${MARKER_FREE_INLINE_GENERATED}"

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
    -DCPP_ABI_GENERATED_MODULE="${GENERATED}" \
    -DCPP_ABI_INLINE_MODULE="${INLINE_GENERATED}" \
    -DCPP_IMPORT_NAMESPACE_CRATE_MODULE="${FLAT_IMPORT_CRATE_MODULE}" \
    -DCPP_IMPORT_NAMESPACE_INLINE_MODULE="${FLAT_IMPORT_INLINE_GENERATED}"
cmake --build "${BUILD_DIR}" \
    --target cpp_abi_core_runtime cpp_abi_inline_runtime \
        cpp_import_namespace_runtime -j "${JOBS:-2}"
ctest --test-dir "${BUILD_DIR}" --output-on-failure \
    -R '^(cpp_abi_(core|inline)|cpp_import_namespace)_runtime$'

NONREEXPORT_LOG="${WORK_DIR}/cpp_import_namespace_nonreexport.log"
if cmake --build "${BUILD_DIR}" \
    --target cpp_import_namespace_nonreexport -j "${JOBS:-2}" \
    >"${NONREEXPORT_LOG}" 2>&1
then
    echo "private cpp_import_namespace provider leaf was re-exported" >&2
    exit 1
fi
grep -Eq "randgen_rand_raw.*(not visible|no member|declaration.*not reachable|must be imported)" \
    "${NONREEXPORT_LOG}"

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

INLINE_MODULE_OBJECT="$(find "${BUILD_DIR}" -path '*cpp_abi_inline.cppm.o' \
    -type f -print -quit)"
[[ -n "${INLINE_MODULE_OBJECT}" ]]
INLINE_STRONG_SYMBOLS="$(nm -C "${INLINE_MODULE_OBJECT}" | \
    awk '$2 ~ /^[TDB]$/ { print $0 }')"
[[ "$(printf '%s\n' "${INLINE_STRONG_SYMBOLS}" | sed '/^$/d' | wc -l)" -eq 4 ]]
printf '%s\n' "${INLINE_STRONG_SYMBOLS}" | \
    grep -Fq 'initializer for module cpp_abi_inline'
printf '%s\n' "${INLINE_STRONG_SYMBOLS}" | grep -Fq 'echo_bytes@cpp_abi_inline'
printf '%s\n' "${INLINE_STRONG_SYMBOLS}" | \
    grep -Fq 'InlineCodec@cpp_abi_inline::via_earlier'
printf '%s\n' "${INLINE_STRONG_SYMBOLS}" | \
    grep -Fq 'InlineCodec@cpp_abi_inline::count_weights'
! printf '%s\n' "${INLINE_STRONG_SYMBOLS}" | grep -Fq 'rusty_cpp_abi_'

INLINE_WEAK_HELPERS="$(nm -C "${INLINE_MODULE_OBJECT}" | \
    awk '$2 == "W" && /rusty_cpp_abi_(detail|sem)_/ { print $0 }')"
[[ "$(printf '%s\n' "${INLINE_WEAK_HELPERS}" | sed '/^$/d' | wc -l)" -eq 6 ]]

echo "cpp_abi crate and inline adapter compile/runtime/symbol gate passed"
