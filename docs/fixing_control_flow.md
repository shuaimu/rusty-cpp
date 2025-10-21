# How to Fix Control Flow Analysis in the Borrow Checker

## Current Problem

The checker currently processes statements linearly without understanding control flow. The IR has a `ControlFlowGraph` structure, but it's not properly built - everything goes into a single basic block.

```rust
// Current situation in src/ir/mod.rs
let entry_block = BasicBlock {
    id: 0,
    statements,  // ALL statements dumped here
    terminator: None,
};
```

## Solution Overview

To fix control flow analysis, we need to:

1. **Build a proper Control Flow Graph (CFG)**
2. **Implement path-sensitive dataflow analysis**
3. **Track variable states per program point**
4. **Handle merge points correctly**

## Detailed Implementation Plan

### 1. Proper CFG Construction

#### What's Needed
```rust
// Enhanced IR structures needed
pub struct BasicBlock {
    pub id: usize,
    pub statements: Vec<IrStatement>,
    pub terminator: Terminator,  // Not optional!
}

pub enum Terminator {
    Return(Option<String>),
    Jump(NodeIndex),
    Branch {
        condition: String,
        then_block: NodeIndex,
        else_block: NodeIndex,
    },
    Switch {
        value: String,
        cases: Vec<(i32, NodeIndex)>,
        default: NodeIndex,
    },
    Loop {
        condition: String,
        body: NodeIndex,
        exit: NodeIndex,
    },
}
```

#### Implementation Steps

1. **AST Visitor Enhancement** - Parse all control structures:
```rust
// In ast_visitor.rs
fn extract_statement(entity: &Entity) -> Statement {
    match entity.get_kind() {
        EntityKind::IfStmt => extract_if_statement(entity),
        EntityKind::ForStmt => extract_for_loop(entity),
        EntityKind::WhileStmt => extract_while_loop(entity),
        EntityKind::DoStmt => extract_do_while(entity),
        EntityKind::SwitchStmt => extract_switch(entity),
        EntityKind::BreakStmt => Statement::Break,
        EntityKind::ContinueStmt => Statement::Continue,
        // ... etc
    }
}
```

2. **CFG Builder** - Create proper basic blocks:
```rust
// New module: src/ir/cfg_builder.rs
pub struct CfgBuilder {
    blocks: Vec<BasicBlock>,
    current_block: NodeIndex,
}

impl CfgBuilder {
    fn visit_if(&mut self, condition: Expr, then_stmt: Stmt, else_stmt: Option<Stmt>) {
        let branch_block = self.current_block;
        let then_block = self.new_block();
        let else_block = self.new_block();
        let merge_block = self.new_block();
        
        // Set terminator for branch block
        self.blocks[branch_block].terminator = Terminator::Branch {
            condition,
            then_block,
            else_block,
        };
        
        // Process then branch
        self.current_block = then_block;
        self.visit_statement(then_stmt);
        self.blocks[then_block].terminator = Terminator::Jump(merge_block);
        
        // Process else branch
        self.current_block = else_block;
        if let Some(stmt) = else_stmt {
            self.visit_statement(stmt);
        }
        self.blocks[else_block].terminator = Terminator::Jump(merge_block);
        
        self.current_block = merge_block;
    }
    
    fn visit_loop(&mut self, condition: Expr, body: Stmt) {
        let loop_header = self.new_block();
        let loop_body = self.new_block();
        let loop_exit = self.new_block();
        
        // Jump to loop header
        self.blocks[self.current_block].terminator = Terminator::Jump(loop_header);
        
        // Loop header checks condition
        self.blocks[loop_header].terminator = Terminator::Branch {
            condition,
            then_block: loop_body,
            else_block: loop_exit,
        };
        
        // Process loop body
        self.current_block = loop_body;
        self.visit_statement(body);
        self.blocks[loop_body].terminator = Terminator::Jump(loop_header);
        
        self.current_block = loop_exit;
    }
}
```

### 2. Path-Sensitive Dataflow Analysis

#### What's Needed
Instead of a single `OwnershipTracker`, we need ownership state at each program point:

