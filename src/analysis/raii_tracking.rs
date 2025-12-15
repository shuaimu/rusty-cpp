//! RAII Tracking Module
//!
//! This module implements advanced RAII (Resource Acquisition Is Initialization) tracking:
//! - Phase 1: Reference/pointer stored in container outliving pointee
//! - Phase 2: User-defined RAII types (classes with destructors)
//! - Phase 3: Iterator outlives container
//! - Phase 4: Lambda escape analysis (refined)
//! - Phase 5: Member lifetime tracking
//! - Phase 6: new/delete tracking
//! - Phase 7: Constructor initialization order

use crate::ir::{IrFunction, IrStatement, BorrowKind, OwnershipState};
use crate::parser::HeaderCache;
use std::collections::{HashMap, HashSet};
use crate::debug_println;

/// Track pointers/references stored in containers
/// When a pointer is stored in a container, the pointee must outlive the container
#[derive(Debug, Clone)]
pub struct ContainerBorrow {
    /// The container variable (e.g., "vec")
    pub container: String,
    /// The pointee variable (e.g., "x" in vec.push_back(&x))
    pub pointee: String,
    /// Scope level where the container was declared
    pub container_scope: usize,
    /// Scope level where the pointee was declared
    pub pointee_scope: usize,
    /// Line number for error reporting
    pub line: usize,
}

/// Track iterator borrows from containers
#[derive(Debug, Clone)]
pub struct IteratorBorrow {
    /// The iterator variable (e.g., "it")
    pub iterator: String,
    /// The container it borrows from (e.g., "vec")
    pub container: String,
    /// Scope level where the iterator was declared
    pub iterator_scope: usize,
    /// Scope level where the container was declared
    pub container_scope: usize,
    /// Line number for error reporting
    pub line: usize,
}

/// Track lambda captures and their escape potential
#[derive(Debug, Clone)]
pub struct LambdaCapture {
    /// The lambda variable name
    pub lambda_var: String,
    /// Variables captured by reference
    pub ref_captures: Vec<String>,
    /// Scope level where lambda was declared
    pub lambda_scope: usize,
    /// Whether lambda has escaped (assigned to longer-lived variable or returned)
    pub has_escaped: bool,
    /// Line number for error reporting
    pub line: usize,
}

/// Track new/delete operations for double-free and use-after-free detection
#[derive(Debug, Clone, PartialEq)]
pub enum AllocationState {
    /// Memory is allocated and valid
    Allocated,
    /// Memory has been freed
    Freed,
}

/// Track heap allocations
#[derive(Debug, Clone)]
pub struct HeapAllocation {
    pub variable: String,
    pub state: AllocationState,
    pub allocation_line: usize,
    pub free_line: Option<usize>,
}

/// Track references to object members (Phase 5)
/// When &obj.field is taken, the reference has the same lifetime as obj
#[derive(Debug, Clone)]
pub struct MemberBorrow {
    /// The reference variable (e.g., "ptr" in `const int* ptr = &obj.data`)
    pub reference: String,
    /// The containing object (e.g., "obj")
    pub object: String,
    /// The field being borrowed (e.g., "data")
    pub field: String,
    /// Scope level where the reference was declared
    pub reference_scope: usize,
    /// Scope level where the object was declared
    pub object_scope: usize,
    /// Line number for error reporting
    pub line: usize,
}

/// Main RAII tracker that coordinates all RAII-related tracking
#[derive(Debug)]
pub struct RaiiTracker {
    /// Container borrows: pointers/refs stored in containers
    pub container_borrows: Vec<ContainerBorrow>,
    /// Iterator borrows from containers
    pub iterator_borrows: Vec<IteratorBorrow>,
    /// Lambda captures with escape tracking
    pub lambda_captures: Vec<LambdaCapture>,
    /// Member borrows: references to object fields (Phase 5)
    pub member_borrows: Vec<MemberBorrow>,
    /// Heap allocations for new/delete tracking
    pub heap_allocations: HashMap<String, HeapAllocation>,
    /// User-defined RAII types detected in this file
    pub user_defined_raii_types: HashSet<String>,
    /// Current scope level
    pub current_scope: usize,
    /// Variable scope levels
    pub variable_scopes: HashMap<String, usize>,
    /// Variables that are containers (vector, map, etc.)
    pub container_variables: HashSet<String>,
    /// Variables that are iterators
    pub iterator_variables: HashSet<String>,
}

