# Guard flow and autoderef: the systematic design

Status: phases 1–2 landed; phases 3–5 in progress. The living regression
surface is `transpiler/tests/guard_context_matrix.rs`.

## The problem

Rust's cell/lock types hand out **guard objects** — `Ref`, `RefMut`,
`MutexGuard`, `RwLock` read/write guards — and then *autoderef* makes them
invisible: `g.push_back(v)`, `*g = v`, `g.len()` all deref implicitly, so the
source usually contains **no `*` at all**. rustc resolves this by walking the
receiver's type through its `Deref` chain and recording *adjustments*; the
derefs become explicit in MIR.

The transpiler cannot copy that wholesale — it has partial type knowledge —
and its type model historically said `borrow_mut()` yields `&mut T` (true of
Rust, false of the C++ runtime, which yields the guard **by value**). Every
consumption context that trusted the model dropped a deref that was never in
the source. That single mismatch surfaced one syntactic context at a time as
issues #32 (generic receiver), #34 (assignment LHS), #35 (concrete receiver,
inline and bound), and their unreported siblings.

## The design: two tiers, one rule each

**Typed tier** — the receiver's type is visible (a known `RefCell` field).
The concrete lowerings emit a direct `->` through the guard. Requirement:
*the model must tell the truth*; the guard-producing methods and
guard-by-value returns are recognized explicitly.

**Untyped tier** — the receiver is generic, or concrete but out of view.
Classification happens by **value flow, not syntactic shape**
(`transpiler/src/codegen/guard_flow.rs`), and consumption goes through the
tolerant runtime, which is correct whether the value turns out to be a guard
or a plain reference:

- method receiver → the dispatch ladder / `rusty::deref_call(recv, __mdisp_m{}, args…)`
- place or value use (`*g`, `*g = v`, `*g += v`) → `rusty::detail::deref_if_pointer_like(g)`
- free-helper argument (`rusty::len(g)`, `rusty::iter(g)`, …) → the helper's
  own **deref-peel arm** (see invariant 2)

### The doctrine, numbered

1. **Classify flow, not shape.** A guard reaches its consumer directly, via
   `unwrap()`/`expect()` (`lock()` returns `Result<Guard, _>`), via a local
   binding, or via the tail of a block/`if`/`match` initializer. Every flow
   path must give the same answer. A per-site syntactic predicate is how the
   class kept reopening.
2. **Every shape-laddered runtime helper ends with a deref-peel arm**
   (`else if constexpr (requires { *x; }) { return helper(*x); }` —
   see `rusty::iter`, `rusty::len`, `rusty::contains`). This is the single
   choke point for the ~70 method→free-helper routings in the emitter: one
   arm per helper covers every emission site *and* hand-written C++.
   Corollary: a helper ladder with a permissive `return false`/default tail
   hides silent-wrongs — a guard falls through every shape probe and the tail
   invents an answer. `contains` did exactly this.
3. **Fire only on untypable receivers**
   (`infer_simple_expr_type(..).is_none()`). A typable receiver belongs to
   the typed tier, which emits better code; an unconditional classifier
   regressed three golden tests.
4. **The dispatch ladder is only sound if runtime members are
   Rust-faithful.** `deref_call` walks the deref chain and calls the member
   it finds — with the member's semantics. `VecDeque::front()` returning
   `T&` (C++-style) instead of `Option<&T>` broke both tiers; the fix is to
   make the member faithful, not to compensate in the emitter. When adding a
   runtime type, its members must have Rust return shapes.
5. **The guard itself is a value too.** Binding (`let g = …` → `auto&&`,
   which lifetime-extends a guard prvalue *and* still aliases a plain
   reference — by-value would silently copy the reference case), returning,
   and `drop(g)` (emits `std::move` — drop *consumes*) operate on the guard,
   not through it. The adapter must not fire there.

### Bindings

`let g = x.borrow_mut();` on an untypable receiver binds `auto&&` and records
`g` in `local_uncertain_guard_bindings`; later consumptions of the bare name
classify through `expr_is_uncertain_guard_local`. Trap: scope-stack trackers
silently no-op when the stack is empty, and **inline-rust never pushes a
scope** — the recorder self-seeds.

## The regression surface

`transpiler/tests/guard_context_matrix.rs` transpiles (through the real
binary), compiles, and **runs** every known consumption context, asserting
the produced values — currently 24 rows: inline calls, autoderef calls on
bound guards, deref assign/read/compound-assign, reborrowed arguments,
comparison, arithmetic, `if let`, `for`, return position, chained calls,
`lock().unwrap()`, indexing, repeated use, `if`/`match`/block-tail
initializers, the free-helper family (`len`/`is_empty`/`front`/`back`/
`get`/`contains`), a generic-receiver variant, and drop-then-reborrow.

**A new guard bug is a new row.** Write the row first (it must fail), then
fix; rows cannot regress silently. Probe hygiene: rows must be *valid Rust*
(`Mutex::lock()` returns a `Result` — `.lock().m()` tests nothing; VecDeque
has `front`/`back`, not `first`/`last`).

## What falls out of the audit (fixed en route)

- `VecDeque::front/back/get/pop_front/pop_back` made Rust-faithful
  (`Option<…>` returns) — both tiers were broken and nothing had ever
  exercised them.
- `rusty::is_empty` promoted to a header with a correct ladder; the emitted
  block's copy had a permissive `false` tail and stands down via
  `RUSTY_HAS_FREE_IS_EMPTY`.
- `rusty::addr_of_temp{,_mut}` promoted to `rusty.hpp`
  (`RUSTY_HAS_ADDR_OF_TEMP` gates the emitted copies) — inline-rust rewrites
  carry no runtime block, so block-only helpers were undefined references.
- `rusty::contains` gained the haystack peel arm (rule 2).
- `drop(x)` emits `std::move(x)` — it consumes; a move-only guard failed on
  the deleted copy ctor, and a copyable value silently dropped a *copy*.

## Known remaining gaps (planned)

- **Guards returned from user functions** (`fn all(&self) -> MutexGuard<…>`)
  — classify by the callee's declared return-type tail naming a guard type.
- **Tuple-destructured guards** (`let (a, b) = (x.borrow(), y.borrow());`).
- Typed-tier consolidation: `receiver_is_refcell_borrow`,
  `returns_guard_by_value` and friends should query one
  `known_guard_producing_call()` in guard_flow so both tiers share one
  source of truth.
- `deref_ref`/`deref_mut` remain emitted-block-only (as `addr_of_temp` was);
  promote when touched next.
