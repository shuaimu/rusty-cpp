# Summary: How to Fix Control Flow Issues

## The Problem
The borrow checker currently doesn't understand:
- Scopes (when references go out of scope)
- Loops (that they execute multiple times)
- Conditionals (that branches are mutually exclusive)
- Control flow (break, continue, return, etc.)

## Three Approaches to Fix It

### Approach 1: Quick Fix - Scope Tracking Only (1-2 days)
**Easiest to implement, fixes ~40% of issues**

```rust
// Add scope markers to the IR
enum IrStatement {
    // ... existing variants
    EnterScope,
    ExitScope,
}

// Track scopes during analysis
struct OwnershipTracker {
    scope_stack: Vec<ActiveBorrows>,
    // When ExitScope, remove borrows from that scope
}
```

**Pros:**
- Simple to implement
- Fixes false positives from block scopes
- Minimal changes to existing code

**Cons:**
- Doesn't fix loop or conditional issues
- Still treats all code as linear

### Approach 2: Moderate Fix - Basic CFG (1 week)
**Medium complexity, fixes ~70% of issues**

Build a proper Control Flow Graph:
```rust
// Enhance AST parsing
fn parse_if_statement() -> CfgNodes {
    let then_block = create_block();
    let else_block = create_block();
    let merge_block = create_block();
    // Connect blocks properly
}

// Analyze each path
for path in cfg.get_paths() {
    check_path(path);
}
```

**Pros:**
- Understands that if/else are exclusive
- Can detect loop iteration issues
- More accurate analysis

**Cons:**
- Requires rewriting IR generation
- Complex to implement correctly
- Still missing path sensitivity

### Approach 3: Full Fix - Dataflow Analysis (2-3 weeks)
**Most complex, fixes ~95% of issues**

Implement complete dataflow analysis:
```rust
struct DataflowAnalysis {
    // Track state at each program point
    states: HashMap<ProgramPoint, OwnershipState>,
    
    // Join states at merge points
    fn join_states(s1: State, s2: State) -> State {
        // If moved in one path but not other -> MaybeMoved
    }
    
    // Fixed-point iteration
    fn analyze() {
        while states_changing {
            propagate_forward();
        }
    }
}
```

**Pros:**
- Handles all control flow correctly
- Can track "maybe moved" states
- Industry-standard approach

**Cons:**
- Complex implementation
- Performance overhead
- Requires significant refactoring

## Recommended Path Forward

### Start with Approach 1 (Scope Tracking)
1. Add `EnterScope`/`ExitScope` to IR
2. Track scope stack in analyzer
3. Clean up borrows when exiting scope
4. Test with nested scope examples

### Then Add Loop Detection
1. Detect loops in AST
2. Analyze loop body twice
3. Report errors if state changes between iterations

### Example Implementation Plan

#### Week 1: Scope Tracking
```rust
// Day 1-2: Modify parser
fn parse_compound_stmt() {
    emit(IrStatement::EnterScope);
    parse_statements();
    emit(IrStatement::ExitScope);
}

// Day 3-4: Modify analyzer
fn process_statement(stmt: IrStatement) {
    match stmt {
        EnterScope => self.push_scope(),
        ExitScope => self.pop_scope_and_cleanup(),
        // ...
    }
}

// Day 5: Test and refine
```

#### Week 2: Basic Conditionals
```rust
// Track that if/else are exclusive
fn analyze_if(cond, then_branch, else_branch) {
    let saved_state = self.state.clone();
    
    analyze(then_branch);
    let then_state = self.state.clone();
    
    self.state = saved_state;
    analyze(else_branch);
    let else_state = self.state;
    
    // Merge states conservatively
    self.state = merge(then_state, else_state);
}
```

## Quick Win Implementation

Here's the minimal change that would help:

```rust
// In src/analysis/mod.rs

impl OwnershipTracker {
    fn push_scope(&mut self) {
        self.scope_borrows.push(Vec::new());
    }
    
    fn pop_scope(&mut self) {
        if let Some(borrows) = self.scope_borrows.pop() {
            for borrow_name in borrows {
                self.active_borrows.remove(&borrow_name);
            }
        }
    }
    
    fn add_borrow(&mut self, name: String) {
        if let Some(current) = self.scope_borrows.last_mut() {
            current.push(name.clone());
        }
        self.active_borrows.insert(name);
    }
}
```

## Impact Assessment

### What Gets Fixed with Each Approach

**Scope Tracking Only:**
✅ False positives from `{}` blocks
✅ Temporary references in inner scopes
❌ Loop bugs (use-after-move in iteration)
❌ Conditional path issues

**Basic CFG:**
✅ All scope issues
✅ If/else mutual exclusion
✅ Basic loop checking
❌ Complex path combinations
❌ "Maybe moved" scenarios

**Full Dataflow:**
✅ All of the above
✅ Path-sensitive analysis
✅ Loop invariants
✅ Interprocedural analysis

## Conclusion

The control flow issues CAN be fixed, with varying levels of effort:

1. **Quick fix (2 days)**: Just add scope tracking - helps a lot
2. **Good fix (1 week)**: Basic CFG - handles most real cases  
3. **Complete fix (3 weeks)**: Full dataflow - handles everything

The quick fix would eliminate many false positives and make the tool much more usable, while the complete fix would make it production-ready.