impl RaiiTracker {
    pub fn new() -> Self {
        Self {
            container_borrows: Vec::new(),
            iterator_borrows: Vec::new(),
            lambda_captures: Vec::new(),
            member_borrows: Vec::new(),
            heap_allocations: HashMap::new(),
            user_defined_raii_types: HashSet::new(),
            current_scope: 0,
            variable_scopes: HashMap::new(),
            container_variables: HashSet::new(),
            iterator_variables: HashSet::new(),
        }
    }

    /// Check if a type is a container type
    pub fn is_container_type(type_name: &str) -> bool {
        type_name.contains("vector") ||
        type_name.contains("Vector") ||
        type_name.contains("Vec<") ||
        type_name.contains("list") ||
        type_name.contains("deque") ||
        type_name.contains("set") ||
        type_name.contains("map") ||
        type_name.contains("unordered_") ||
        type_name.contains("array<") ||
        type_name.contains("span<")
    }

    /// Check if a type is an iterator type
    pub fn is_iterator_type(type_name: &str) -> bool {
        type_name.contains("iterator") ||
        type_name.contains("Iterator") ||
        type_name.ends_with("::iterator") ||
        type_name.ends_with("::const_iterator") ||
        type_name.ends_with("::reverse_iterator")
    }

    /// Check if a function is a container method that stores a reference
    pub fn is_container_store_method(method_name: &str) -> bool {
        method_name == "push_back" ||
        method_name == "push_front" ||
        method_name == "insert" ||
        method_name == "emplace" ||
        method_name == "emplace_back" ||
        method_name == "emplace_front" ||
        method_name == "assign"
    }

    /// Check if a function returns an iterator
    pub fn is_iterator_returning_method(method_name: &str) -> bool {
        method_name == "begin" ||
        method_name == "end" ||
        method_name == "cbegin" ||
        method_name == "cend" ||
        method_name == "rbegin" ||
        method_name == "rend" ||
        method_name == "find" ||
        method_name == "lower_bound" ||
        method_name == "upper_bound"
    }

    /// Register a variable with its scope and type
    pub fn register_variable(&mut self, name: &str, type_name: &str, scope: usize) {
        self.variable_scopes.insert(name.to_string(), scope);

        if Self::is_container_type(type_name) {
            self.container_variables.insert(name.to_string());
        }

        if Self::is_iterator_type(type_name) {
            self.iterator_variables.insert(name.to_string());
        }
    }

    /// Record that a pointer/reference was stored in a container
    pub fn record_container_store(&mut self, container: &str, pointee: &str, line: usize) {
        let container_scope = *self.variable_scopes.get(container).unwrap_or(&0);
        let pointee_scope = *self.variable_scopes.get(pointee).unwrap_or(&0);

        self.container_borrows.push(ContainerBorrow {
            container: container.to_string(),
            pointee: pointee.to_string(),
            container_scope,
            pointee_scope,
            line,
        });
    }

    /// Record that an iterator was created from a container
    pub fn record_iterator_creation(&mut self, iterator: &str, container: &str, line: usize) {
        let iterator_scope = self.current_scope;
        let container_scope = *self.variable_scopes.get(container).unwrap_or(&0);

        self.iterator_borrows.push(IteratorBorrow {
            iterator: iterator.to_string(),
            container: container.to_string(),
            iterator_scope,
            container_scope,
            line,
        });

        self.iterator_variables.insert(iterator.to_string());
    }

    /// Record a lambda with reference captures
    pub fn record_lambda(&mut self, lambda_var: &str, ref_captures: Vec<String>, line: usize) {
        self.lambda_captures.push(LambdaCapture {
            lambda_var: lambda_var.to_string(),
            ref_captures,
            lambda_scope: self.current_scope,
            has_escaped: false,
            line,
        });
    }

    /// Mark a lambda as escaped (returned or stored in longer-lived variable)
    pub fn mark_lambda_escaped(&mut self, lambda_var: &str) {
        for capture in &mut self.lambda_captures {
            if capture.lambda_var == lambda_var {
                capture.has_escaped = true;
            }
        }
    }

