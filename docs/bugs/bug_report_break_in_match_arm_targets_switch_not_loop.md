# Bug: `break` in a `match` arm targets the C++ `switch`, not the enclosing loop

## Summary
A Rust `break` written inside a `match` arm breaks the **enclosing `loop`**, because a
`match` is not a loop. The transpiler lowers the `match` to a C++ `switch`, where `break`
instead targets the **`switch`**. To compensate it emits a *second* `break;` — but that
second statement is **unreachable dead code**, so the enclosing loop is never broken. The
loop re-iterates (typically on values that the arm already `std::move`d out), causing an
**infinite loop and/or use-after-move corruption**.

This is the same class of defect as the dropped-loop-label bug in the hashbrown
`rehash_in_place` port (`continue 'outer` → bare `continue;`): both are cases where Rust
loop control-flow must target an *outer* construct but the C++ rendering captures it.

## Category
Codegen / transpiler (not the borrow checker). Affects generated C++20 module output.

## Minimal Reproduction
Rust input (a `match` whose arms `break` the enclosing `loop`, the standard "merge two
sorted cursors" shape):

```rust
fn merge(items: impl Iterator<Item = i32>, mut cur: Cursor) {
    for other in items {
        loop {
            match cur.peek().cmp(&other) {
                Ordering::Equal   => { cur.take(); break; }   // done with `other`; break the loop
                Ordering::Greater => { cur.insert(other);  break; }   // ditto
                Ordering::Less    => { cur.next(); }          // keep scanning: re-loop
            }
        }
    }
}
```

## Expected
The `Equal` / `Greater` arms must terminate the `loop` so the outer `for` advances to the
next `other`. A correct C++ lowering uses a flag (or `goto`) so `break` reaches the loop:

```cpp
for (auto&& other : items) {
    bool done = false;
    while (true) {
        switch (cur.peek().cmp(other)) {
            case Ordering::Equal:   { cur.take();        done = true; break; }
            case Ordering::Greater: { cur.insert(other); done = true; break; }
            case Ordering::Less:    { cur.next(); break; }   // re-loop (no flag)
        }
        if (done) break;   // break the while(true)
    }
}
```

(The codebase already emits exactly this `bool done` / `if (done) break;` idiom elsewhere —
e.g. `btree_port.btree.map.cppm` lines 2717/2722, 2733/2741, 3469/3474 — so the correct
shape is known to the generator; it just isn't applied here.)

## Actual
The transpiler emits two consecutive `break;` statements. The first exits the `switch`
case; the second is unreachable and breaks nothing, so `while (true)` never terminates on
the `Equal`/`Greater` arms:

```cpp
// transpiled/btree_port/btree_port.btree.map.cppm  — BTreeMap::merge
for (auto&& _for_item : rusty::for_in(other_iter)) {        // 5775  outer loop
    auto&& other_key = ...std::get<0>(...);
    auto&& other_val = ...std::get<1>(...);
    while (true) {                                          // 5778  inner loop
        if (auto&& s = self_cursor.peek_next(); s.is_some()) {
            ...
            switch (K::cmp(self_key, other_key)) {          // 5782
            case Ordering::Equal: {
                ... conflict(k, std::move(v), std::move(other_val)) ...   // 5789  moves other_val
                self_cursor.insert_after_unchecked(std::move(k), std::move(v_shadow1));
                break;                                       // 5795  exits the switch
                break;                                       // 5796  DEAD CODE — never runs
            }
            case Ordering::Greater: {
                self_cursor.insert_before_unchecked(std::move(other_key), std::move(other_val)); // 5802 moves both
                break;                                       // 5804  exits the switch
                break;                                       // 5805  DEAD CODE — never runs
            }
            case Ordering::Less: {
                self_cursor.next();
                break;                                       // 5810  correct: exit switch, re-loop
            }
            }
        } else {
            self_cursor.insert_before_unchecked(std::move(other_key), std::move(other_val));
            break;                                           // 5818  correct: else is not in a switch
        }
    }
}
```

After the `Equal` arm runs it has already `std::move`d `other_val` into `conflict(...)`
(5789); after `Greater` it has moved both `other_key` and `other_val` (5802). Because the
intended outer-loop break is dead, `while(true)` re-iterates and re-reads the moved-from
`other_key`/`other_val`, with cursor state that no longer matches — so the loop spins
(inserting garbage / corrupting the tree) instead of advancing to the next `other`.

The `Less` arm (single `break;`) and the `else` branch (single `break;`, not inside a
`switch`) are correct, which confirms the intended target of the dead second break was the
`while(true)`, not the `switch`.

## Impact
`BTreeMap::merge` is broken for any input where an `other` key compares `Equal` or `Greater`
to a `self` key — i.e. essentially every non-trivial merge: infinite loop and/or
use-after-move corruption. (Currently latent in the mako build only because the BTreeMap
port is not exercised by the rrr code path.)

## Root Cause
Rust `break`/`continue` inside a `match` arm break/continue the *enclosing loop* (a `match`
is not a loop). When the transpiler renders the `match` as a C++ `switch`, a bare `break`
binds to the `switch`. The generator detects this and tries to also break the loop by
appending a second `break;`, but a statement after an unconditional `break` is unreachable.

This is the `switch`-flavoured sibling of dropped loop labels: in both cases C++ has no
construct that lets a `break`/`continue` reach past the innermost `switch`/loop, so the
generator must emit a `goto` (to a label after the loop) or a `bool done` flag checked after
the `switch` — never two stacked `break`s.

## Suggested Fix (transpiler)
When a Rust `break`/`continue` inside a `match` arm must target a loop that the lowered
`switch` would shadow, emit either:
1. a `goto <loop_end>;` to a label placed immediately after the loop, or
2. the existing `bool done` / `if (done) break;` idiom (set the flag in the arm, keep a
   single `break;` to leave the `switch`, and check the flag after the `switch`).

The same mechanism is needed for labeled `break 'l` / `continue 'l` (see the hashbrown
`rehash_in_place` dropped-label bug); a single goto-based loop-control lowering covers both.

## Workaround (port-level, applied for the hashbrown sibling, not yet for this one)
Hand-edit the generated `.cppm`: replace the dead `break; break;` in the `Equal` and
`Greater` arms with a `bool done = false;` declared at the top of the `while (true)` body,
`done = true;` in those arms before the single switch `break;`, and `if (done) break;`
after the `switch`. Not yet applied because the BTreeMap port is unused on the current path.

## Affected File
`transpiled/btree_port/btree_port.btree.map.cppm`, `BTreeMap::merge`, lines ~5775–5806
(dead breaks at 5796 and 5805). Generated from the Rust `alloc::collections::btree::map`
merge implementation.
