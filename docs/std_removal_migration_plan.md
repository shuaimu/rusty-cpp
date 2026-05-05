# Rusty Runtime `std`-Removal Plan

Status: In Progress (Phases 1-9 implemented; full-matrix verification pending long-running crates)
Owner: runtime/transpiler
Last updated: 2026-05-05

## Motivation
The runtime currently depends heavily on the C++ standard library. That blocks:
- freestanding-ish builds,
- tighter control over ABI and platform primitives,
- direct mapping from Rust concepts to platform APIs (for example, `std::thread` -> `pthread`).

The goal is to reduce and eventually remove `std` dependencies from the runtime and transpiled output surfaces where practical.

## Baseline Audit (current tree)
Audit scope: `include/rusty/**/*.hpp`, `include/rusty/rusty.cppm`.

- Files in scope: 53
- Files including standard headers: 49
- Lines containing `std::...`: 2924
- Total `std::...` symbol references: 3639

Hotspots by `std::...` lines:
- `include/rusty/array.hpp` (450)
- `include/rusty/slice.hpp` (388)
- `include/rusty/winnow_stream.hpp` (152)
- `include/rusty/result.hpp` (145)
- `include/rusty/option.hpp` (124)

High-impact runtime primitives currently on `std`:
- `thread`, `mutex`, `rwlock`, `condvar`, `once`, channel internals.

## Target Profiles
We will execute in two profiles:

1. `host-minimal`
- Goal: remove direct `std` runtime primitive usage first.
- Allow temporary `std` use in type traits and some generated type surfaces.

2. `strict-no-std` (later)
- Goal: no `std` in runtime APIs and generated surfaces (except unavoidable compiler/toolchain intrinsics).

## Migration Strategy
### Phase 1: Platform Layer + Runtime Primitive Indirection
- Add `rusty::platform` backend abstraction.
- Add backend switch:
  - default `cppstd`
  - opt-in `posix` via compile definition.
- Route runtime primitives through platform layer first:
  - mutex
  - condvar
  - rwlock
  - once
  - channel locks/condition-variable internals
  - small spin/yield call sites in lock-free channel

Acceptance:
- Existing tests still pass with default backend.
- POSIX backend compiles for migrated headers.

### Phase 2: Threading Backend Replacement
- Introduce `rusty::thread` internals backed by platform layer.
- Replace direct `std::thread` usage path-by-path.
- Move sleep/yield/ID calls to platform API uniformly.

Acceptance:
- `thread` tests pass for both backends.

### Phase 3: Process/FS + IO Surface Cleanup
- Remove direct `std::filesystem` and related runtime dependencies.
- Replace with POSIX wrappers (or platform shims with equivalent behavior).

Acceptance:
- `process`/`io` APIs pass existing runtime tests.

### Phase 4: Container/Utility Surface Reduction
- Reduce `std::vector`, `std::string`, `std::optional`, `std::variant` exposure in runtime internals.
- Prefer `rusty::Vec`, `rusty::String`, `rusty::Option`, `rusty::Result` where feasible.

Acceptance:
- no regression in transpiler parity set.

### Phase 5: Transpiler Contract Update
- Add codegen mode to emit more rusty-native surfaces, reducing hard dependencies on `std` type spellings.

Acceptance:
- parity matrix remains stable in module builds.

### Phase 6: Repeatable `std` Audit Tooling
- Add a repo-local audit script to measure:
  - `std::...` line and symbol counts,
  - high-churn files,
  - forbidden-include hits by profile.
- Keep output machine-readable for CI handoff.

Acceptance:
- One-command audit works for runtime/transpiler/all scopes.
- Host-minimal and strict-no-std forbidden-include profile checks available.

### Phase 7: Runtime Utility/Meta Alias Layer
- Add runtime-owned alias spellings for view and utility surfaces:
  - `rusty::StrView`
  - `rusty::Span<T, Extent>`
  - `rusty::forward`, `rusty::exchange`, `rusty::swap`
- Keep semantics compatible with existing `std`-backed runtime behavior.

Acceptance:
- Default build behavior unchanged.
- Alias surfaces available for transpiler opt-in and manual runtime use.

### Phase 8: Container Interop Feature Gates
- Add opt-out gates for remaining `std` container interop surfaces:
  - `RUSTY_NO_STD_VECTOR_INTEROP`
  - `RUSTY_NO_STD_OPTIONAL_INTEROP`
- Preserve default compatibility when gates are unset.

Acceptance:
- Runtime still builds with default settings.
- Interop APIs can be removed by macro without touching transpiler output.

### Phase 9: Transpiler View-Alias Opt-In
- Add transpiler CLI/options flag to prefer runtime view aliases:
  - `--prefer-rusty-view-aliases`
- Lower output spellings:
  - `std::string_view` -> `rusty::StrView`
  - `std::span<...>` -> `rusty::Span<...>`