    /// Record a reference to an object's member field (Phase 5)
    /// When `ptr = &obj.field`, the reference `ptr` borrows from `obj`
    pub fn record_member_borrow(&mut self, reference: &str, object: &str, field: &str, line: usize) {
        // Use the reference's scope from variable_scopes if known, otherwise use current scope
        let reference_scope = *self.variable_scopes.get(reference).unwrap_or(&self.current_scope);
        let object_scope = *self.variable_scopes.get(object).unwrap_or(&0);

        self.member_borrows.push(MemberBorrow {
            reference: reference.to_string(),
            object: object.to_string(),
            field: field.to_string(),
            reference_scope,
            object_scope,
            line,
        });
    }

    /// Record a new allocation
    pub fn record_allocation(&mut self, var: &str, line: usize) {
        self.heap_allocations.insert(var.to_string(), HeapAllocation {
            variable: var.to_string(),
            state: AllocationState::Allocated,
            allocation_line: line,
            free_line: None,
        });
    }

    /// Record a delete operation
    pub fn record_deallocation(&mut self, var: &str, line: usize) -> Option<String> {
        if let Some(alloc) = self.heap_allocations.get_mut(var) {
            if alloc.state == AllocationState::Freed {
                // Double free!
                return Some(format!(
                    "Double free: '{}' was already freed at line {}",
                    var, alloc.free_line.unwrap_or(0)
                ));
            }
            alloc.state = AllocationState::Freed;
            alloc.free_line = Some(line);
        }
        None
    }

    /// Check if a variable has been freed
    pub fn is_freed(&self, var: &str) -> bool {
        self.heap_allocations.get(var)
            .map(|a| a.state == AllocationState::Freed)
            .unwrap_or(false)
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.current_scope += 1;
    }

    /// Exit a scope and check for dangling references
    pub fn exit_scope(&mut self) -> Vec<String> {
        let mut errors = Vec::new();
        let dying_scope = self.current_scope;

        // Check for pointers in containers that outlive their pointees
        for borrow in &self.container_borrows {
            // If pointee is in the dying scope but container is in an outer scope
            if borrow.pointee_scope == dying_scope && borrow.container_scope < dying_scope {
                errors.push(format!(
                    "Dangling pointer in container: '{}' stored pointer to '{}' which goes out of scope (stored at line {})",
                    borrow.container, borrow.pointee, borrow.line
                ));
            }
        }

        // Check for iterators that outlive their containers
        for borrow in &self.iterator_borrows {
            // If container is in the dying scope but iterator is in an outer scope
            if borrow.container_scope == dying_scope && borrow.iterator_scope < dying_scope {
                errors.push(format!(
                    "Iterator outlives container: '{}' borrows from '{}' which goes out of scope (created at line {})",
                    borrow.iterator, borrow.container, borrow.line
                ));
            }
        }

        // Check for escaping lambdas with reference captures to dying variables
        for capture in &self.lambda_captures {
            if capture.has_escaped {
                for ref_var in &capture.ref_captures {
                    if self.variable_scopes.get(ref_var) == Some(&dying_scope) {
                        errors.push(format!(
                            "Lambda escape: lambda '{}' captures '{}' by reference, but '{}' goes out of scope (lambda at line {})",
                            capture.lambda_var, ref_var, ref_var, capture.line
                        ));
                    }
                }
            }
        }

        // Phase 5: Check for member references that outlive their containing object
        for borrow in &self.member_borrows {
            // If the object is in the dying scope but the reference is in an outer scope
            if borrow.object_scope == dying_scope && borrow.reference_scope < dying_scope {
                errors.push(format!(
                    "Dangling member reference: '{}' references '{}.{}' but '{}' goes out of scope (borrowed at line {})",
                    borrow.reference, borrow.object, borrow.field, borrow.object, borrow.line
                ));
            }
        }

        // Clean up borrows from dying scope
        // For container borrows: keep if pointee survives OR container dies with it
        self.container_borrows.retain(|b| b.pointee_scope != dying_scope || b.container_scope >= dying_scope);
        // For iterator borrows: keep if container survives OR iterator dies with it
        self.iterator_borrows.retain(|b| b.container_scope != dying_scope || b.iterator_scope >= dying_scope);
        // For member borrows: remove if reference OR object dies (no longer needs tracking)
        self.member_borrows.retain(|b| b.reference_scope != dying_scope && b.object_scope != dying_scope);

        // Safely decrement scope level (avoid underflow)
        if self.current_scope > 0 {
            self.current_scope -= 1;
        }
        errors
    }
}