```rust
// New structure for dataflow analysis
pub struct DataflowAnalysis {
    // State at entry of each block
    block_entry_states: HashMap<NodeIndex, OwnershipState>,
    // State at exit of each block  
    block_exit_states: HashMap<NodeIndex, OwnershipState>,
}

pub struct OwnershipState {
    ownership: HashMap<String, OwnershipStatus>,
    borrows: HashMap<String, BorrowInfo>,
}
```

#### Implementation
```rust
impl DataflowAnalysis {
    pub fn analyze(cfg: &ControlFlowGraph) -> Result<Self, Vec<String>> {
        let mut analysis = Self::new();
        let mut worklist = VecDeque::new();
        
        // Start with entry block
        worklist.push_back(cfg.entry_block());
        
        while let Some(block_id) = worklist.pop_front() {
            let block = &cfg[block_id];
            
            // Get entry state (join of predecessors)
            let entry_state = analysis.join_predecessor_states(cfg, block_id);
            
            // Process block statements
            let exit_state = analysis.transfer_function(block, entry_state)?;
            
            // If state changed, add successors to worklist
            if analysis.block_exit_states[&block_id] != exit_state {
                analysis.block_exit_states.insert(block_id, exit_state);
                for successor in cfg.successors(block_id) {
                    worklist.push_back(successor);
                }
            }
        }
        
        Ok(analysis)
    }
    
    fn join_predecessor_states(&self, cfg: &ControlFlowGraph, block: NodeIndex) -> OwnershipState {
        let predecessors: Vec<_> = cfg.predecessors(block).collect();
        
        if predecessors.is_empty() {
            return OwnershipState::initial();
        }
        
        // Join states from all predecessors
        let mut joined = OwnershipState::new();
        
        for pred in predecessors {
            let pred_exit = &self.block_exit_states[&pred];
            joined = self.join_states(joined, pred_exit);
        }
        
        joined
    }
    
    fn join_states(&self, state1: OwnershipState, state2: &OwnershipState) -> OwnershipState {
        // Conservative join: if states differ, mark as "maybe moved"
        let mut result = OwnershipState::new();
        
        for (var, status1) in state1.ownership {
            if let Some(status2) = state2.ownership.get(&var) {
                result.ownership.insert(var, self.join_ownership(status1, status2));
            }
        }
        
        result
    }
    
    fn join_ownership(&self, s1: OwnershipStatus, s2: &OwnershipStatus) -> OwnershipStatus {
        match (s1, s2) {
            (OwnershipStatus::Owned, OwnershipStatus::Owned) => OwnershipStatus::Owned,
            (OwnershipStatus::Moved, OwnershipStatus::Moved) => OwnershipStatus::Moved,
            (OwnershipStatus::Owned, OwnershipStatus::Moved) |
            (OwnershipStatus::Moved, OwnershipStatus::Owned) => OwnershipStatus::MaybeMoved,
            // ... handle other cases
        }
    }
}
```

### 3. Loop Analysis

For loops, we need to:
1. Analyze the loop body assuming it can execute 0 or more times
2. Detect if variables are moved in the loop (error if used in second iteration)

```rust
pub struct LoopAnalyzer {
    loop_headers: HashSet<NodeIndex>,
    loop_bodies: HashMap<NodeIndex, HashSet<NodeIndex>>,
}

impl LoopAnalyzer {
    fn analyze_loop(&mut self, header: NodeIndex, cfg: &ControlFlowGraph) -> Result<(), String> {
        // First iteration: assume variables are available
        let mut first_iter_state = self.analyze_iteration(header, initial_state);
        
        // Second iteration: use exit state from first iteration
        let second_iter_state = self.analyze_iteration(header, first_iter_state);
        
        // Check for use-after-move in second iteration
        for (var, status) in second_iter_state.ownership {
            if status == OwnershipStatus::Moved {
                if self.is_used_in_loop(var, header) {
                    return Err(format!("Use after move in loop: {}", var));
                }
            }
        }
        
        Ok(())
    }
}
```

### 4. Scope Tracking

Track when variables go out of scope:

```rust
pub struct ScopeTracker {
    scopes: Vec<Scope>,
    current_scope: usize,
    variable_scopes: HashMap<String, usize>,
}

impl ScopeTracker {
    fn enter_block(&mut self) {
        let new_scope = Scope {
            parent: Some(self.current_scope),
            variables: HashSet::new(),
        };
        self.scopes.push(new_scope);
        self.current_scope = self.scopes.len() - 1;
    }
    
    fn exit_block(&mut self) {
        // Mark all variables in this scope as dead
        let scope = &self.scopes[self.current_scope];
        for var in &scope.variables {
            self.mark_dead(var);
        }
        
        // Return to parent scope
        if let Some(parent) = scope.parent {
            self.current_scope = parent;
        }
    }
}
```

## Incremental Implementation Strategy

### Phase 1: Basic CFG (Easiest)
1. Parse if/else statements properly
2. Create separate basic blocks for then/else branches
3. Add merge blocks after conditionals
4. Test with simple if/else examples

### Phase 2: Scope Tracking
1. Track block scopes `{ }`
2. Mark variables as out-of-scope when block ends
3. Remove borrows when variables go out of scope
4. Test with nested scope examples

### Phase 3: Loop Detection
1. Parse for/while loops
2. Create loop header and body blocks
3. Analyze loop body twice to detect iteration issues
4. Test with loop examples

### Phase 4: Path Sensitivity
1. Implement dataflow analysis framework
2. Track different states for different paths
3. Join states at merge points
4. Handle "maybe moved" states

### Phase 5: Advanced Features
1. Switch statements
2. Break/continue
3. Early returns
4. Exception handling (try/catch)

## Example: Fixing If/Else

Here's a concrete example of fixing if/else handling:

```rust
// Before (current implementation)
fn convert_function(func: &Function) -> IrFunction {
    let mut statements = Vec::new();
    for stmt in &func.body {
        statements.extend(convert_statement(stmt)?);
    }
    // Everything in one block!
    let block = BasicBlock { statements, ... };
}

// After (with proper CFG)
fn convert_function(func: &Function) -> IrFunction {
    let mut cfg_builder = CfgBuilder::new();
    
    for stmt in &func.body {
        cfg_builder.visit_statement(stmt);
    }
    
    cfg_builder.build()  // Returns proper CFG
}

fn visit_if_statement(
    &mut self,
    condition: &Expr,
    then_stmt: &Statement,
    else_stmt: Option<&Statement>
) {
    // Create blocks
    let then_block = self.new_block();
    let else_block = self.new_block();
    let merge_block = self.new_block();
    
    // Add branch terminator
    self.current_block_mut().terminator = Terminator::Branch {
        condition: self.convert_expr(condition),
        then_block,
        else_block,
    };
    
    // Process then branch
    self.current = then_block;
    self.visit_statement(then_stmt);
    self.add_jump(merge_block);
    
    // Process else branch
    self.current = else_block;
    if let Some(stmt) = else_stmt {
        self.visit_statement(stmt);
    }
    self.add_jump(merge_block);
    
    // Continue at merge point
    self.current = merge_block;
}
```

## Estimated Effort

- **Phase 1 (Basic CFG)**: 2-3 days
- **Phase 2 (Scope Tracking)**: 1-2 days  
- **Phase 3 (Loop Detection)**: 2-3 days
- **Phase 4 (Path Sensitivity)**: 3-5 days
- **Phase 5 (Advanced)**: 5-7 days

**Total**: 2-3 weeks for comprehensive fix

## Alternative: Simpler Partial Fix

If full CFG is too complex, a simpler approach:

1. **Just track scopes** - Add scope entry/exit tracking
2. **Conservative loop handling** - Assume all loops execute 0+ times
3. **Basic if/else** - Track that branches are exclusive

This would fix many false positives without full dataflow analysis.

## Testing Strategy

Create test cases for each phase:
```cpp
// Test for Phase 1
void test_if_else() {
    int x = 42;
    if (condition) {
        int& r1 = x;
    } else {
        int& r2 = x;  // Should not error
    }
}

// Test for Phase 2  
void test_scope() {
    int x = 42;
    { int& r1 = x; }
    { int& r2 = x; }  // Should not error
}

// Test for Phase 3
void test_loop() {
    int x = 42;
    for(int i = 0; i < 2; i++) {
        int y = std::move(x);  // Should error
    }
}
```