- Extend parity matrix harness flag pass-through.

Acceptance:
- Flag is fully wired through CLI -> parity harness -> codegen.
- Default output remains unchanged when flag is not set.

## CI / Regression Gates
- Keep current parity matrix pass set green during each phase.
- Add `std-audit` counters in CI:
  - count of `std::` symbols,
  - count of forbidden includes by profile.
- Disallow net-new `std` usage in migrated files.

## Work Completed in This Change
### Phase 1
- Added backend configuration and threading/sync platform abstractions:
  - `include/rusty/platform/config.hpp`
  - `include/rusty/platform/threading.hpp`
- Migrated runtime headers to platform primitives (default behavior unchanged):
  - `include/rusty/mutex.hpp`
  - `include/rusty/condvar.hpp`
  - `include/rusty/rwlock.hpp`
  - `include/rusty/once.hpp`
  - `include/rusty/sync/mpsc.hpp`
  - `include/rusty/sync/mpsc_lockfree.hpp` (yield/sleep sites)
  - `include/rusty/mem.hpp`
  - `include/rusty/barrier.hpp`

### Phase 2
- Added backend-owned thread API in `include/rusty/platform/threading.hpp`:
  - `thread`, `thread_id`, `thread_id_equal`, `thread_id_less`, `thread_id_hash`.
- Migrated `include/rusty/thread.hpp` to use platform thread primitives for:
  - spawn/join/detach handle storage,
  - thread IDs,
  - sleep/yield,
  - park/unpark lock primitives.

### Phase 3
- Removed direct `std::filesystem` use from `include/rusty/process.hpp`.
- Replaced path manipulation and `current_exe()` fallback logic with POSIX/C-library-backed implementation.

### Phase 4
- Added `rusty::Vec` batch receive surfaces to lock-free MPSC:
  - `batch_recv_rusty`
  - `recv_batch_rusty`
- Added optional compile-time switch `RUSTY_NO_STD_VECTOR_BATCH_API` to disable std-vector batch API spellings.

### Phase 5
- Added transpiler/runtime contract hook for unit-type spelling reduction:
  - runtime alias: `rusty::Unit` (`rusty.hpp`, `rusty.cppm`)
  - transpiler option plumbing:
    - CLI: `--prefer-rusty-unit-alias`
    - parity matrix script pass-through: `--prefer-rusty-unit`
    - codegen replacement mode: `std::tuple<>` -> `rusty::Unit` (opt-in).

### Phase 6
- Added audit tool:
  - `tools/std_audit_cpp.sh`
- Capabilities:
  - `--scope runtime|transpiler|all`
  - `--profile host-minimal|strict-no-std`
  - `--format text|json`

### Phase 7
- Added runtime alias/utilities:
  - `rusty::StrView`
  - `rusty::Span<T, Extent>`
  - `rusty::forward`, `rusty::exchange`, `rusty::swap`
- Implemented in:
  - `include/rusty/rusty.hpp`
  - `include/rusty/rusty.cppm`

### Phase 8
- Added vector interop gates:
  - `RUSTY_NO_STD_VECTOR_INTEROP`
  - gated `Vec<T> -> std::vector<T>` conversion
  - gated `boxed::into_vec(std::vector<...>)`
- Added optional interop gate:
  - `RUSTY_NO_STD_OPTIONAL_INTEROP`
  - gated `Option` nullopt/optional constructors/assignments/comparisons
- Added `BTreeMap::range_rusty(...)` and gated `range(...)` behavior.
- Replaced async executor task storage from `std::vector` to `rusty::Vec`.

### Phase 9
- Added transpiler opt-in flag:
  - CLI: `--prefer-rusty-view-aliases`
  - parity matrix script: `--prefer-rusty-views`
- Added codegen output rewrite mode for view aliases.

## Known Remaining Blockers
- We still rely on `std::future`/`std::packaged_task` in `thread.hpp` result plumbing.
- Many runtime container/string APIs remain `std`-shaped by design for compatibility (even with interop gates).
- Full strict-no-std transpiler output is not complete; current changes provide additional opt-in alias paths (`Unit`, view aliases).
- Strict profile conformance still needs dedicated CI job wiring.

## Next Implementation Steps
1. Add CI jobs for:
   - `RUSTY_PLATFORM_BACKEND_POSIX=1`
   - `RUSTY_NO_STD_VECTOR_BATCH_API=1`
   - `RUSTY_NO_STD_VECTOR_INTEROP=1`
   - `RUSTY_NO_STD_OPTIONAL_INTEROP=1`
   - `tools/std_audit_cpp.sh --scope runtime --profile host-minimal`
2. Replace `std::future`/`std::packaged_task` usage with runtime-owned async/result handoff primitives.
3. Incrementally reduce high-volume `std::forward`/`std::move` callsites to runtime utilities where semantics are equivalent.