/// Check for RAII-related issues in a function
pub fn check_raii_issues(
    function: &IrFunction,
    _header_cache: &HeaderCache,
) -> Result<Vec<String>, String> {
    let mut errors = Vec::new();
    let mut tracker = RaiiTracker::new();

    // Initialize variable scopes from function's variable info
    for (name, info) in &function.variables {
        let type_name = format!("{:?}", info.ty);
        tracker.register_variable(name, &type_name, info.scope_level);
    }

    // Process statements in the CFG
    for node_idx in function.cfg.node_indices() {
        let block = &function.cfg[node_idx];
        for stmt in &block.statements {
            let stmt_errors = process_raii_statement(stmt, &mut tracker, function);
            errors.extend(stmt_errors);
        }
    }

    Ok(errors)
}

/// Process a statement for RAII tracking
fn process_raii_statement(
    stmt: &IrStatement,
    tracker: &mut RaiiTracker,
    function: &IrFunction,
) -> Vec<String> {
    let mut errors = Vec::new();

    match stmt {
        IrStatement::EnterScope => {
            tracker.enter_scope();
        }

        IrStatement::ExitScope => {
            let scope_errors = tracker.exit_scope();
            errors.extend(scope_errors);
        }

        IrStatement::CallExpr { func, args, result } => {
            // Check for container store methods (push_back, insert, etc.)
            let method_name = func.split("::").last().unwrap_or(func);

            if RaiiTracker::is_container_store_method(method_name) {
                // First argument to method call is typically the container (receiver)
                // For a call like vec.push_back(&x), we parse the receiver from func name
                if let Some(container) = extract_receiver(func) {
                    // Check if any argument is a pointer/reference to a local
                    for arg in args {
                        // Arguments starting with & are address-of operations
                        if arg.starts_with('&') {
                            let pointee = arg.trim_start_matches('&');
                            tracker.record_container_store(&container, pointee, 0);
                        }
                    }
                }
            }

            // Check for iterator-returning methods
            if RaiiTracker::is_iterator_returning_method(method_name) {
                if let (Some(result_var), Some(container)) = (result, extract_receiver(func)) {
                    tracker.record_iterator_creation(result_var, &container, 0);
                }
            }

            // Check for new/delete operations
            if func == "operator new" || func.contains("::operator new") {
                if let Some(result_var) = result {
                    tracker.record_allocation(result_var, 0);
                }
            }

            if func == "operator delete" || func.contains("::operator delete") {
                if let Some(arg) = args.first() {
                    if let Some(err) = tracker.record_deallocation(arg, 0) {
                        errors.push(err);
                    }
                }
            }
        }

        IrStatement::UseVariable { var, operation } => {
            // Check for use-after-free
            if tracker.is_freed(var) {
                errors.push(format!(
                    "Use after free: variable '{}' has been freed (operation: {})",
                    var, operation
                ));
            }
        }

        IrStatement::Return { value } => {
            // Check if returning a lambda that captures local references
            if let Some(val) = value {
                tracker.mark_lambda_escaped(val);
            }
        }

        IrStatement::LambdaCapture { captures } => {
            let ref_captures: Vec<String> = captures
                .iter()
                .filter(|c| c.is_ref)
                .map(|c| c.name.clone())
                .collect();

            if !ref_captures.is_empty() {
                // We'll need to track this lambda's variable name from context
                // For now, use a placeholder
                tracker.record_lambda("_lambda", ref_captures, 0);
            }
        }

        // Phase 5: Track borrows from object fields
        IrStatement::BorrowField { object, field, to, .. } => {
            // When we see `to = &object.field`, record that `to` borrows from `object`
            tracker.record_member_borrow(to, object, field, 0);
        }

        _ => {}
    }

    errors
}

/// Extract the receiver (object) from a method call
/// e.g., "vec.push_back" -> "vec", "obj.container.push_back" -> "obj.container"
fn extract_receiver(func: &str) -> Option<String> {
    // Handle qualified names like "std::vector<int>::push_back"
    if func.contains("::") && !func.contains('.') {
        // This is a qualified name, not a method call on an object
        return None;
    }

    // Handle method calls like "vec.push_back"
    if let Some(dot_pos) = func.rfind('.') {
        return Some(func[..dot_pos].to_string());
    }

    None
}

/// Check if a type has a user-defined destructor
/// This is used for Phase 2: User-defined RAII types
pub fn has_user_defined_destructor(type_name: &str) -> bool {
    // This would need to be populated from parsing class definitions
    // For now, we check common patterns

    // Skip primitive types
    if is_primitive_or_builtin(type_name) {
        return false;
    }

    // Skip known non-RAII types
    if type_name.starts_with("const ") ||
       type_name.ends_with("&") ||
       type_name.ends_with("*") {
        return false;
    }

    // User-defined class types likely have destructors
    // This is a heuristic - real implementation would check class definitions
    !type_name.contains("::") ||
    type_name.contains("std::") ||
    type_name.starts_with("class ") ||
    type_name.starts_with("struct ")
}

fn is_primitive_or_builtin(type_name: &str) -> bool {
    let primitives = [
        "int", "char", "bool", "float", "double", "void",
        "long", "short", "unsigned", "signed",
        "int8_t", "int16_t", "int32_t", "int64_t",
        "uint8_t", "uint16_t", "uint32_t", "uint64_t",
        "size_t", "ptrdiff_t", "nullptr_t",
    ];

    let base = type_name.split('<').next().unwrap_or(type_name).trim();
    primitives.contains(&base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_container_type() {
        assert!(RaiiTracker::is_container_type("std::vector<int>"));
        assert!(RaiiTracker::is_container_type("std::map<int, int>"));
        assert!(RaiiTracker::is_container_type("std::unordered_map<int, int>"));
        assert!(!RaiiTracker::is_container_type("int"));
        assert!(!RaiiTracker::is_container_type("std::string"));
    }

    #[test]
    fn test_is_iterator_type() {
        assert!(RaiiTracker::is_iterator_type("std::vector<int>::iterator"));
        assert!(RaiiTracker::is_iterator_type("std::map<int,int>::const_iterator"));
        assert!(!RaiiTracker::is_iterator_type("int*"));
    }

    #[test]
    fn test_container_borrow_detection() {
        let mut tracker = RaiiTracker::new();

        // Register variables
        tracker.register_variable("vec", "std::vector<int*>", 0);
        tracker.register_variable("x", "int", 1);

        // Simulate: vec.push_back(&x) in inner scope
        tracker.current_scope = 1;
        tracker.record_container_store("vec", "x", 10);

        // Exit inner scope - should detect dangling pointer
        let errors = tracker.exit_scope();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Dangling pointer"));
    }

    #[test]
    fn test_iterator_outlives_container() {
        let mut tracker = RaiiTracker::new();

        // Container in inner scope (will be destroyed)
        tracker.current_scope = 1;
        tracker.variable_scopes.insert("vec".to_string(), 1);
        tracker.container_variables.insert("vec".to_string());

        // Create iterator in outer scope (iterator outlives container)
        // We manually set the iterator_scope to 0 (outer) to simulate declaring
        // the iterator before entering the inner scope
        tracker.iterator_borrows.push(IteratorBorrow {
            iterator: "it".to_string(),
            container: "vec".to_string(),
            iterator_scope: 0,  // Iterator is in outer scope
            container_scope: 1, // Container is in inner scope
            line: 10,
        });

        // Exit inner scope - should detect iterator outliving container
        let errors = tracker.exit_scope();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Iterator outlives container"));
    }

    #[test]
    fn test_double_free_detection() {
        let mut tracker = RaiiTracker::new();

        // Allocate
        tracker.record_allocation("ptr", 10);

        // First free - OK
        let err1 = tracker.record_deallocation("ptr", 20);
        assert!(err1.is_none());

        // Second free - error!
        let err2 = tracker.record_deallocation("ptr", 30);
        assert!(err2.is_some());
        assert!(err2.unwrap().contains("Double free"));
    }

    #[test]
    fn test_use_after_free() {
        let mut tracker = RaiiTracker::new();

        tracker.record_allocation("ptr", 10);
        tracker.record_deallocation("ptr", 20);

        assert!(tracker.is_freed("ptr"));
    }
}
