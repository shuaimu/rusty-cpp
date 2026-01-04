use crate::parser::{CppAst, MethodQualifier};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use crate::debug_println;

/// Parse operator name from a function name
/// Returns the operator symbol (e.g., "*", "=", "==") or None
fn parse_operator_name(func_name: &str) -> Option<&str> {
    // Find the last occurrence of "operator" keyword
    if let Some(pos) = func_name.rfind("operator") {
        let op = &func_name[pos + "operator".len()..];
        if !op.is_empty() {
            return Some(op);
        }
    }
    None
}

/// Check if function is operator* (dereference), not operator*= or operator*
fn is_dereference_operator(func_name: &str) -> bool {
    if let Some(op) = parse_operator_name(func_name) {
        op == "*"
    } else {
        false
    }
}

/// Check if function is operator= (assignment), not operator==, !=, <=, >=
fn is_assignment_operator(func_name: &str) -> bool {
    if let Some(op) = parse_operator_name(func_name) {
        op == "="
    } else {
        false
    }
}

/// Check if function is operator-> (member access)
fn is_member_access_operator(func_name: &str) -> bool {
    if let Some(op) = parse_operator_name(func_name) {
        op == "->"
    } else {
        false
    }
}

/// Check if an expression chain originates from a temporary (constructor call).
/// This handles chained method calls like Builder().set(42).get_value().
/// Returns true if the ultimate receiver is a constructor call (creating a temporary).
fn is_receiver_temporary(expr: &crate::parser::Expression) -> bool {
    match expr {
        // A function call where the name looks like a constructor (ClassName or ClassName::ClassName)
        crate::parser::Expression::FunctionCall { name, args } => {
            // Check if this is a constructor call (name is just a type name, no :: or matches X::X pattern)
            let is_constructor = if name.contains("::") {
                // Check for explicit constructor like Builder::Builder
                let parts: Vec<&str> = name.split("::").collect();
                if parts.len() >= 2 {
                    let last = parts[parts.len() - 1];
                    let second_last = parts[parts.len() - 2];
                    last == second_last  // X::X pattern
                } else {
                    false
                }
            } else {
                // A standalone name like "Builder" is a constructor call
                // if it starts with uppercase (convention for type names)
                name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            };

            if is_constructor {
                return true;
            }

            // For method calls, check if the receiver (first arg) is a temporary
            // Method call pattern: the function name is Class::method and first arg is receiver
            if name.contains("::") && !args.is_empty() {
                // The first argument is the receiver for method calls
                return is_receiver_temporary(&args[0]);
            }

            false
        }
        // Member access on a temporary propagates the temporary status
        crate::parser::Expression::MemberAccess { object, .. } => {
            is_receiver_temporary(object)
        }
        // Dereference of a temporary propagates the temporary status
        crate::parser::Expression::Dereference(inner) => {
            is_receiver_temporary(inner)
        }
        // Variable references are NOT temporaries
        crate::parser::Expression::Variable(_) => false,
        // Literals are temporaries (but they're value types, so less important)
        crate::parser::Expression::Literal(_) => true,
        crate::parser::Expression::StringLiteral(_) => true, // Static lifetime though
        // Other expressions are conservatively considered not temporary
        _ => false,
    }
}

/// Extract the full object path and final field from a nested MemberAccess expression
/// For `o.inner.data`, returns Some(("o.inner", "data"))
/// For `o.field`, returns Some(("o", "field"))
/// For other expressions, returns None
fn extract_member_path(expr: &crate::parser::Expression) -> Option<(String, String)> {
    match expr {
        crate::parser::Expression::MemberAccess { object, field } => {
            match object.as_ref() {
                crate::parser::Expression::Variable(var_name) => {
                    // Simple case: var.field
                    Some((var_name.clone(), field.clone()))
                }
                crate::parser::Expression::MemberAccess { .. } => {
                    // Nested case: obj.path.field - recursively build the path
                    let object_path = extract_full_member_path(object.as_ref())?;
                    Some((object_path, field.clone()))
                }
                _ => None
            }
        }
        _ => None
    }
}

/// Extract the full path string from a MemberAccess chain
/// For `o.inner.data`, returns "o.inner.data"
/// For Variable("x"), returns "x"
fn extract_full_member_path(expr: &crate::parser::Expression) -> Option<String> {
    match expr {
        crate::parser::Expression::Variable(name) => Some(name.clone()),
        crate::parser::Expression::MemberAccess { object, field } => {
            let obj_path = extract_full_member_path(object.as_ref())?;
            Some(format!("{}.{}", obj_path, field))
        }
        _ => None
    }
}

#[derive(Debug, Clone)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    #[allow(dead_code)]
    pub ownership_graph: OwnershipGraph,
    /// RAII Phase 2: Types with user-defined destructors
    pub user_defined_raii_types: std::collections::HashSet<String>,
    /// Struct lifetime tracking: Classes that have reference members
    /// These types implicitly "borrow" from the variables passed to their constructors
    pub types_with_ref_members: std::collections::HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    #[allow(dead_code)]
    pub name: String,
    pub cfg: ControlFlowGraph,
    pub variables: HashMap<String, VariableInfo>,
    pub return_type: String,  // Return type from AST
    pub source_file: String,  // Source file path for distinguishing user code from system headers
    // Method information for tracking 'this' pointer
    pub is_method: bool,
    pub method_qualifier: Option<MethodQualifier>,
    pub class_name: Option<String>,
    // Template information
    pub template_parameters: Vec<String>,  // e.g., ["T", "U"] for template<typename T, typename U>
    // Phase 1: Lifetime information from annotations
    pub lifetime_params: HashMap<String, LifetimeParam>,  // e.g., {"a" -> LifetimeParam, "b" -> LifetimeParam}
    pub param_lifetimes: Vec<Option<ParameterLifetime>>,  // Lifetime for each parameter (indexed by param position)
    pub return_lifetime: Option<ReturnLifetime>,          // Lifetime of return value
    pub lifetime_constraints: Vec<LifetimeConstraint>,    // e.g., 'a: 'b (a outlives b)
}

/// Represents a lifetime parameter declared in the function signature
/// Example: In `@lifetime: (&'a, &'b) -> &'a where 'a: 'b`, we have lifetime params 'a and 'b
#[derive(Debug, Clone, PartialEq)]
pub struct LifetimeParam {
    pub name: String,  // e.g., "a" (without the apostrophe)
}

/// Represents the lifetime annotation of a parameter
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterLifetime {
    pub lifetime_name: String,  // e.g., "a" for &'a T
    pub is_mutable: bool,       // true for &'a mut T, false for &'a T
    pub is_owned: bool,         // true for "owned" annotation
}

/// Represents the lifetime annotation of the return value
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnLifetime {
    pub lifetime_name: String,  // e.g., "a" for &'a T
    pub is_mutable: bool,       // true for &'a mut T, false for &'a T
    pub is_owned: bool,         // true for "owned" annotation
}

/// Represents a lifetime constraint (e.g., 'a: 'b means 'a outlives 'b)
#[derive(Debug, Clone, PartialEq)]
pub struct LifetimeConstraint {
    pub longer: String,   // e.g., "a" in 'a: 'b
    pub shorter: String,  // e.g., "b" in 'a: 'b
}

#[derive(Debug, Clone)]
pub struct VariableInfo {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub ty: VariableType,
    pub ownership: OwnershipState,
    #[allow(dead_code)]
    pub lifetime: Option<Lifetime>,
    pub is_parameter: bool,  // True if this is a function parameter
    pub is_static: bool,     // True if this is a static variable
    pub scope_level: usize,  // Scope depth where variable was declared (0 = function level)
    pub has_destructor: bool, // True if this is an RAII type (Box, Rc, Arc, etc.)
    pub declaration_index: usize, // Order of declaration within scope (for drop order)
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum VariableType {
    Owned(String),           // Type name
    Reference(String),       // Referenced type
    MutableReference(String),
    UniquePtr(String),
    SharedPtr(String),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OwnershipState {
    Owned,
    Borrowed(BorrowKind),
    Moved,
    Uninitialized,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum BorrowKind {
    Immutable,
    Mutable,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lifetime {
    pub name: String,
    pub scope_start: usize,
    pub scope_end: usize,
}

pub type ControlFlowGraph = DiGraph<BasicBlock, ()>;
pub type OwnershipGraph = DiGraph<String, OwnershipEdge>;

#[derive(Debug, Clone)]
pub struct BasicBlock {
    #[allow(dead_code)]
    pub id: usize,
    pub statements: Vec<IrStatement>,
    #[allow(dead_code)]
    pub terminator: Option<Terminator>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum IrStatement {
    Assign {
        lhs: String,
        rhs: IrExpression,
        line: usize,
    },
    Move {
        from: String,
        to: String,
        line: usize,
    },
    Borrow {
        from: String,
        to: String,
        kind: BorrowKind,
        line: usize,
        is_pointer: bool,  // true if borrow via pointer (T* p = &x), false for references (T& r = x)
    },
    CallExpr {
        func: String,
        args: Vec<String>,
        result: Option<String>,
        /// True if this is a method call where the receiver is a temporary expression
        /// (e.g., `Builder().method()` where `Builder()` is a temporary)
        receiver_is_temporary: bool,
    },
    Return {
        value: Option<String>,
        line: usize,
    },
    Drop(String),
    // Scope markers for tracking when blocks begin/end
    EnterScope,
    ExitScope,
    // Loop markers for tracking loop iterations
    EnterLoop,
    ExitLoop,
    // Conditional execution markers
    If {
        then_branch: Vec<IrStatement>,
        else_branch: Option<Vec<IrStatement>>,
    },
    // Safety markers
    EnterUnsafe,
    ExitUnsafe,
    // Phase 4: Pack expansion tracking
    PackExpansion {
        pack_name: String,
        operation: String,  // "forward", "move", or "use"
    },
    // Variable usage (for checking moved state)
    UseVariable {
        var: String,
        operation: String, // "dereference", "method_call", etc.
    },
    // NEW: Field-level operations
    MoveField {
        object: String,      // "container"
        field: String,       // "data"
        to: String,          // "_moved_data"
        line: usize,
    },
    UseField {
        object: String,
        field: String,
        operation: String,   // "read", "write", "call"
    },
    BorrowField {
        object: String,
        field: String,
        to: String,
        kind: BorrowKind,
        line: usize,
    },
    // Implicit drop at scope end (for RAII types)
    ImplicitDrop {
        var: String,
        scope_level: usize,
        has_destructor: bool,  // True if variable is RAII type (should be marked as moved)
    },
    // Lambda expression with captures (for safety checking)
    LambdaCapture {
        captures: Vec<LambdaCaptureInfo>,
    },
    // Variable declaration (for loop-local tracking)
    VarDecl {
        name: String,
        type_name: String,
    },
    /// Struct with reference members borrows from constructor arguments
    /// Like Rust's `Holder<'a>` where struct lifetime is tied to referenced data
    StructBorrow {
        struct_var: String,      // The struct instance (e.g., "h")
        borrowed_from: String,   // Variable passed to constructor (e.g., "x")
        struct_type: String,     // The struct type (e.g., "Holder")
        line: usize,
    },
}

/// Information about a lambda capture
#[derive(Debug, Clone)]
pub struct LambdaCaptureInfo {
    pub name: String,
    pub is_ref: bool,  // true = reference capture, false = copy capture
    pub is_this: bool, // true if capturing 'this'
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum IrExpression {
    Variable(String),
    Move(String),
    Borrow(String, BorrowKind),
    New(String),  // Allocation
    Literal(String),  // Literal value assignment (restores ownership)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Terminator {
    Return(Option<String>),
    Jump(NodeIndex),
    Branch {
        condition: String,
        then_block: NodeIndex,
        else_block: NodeIndex,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum OwnershipEdge {
    Owns,
    Borrows,
    MutBorrows,
}

/// Detect if a type has a non-trivial destructor (RAII type)
/// These types need implicit drop tracking at scope end
fn is_raii_type(type_name: &str) -> bool {
    is_raii_type_with_user_defined(type_name, &std::collections::HashSet::new())
}

/// RAII Phase 2: Check if type is RAII, including user-defined types with destructors
pub fn is_raii_type_with_user_defined(type_name: &str, user_defined_raii_types: &std::collections::HashSet<String>) -> bool {
    // IMPORTANT: References don't have destructors - the referenced object does
    // So a `std::string&` is NOT an RAII type (it's just an alias)
    // References should not be marked as having destructors
    let trimmed = type_name.trim();
    if trimmed.ends_with('&') || trimmed.ends_with("& ") {
        return false;  // References never have destructors
    }
    // Also check for "const T&" pattern where & comes after the base type
    if trimmed.contains('&') && !trimmed.contains('<') {
        // If there's a & but not in template params, it's a reference
        return false;
    }

    // Check for Rusty RAII types (with or without namespace prefix)
    if type_name.starts_with("rusty::Box<") ||
       type_name.starts_with("Box<") ||  // Without namespace
       type_name.starts_with("rusty::Rc<") ||
       type_name.starts_with("Rc<") ||
       type_name.starts_with("rusty::Arc<") ||
       type_name.starts_with("Arc<") ||
       type_name.starts_with("rusty::RefCell<") ||
       type_name.starts_with("RefCell<") ||
       type_name.starts_with("rusty::Cell<") ||
       type_name.starts_with("Cell<") {
        return true;
    }

    // Check for standard library RAII types
    if type_name.starts_with("std::unique_ptr<") ||
       type_name.starts_with("unique_ptr<") ||  // Without namespace
       type_name.starts_with("std::shared_ptr<") ||
       type_name.starts_with("shared_ptr<") ||
       type_name.starts_with("std::weak_ptr<") ||
       type_name.starts_with("weak_ptr<") ||
       type_name.starts_with("std::vector<") ||
       type_name.starts_with("vector<") ||
       type_name.starts_with("std::string") ||
       type_name.starts_with("string") ||
       type_name.starts_with("std::fstream") ||
       type_name.starts_with("fstream") ||
       type_name.starts_with("std::ifstream") ||
       type_name.starts_with("ifstream") ||
       type_name.starts_with("std::ofstream") ||
       type_name.starts_with("ofstream") ||
       type_name.starts_with("std::mutex") ||
       type_name.starts_with("mutex") ||
       type_name.starts_with("std::lock_guard<") ||
       type_name.starts_with("lock_guard<") ||
       type_name.starts_with("std::unique_lock<") ||
       type_name.starts_with("unique_lock<") {
        return true;
    }

    // RAII Phase 2: Check user-defined types with destructors
    // Extract base type name (without template parameters and qualifiers)
    let base_type = type_name
        .split('<').next().unwrap_or(type_name)
        .trim_start_matches("const ")
        .trim_end_matches('&')
        .trim_end_matches('*')
        .trim();

    if user_defined_raii_types.contains(base_type) {
        return true;
    }

    // Also check with common namespace prefixes stripped
    for raii_type in user_defined_raii_types {
        // Check if type_name contains the RAII type name
        if type_name.contains(raii_type) {
            return true;
        }
    }

    false
}

#[allow(dead_code)]
pub fn build_ir(ast: CppAst) -> Result<IrProgram, String> {
    let mut functions = Vec::new();
    let ownership_graph = DiGraph::new();

    // RAII Phase 2: Collect types with user-defined destructors
    let mut user_defined_raii_types = std::collections::HashSet::new();
    // Struct lifetime tracking: Collect types with reference members
    let mut types_with_ref_members = std::collections::HashSet::new();
    for class in &ast.classes {
        if class.has_destructor {
            user_defined_raii_types.insert(class.name.clone());
            debug_println!("RAII: Registered user-defined RAII type '{}'", class.name);
        }
        // Check if class has any reference members
        if class.members.iter().any(|m| m.is_reference) {
            types_with_ref_members.insert(class.name.clone());
            debug_println!("STRUCT_LIFETIME: Type '{}' has reference members", class.name);
        }
    }

    for func in ast.functions {
        let ir_func = convert_function(&func)?;
        functions.push(ir_func);
    }

    Ok(IrProgram {
        functions,
        ownership_graph,
        user_defined_raii_types,
        types_with_ref_members,
    })
}

pub fn build_ir_with_safety_context(
    ast: CppAst,
    _safety_context: crate::parser::safety_annotations::SafetyContext
) -> Result<IrProgram, String> {
    let mut functions = Vec::new();
    let ownership_graph = DiGraph::new();

    // RAII Phase 2: Collect types with user-defined destructors
    let mut user_defined_raii_types = std::collections::HashSet::new();
    // Struct lifetime tracking: Collect types with reference members
    let mut types_with_ref_members = std::collections::HashSet::new();
    for class in &ast.classes {
        if class.has_destructor {
            user_defined_raii_types.insert(class.name.clone());
            debug_println!("RAII: Registered user-defined RAII type '{}'", class.name);
        }
        // Check if class has any reference members
        if class.members.iter().any(|m| m.is_reference) {
            types_with_ref_members.insert(class.name.clone());
            debug_println!("STRUCT_LIFETIME: Type '{}' has reference members", class.name);
        }
    }

    for func in ast.functions {
        let ir_func = convert_function(&func)?;
        functions.push(ir_func);
    }

    Ok(IrProgram {
        functions,
        ownership_graph,
        user_defined_raii_types,
        types_with_ref_members,
    })
}

fn convert_function(func: &crate::parser::Function) -> Result<IrFunction, String> {
    let mut cfg = DiGraph::new();
    let mut variables = HashMap::new();
    let mut current_scope_level = 0; // Track scope depth (0 = function level)

    // Create entry block and convert statements
    let mut statements = Vec::new();

    for stmt in &func.body {
        // Convert the statement
        if let Some(ir_stmts) = convert_statement(stmt, &mut variables, &mut current_scope_level)? {
            statements.extend(ir_stmts);
        }
    }
    
    let entry_block = BasicBlock {
        id: 0,
        statements,
        terminator: None,
    };
    
    let _entry_node = cfg.add_node(entry_block);
    
    // Process parameters
    for param in &func.parameters {
        let (var_type, ownership) = if param.is_unique_ptr {
            (VariableType::UniquePtr(param.type_name.clone()), OwnershipState::Owned)
        } else if param.is_reference {
            if param.is_const {
                (VariableType::Reference(param.type_name.clone()), 
                 OwnershipState::Borrowed(BorrowKind::Immutable))
            } else {
                (VariableType::MutableReference(param.type_name.clone()),
                 OwnershipState::Borrowed(BorrowKind::Mutable))
            }
        } else {
            (VariableType::Owned(param.type_name.clone()), OwnershipState::Owned)
        };
        
        let declaration_index = variables.len();  // Parameters declared in order
        variables.insert(
            param.name.clone(),
            VariableInfo {
                name: param.name.clone(),
                ty: var_type,
                ownership,
                lifetime: None,
                is_parameter: true,  // This is a parameter
                is_static: false,    // Parameters are not static
                scope_level: 0,      // Parameters are at function scope
                has_destructor: is_raii_type(&param.type_name),
                declaration_index,   // NEW: Track declaration order
            },
        );
    }
    
    Ok(IrFunction {
        name: func.name.clone(),
        cfg,
        variables,
        return_type: func.return_type.clone(),
        source_file: func.location.file.clone(),
        is_method: func.is_method,
        method_qualifier: func.method_qualifier.clone(),
        class_name: func.class_name.clone(),
        template_parameters: func.template_parameters.clone(),
        // Phase 1: Initialize lifetime fields (will be populated from annotations)
        lifetime_params: HashMap::new(),
        param_lifetimes: Vec::new(),
        return_lifetime: None,
        lifetime_constraints: Vec::new(),
    })
}

// Helper function to get line number from a statement
#[allow(dead_code)]
fn get_statement_line(stmt: &crate::parser::Statement) -> Option<u32> {
    use crate::parser::Statement;
    match stmt {
        Statement::Assignment { location, .. } => Some(location.line),
        Statement::ReferenceBinding { location, .. } => Some(location.line),
        Statement::FunctionCall { location, .. } => Some(location.line),
        Statement::If { location, .. } => Some(location.line),
        Statement::ExpressionStatement { location, .. } => Some(location.line),
        _ => None,
    }
}

/// Extract the source variable from a return expression, handling all expression types.
/// For complex expressions, this recursively finds the ultimate source variable.
/// Returns None for literals, function calls, and other expressions with no source variable.
fn extract_return_source(
    expr: &crate::parser::Expression,
    statements: &mut Vec<IrStatement>
) -> Option<String> {
    use crate::parser::Expression;

    match expr {
        Expression::Variable(var) => {
            // Simple case: return x;
            Some(var.clone())
        }

        Expression::Dereference(inner) => {
            // Dereference: return *ptr;
            // The source is whatever 'ptr' points to, so recursively extract
            debug_println!("DEBUG IR: Return dereference expression");
            extract_return_source(inner, statements)
        }

        Expression::MemberAccess { object, field } => {
            // Member access: return obj.field; or return this->ptr;
            // The source is the object being accessed
            debug_println!("DEBUG IR: Return member access: {}.{}",
                if let Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                field);
            extract_return_source(object, statements)
        }

        Expression::AddressOf(inner) => {
            // Address-of: return &x;
            // The source is the variable whose address we're taking
            debug_println!("DEBUG IR: Return address-of expression");
            extract_return_source(inner, statements)
        }

        Expression::Move { inner, .. } => {
            // Move: return std::move(x);
            debug_println!("DEBUG IR: Processing Move in return statement");
            match inner.as_ref() {
                Expression::Variable(var) => {
                    debug_println!("DEBUG IR: Return Move(Variable): {}", var);
                    // Generate Move statement
                    statements.push(IrStatement::Move {
                        from: var.clone(),
                        to: format!("_returned_{}", var),
                        line: 0,  // Line not available in this context
                    });
                    Some(var.clone())
                }
                Expression::MemberAccess { object, field } => {
                    debug_println!("DEBUG IR: Return Move(MemberAccess): {}.{}",
                        if let Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                        field);
                    if let Expression::Variable(obj_name) = object.as_ref() {
                        // Generate MoveField statement
                        statements.push(IrStatement::MoveField {
                            object: obj_name.clone(),
                            field: field.clone(),
                            to: format!("_returned_{}", field),
                            line: 0,
                        });
                        Some(format!("{}.{}", obj_name, field))
                    } else {
                        None
                    }
                }
                _ => {
                    // Move of complex expression - try to extract source
                    extract_return_source(inner, statements)
                }
            }
        }

        Expression::FunctionCall { name, args } => {
            // Function call: return foo();
            // This creates a temporary, but for implicit constructors (e.g., return ptr;)
            // the variable might be in the arguments
            debug_println!("DEBUG IR: Return function call: {}", name);

            // Check if any argument is a Move expression (e.g., return Constructor(std::move(x)))
            // This handles cases like: return std::move(data); where the move is wrapped in a constructor
            for arg in args.iter() {
                if let Expression::Move { .. } = arg {
                    debug_println!("DEBUG IR: Found Move inside function call argument");
                    // Recursively extract from the Move
                    return extract_return_source(arg, statements);
                }
            }

            // Method calls (identified by :: in name) create new values that don't reference
            // their receiver. For example: return opt.unwrap()->id;
            //   - opt is moved by unwrap()
            //   - The return value is the result of unwrap(), not opt itself
            //   - So we should NOT track opt as the source
            //
            // Constructor calls (no :: in name) may store references to arguments.
            // For example: return Holder{x};
            //   - Holder might store a reference to x
            //   - We need to track x as the source for dangling reference detection
            let is_method_call = name.contains("::");

            if is_method_call {
                // Method call - receiver is consumed/transformed, result is new value
                None
            } else if let Some(Expression::Variable(var)) = args.first() {
                // Constructor or free function - first arg might be referenced by return value
                Some(var.clone())
            } else {
                None
            }
        }

        Expression::BinaryOp { left, right, op } => {
            // Binary operation: return a + b;
            // These create temporaries, but we could track both operands
            debug_println!("DEBUG IR: Return binary operation: {:?}", op);
            // For now, return None as these are complex temporaries
            // Future: could track both left and right as sources
            None
        }

        Expression::Literal(_) => {
            // Literal: return 42;
            // Literals have no source variable
            None
        }

        Expression::StringLiteral(_) => {
            // String literal: return "hello";
            // String literals have static lifetime and no source variable to track
            None
        }

        Expression::Lambda { .. } => {
            // Lambda: return [captures]() { ... };
            // Lambdas are self-contained closures, no direct source variable
            None
        }

        Expression::Cast(inner) => {
            // Cast: return static_cast<T>(x);
            // The source is whatever is being casted
            debug_println!("DEBUG IR: Return cast expression");
            extract_return_source(inner, statements)
        }

        Expression::Nullptr => {
            // Null pointer literal: return nullptr;
            // No source variable to track
            None
        }

        Expression::New(inner) => {
            // new expression: return new T();
            // This allocates memory - could recursively check inner, but typically
            // new expressions create owned values with no source variable
            debug_println!("DEBUG IR: Return new expression");
            None
        }

        Expression::Delete(inner) => {
            // delete expression: should not appear in return statements
            // but if it does, recursively extract source
            debug_println!("DEBUG IR: Return delete expression");
            extract_return_source(inner, statements)
        }

        Expression::PointerArithmetic { pointer, .. } => {
            // Pointer arithmetic: return p + n;
            // The source is the pointer being manipulated
            debug_println!("DEBUG IR: Return pointer arithmetic expression");
            extract_return_source(pointer, statements)
        }
    }
}

fn convert_statement(
    stmt: &crate::parser::Statement,
    variables: &mut HashMap<String, VariableInfo>,
    current_scope_level: &mut usize,
) -> Result<Option<Vec<IrStatement>>, String> {
    use crate::parser::Statement;

    debug_println!("DEBUG IR: Converting statement: {:?}", match stmt {
        Statement::VariableDecl(_) => "VariableDecl",
        Statement::Assignment { .. } => "Assignment",
        Statement::ReferenceBinding { .. } => "ReferenceBinding",
        Statement::Return(_) => "Return",
        Statement::FunctionCall { name, .. } => {
            debug_println!("DEBUG IR:   FunctionCall name: {}", name);
            "FunctionCall"
        },
        Statement::ExpressionStatement { .. } => "ExpressionStatement",
        Statement::If { condition, .. } => {
            debug_println!("DEBUG IR:   If condition: {:?}", condition);
            "If"
        },
        _ => "Other"
    });

    match stmt {
        Statement::VariableDecl(var) => {
            let (var_type, ownership) = if var.is_unique_ptr {
                (VariableType::UniquePtr(var.type_name.clone()), OwnershipState::Owned)
            } else if var.is_reference {
                if var.is_const {
                    (VariableType::Reference(var.type_name.clone()),
                     OwnershipState::Uninitialized) // Will be set when bound
                } else {
                    (VariableType::MutableReference(var.type_name.clone()),
                     OwnershipState::Uninitialized)
                }
            } else {
                (VariableType::Owned(var.type_name.clone()), OwnershipState::Owned)
            };
            
            let has_destructor_value = is_raii_type(&var.type_name);
            let declaration_index = variables.len();  // Current count = declaration order
            debug_println!("DEBUG IR: VariableDecl '{}': type='{}', has_destructor={}, declaration_index={}",
                var.name, var.type_name, has_destructor_value, declaration_index);

            variables.insert(
                var.name.clone(),
                VariableInfo {
                    name: var.name.clone(),
                    ty: var_type,
                    ownership,
                    lifetime: None,
                    is_parameter: false,  // This is a local variable
                    is_static: var.is_static,  // Propagate static status from parser
                    scope_level: *current_scope_level,  // Track scope depth
                    has_destructor: has_destructor_value,
                    declaration_index,  // NEW: Track declaration order
                },
            );
            // Generate VarDecl IR statement for loop-local tracking
            Ok(Some(vec![IrStatement::VarDecl {
                name: var.name.clone(),
                type_name: var.type_name.clone(),
            }]))
        }
        Statement::ReferenceBinding { name, target, is_mutable, location } => {
            let mut statements = Vec::new();
            let line = location.line as usize;

            match target {
                // Reference to a variable: create a borrow
                crate::parser::Expression::Variable(target_var) => {
                    let kind = if *is_mutable {
                        BorrowKind::Mutable
                    } else {
                        BorrowKind::Immutable
                    };

                    // Update the reference variable's ownership state and type
                    if let Some(var_info) = variables.get_mut(name) {
                        var_info.ownership = OwnershipState::Borrowed(kind.clone());
                        // Update the type to reflect this is a reference
                        if *is_mutable {
                            if let VariableType::Owned(type_name) = &var_info.ty {
                                var_info.ty = VariableType::MutableReference(type_name.clone());
                            }
                        } else {
                            if let VariableType::Owned(type_name) = &var_info.ty {
                                var_info.ty = VariableType::Reference(type_name.clone());
                            }
                        }
                    }

                    statements.push(IrStatement::Borrow {
                        from: target_var.clone(),
                        to: name.clone(),
                        kind,
                        line,
                        is_pointer: false,  // Reference binding
                    });
                },

                // Reference to function call result: create CallExpr with result
                crate::parser::Expression::FunctionCall { name: func_name, args } => {
                    let mut arg_names = Vec::new();
                    let mut temp_counter = 0;
                    // Track if the first argument (method receiver) is a field access
                    // This is needed to create BorrowField when method result is assigned to reference
                    let mut receiver_field: Option<(String, String)> = None; // (object_path, field_name)

                    // Check if the receiver (first arg for method calls) is a temporary
                    // This detects patterns like Builder().set(42).get_value()
                    let receiver_is_temp = if func_name.contains("::") && !args.is_empty() {
                        // For method calls, check if the receiver (first arg) originates from a temporary
                        is_receiver_temporary(&args[0])
                    } else {
                        false
                    };

                    // Process arguments
                    for (i, arg) in args.iter().enumerate() {
                        match arg {
                            crate::parser::Expression::Variable(var) => {
                                arg_names.push(var.clone());
                            }
                            crate::parser::Expression::Move { inner, .. } => {
                                if let crate::parser::Expression::Variable(var) = inner.as_ref() {
                                    statements.push(IrStatement::Move {
                                        from: var.clone(),
                                        to: format!("_moved_{}", var),
                                        line,
                                    });
                                    arg_names.push(var.clone());
                                }
                            }
                            // Track literals as temporaries for lifetime analysis
                            crate::parser::Expression::Literal(lit) => {
                                let temp_name = format!("_temp_literal_{}_{}", temp_counter, lit);
                                temp_counter += 1;
                                arg_names.push(temp_name);
                            }
                            // Track string literals - they have static lifetime
                            crate::parser::Expression::StringLiteral(_lit) => {
                                let temp_name = format!("_temp_string_literal_{}", temp_counter);
                                temp_counter += 1;
                                arg_names.push(temp_name);
                            }
                            // Track binary expressions as temporaries (e.g., a + b)
                            crate::parser::Expression::BinaryOp { .. } => {
                                let temp_name = format!("_temp_expr_{}", temp_counter);
                                temp_counter += 1;
                                arg_names.push(temp_name);
                            }
                            // For chained method calls, the receiver might be a FunctionCall
                            crate::parser::Expression::FunctionCall { name: inner_name, .. } => {
                                // Use the function name as a placeholder for the temporary
                                let temp_name = format!("_temp_call_{}", inner_name.replace("::", "_"));
                                arg_names.push(temp_name);
                            }
                            // Phase 3: Track MemberAccess as receiver for field borrow tracking
                            crate::parser::Expression::MemberAccess { .. } => {
                                if let Some((obj_path, field_name)) = extract_member_path(arg) {
                                    // If this is the first arg (method receiver), track it for field borrowing
                                    if i == 0 && func_name.contains("::") {
                                        debug_println!("DEBUG IR: Method receiver is field access: {}.{}", obj_path, field_name);
                                        receiver_field = Some((obj_path.clone(), field_name.clone()));
                                    }
                                    arg_names.push(format!("{}.{}", obj_path, field_name));
                                }
                            }
                            _ => {}
                        }
                    }

                    // Special handling for operator* (dereference)
                    // When we have: int& r = *box;
                    // This creates a reference that borrows from the box
                    if is_dereference_operator(&func_name) {
                        if let Some(first_arg) = arg_names.first() {
                            let kind = if *is_mutable {
                                BorrowKind::Mutable
                            } else {
                                BorrowKind::Immutable
                            };

                            debug_println!("DEBUG IR: ReferenceBinding via operator* creates borrow from '{}'", first_arg);

                            // Create a Borrow from the object being dereferenced
                            statements.push(IrStatement::Borrow {
                                from: first_arg.clone(),
                                to: name.clone(),
                                kind: kind.clone(),
                                line,
                                is_pointer: false,  // Reference binding via operator*
                            });

                            // Update the reference variable's ownership state
                            if let Some(var_info) = variables.get_mut(name) {
                                var_info.ownership = OwnershipState::Borrowed(kind);
                            }

                            // Don't create CallExpr for operator* - Borrow is sufficient
                        } else {
                            // No arguments - shouldn't happen for operator*
                            debug_println!("DEBUG IR: operator* with no arguments");
                        }
                    } else {
                        // For other function calls, create CallExpr
                        debug_println!("DEBUG IR: Creating CallExpr for '{}' with receiver_is_temporary={}", func_name, receiver_is_temp);
                        statements.push(IrStatement::CallExpr {
                            func: func_name.clone(),
                            args: arg_names,
                            result: Some(name.clone()),
                            receiver_is_temporary: receiver_is_temp,
                        });

                        // Phase 3: If method was called on a field and result is assigned to reference,
                        // create BorrowField to track that the reference borrows from the field.
                        // This detects patterns like: const string& ref = o.inner.get();
                        // where 'ref' should borrow from 'o.inner'
                        if let Some((obj_path, field_name)) = receiver_field {
                            let kind = if *is_mutable {
                                BorrowKind::Mutable
                            } else {
                                BorrowKind::Immutable
                            };

                            debug_println!("DEBUG IR: Method result '{}' borrows from field '{}.{}'", name, obj_path, field_name);
                            statements.push(IrStatement::BorrowField {
                                object: obj_path,
                                field: field_name,
                                to: name.clone(),
                                kind: kind.clone(),
                                line,
                            });

                            // Update the reference variable's ownership state
                            if let Some(var_info) = variables.get_mut(name) {
                                var_info.ownership = OwnershipState::Borrowed(kind);
                            }
                        } else {
                            // No receiver field - just update ownership state
                            if let Some(var_info) = variables.get_mut(name) {
                                let kind = if *is_mutable {
                                    BorrowKind::Mutable
                                } else {
                                    BorrowKind::Immutable
                                };
                                var_info.ownership = OwnershipState::Borrowed(kind);
                            }
                        }
                    }
                },

                // Reference to a field: create a field borrow
                // Supports both simple (p.field) and nested (o.inner.field) member access
                crate::parser::Expression::MemberAccess { object, field } => {
                    // Use helper to extract full object path for nested access
                    if let Some((obj_path, final_field)) = extract_member_path(target) {
                        debug_println!("DEBUG IR: ReferenceBinding to field: {}.{}", obj_path, final_field);

                        let kind = if *is_mutable {
                            BorrowKind::Mutable
                        } else {
                            BorrowKind::Immutable
                        };

                        // Update the reference variable's ownership state and type
                        if let Some(var_info) = variables.get_mut(name) {
                            var_info.ownership = OwnershipState::Borrowed(kind.clone());
                            // Update the type to reflect this is a reference
                            if *is_mutable {
                                if let VariableType::Owned(type_name) = &var_info.ty {
                                    var_info.ty = VariableType::MutableReference(type_name.clone());
                                }
                            } else {
                                if let VariableType::Owned(type_name) = &var_info.ty {
                                    var_info.ty = VariableType::Reference(type_name.clone());
                                }
                            }
                        }

                        // Generate BorrowField IR statement with full nested path
                        statements.push(IrStatement::BorrowField {
                            object: obj_path,
                            field: final_field,
                            to: name.clone(),
                            kind,
                            line,
                        });
                    } else if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                        // Fallback for simple Variable case
                        debug_println!("DEBUG IR: ReferenceBinding to field (simple): {}.{}", obj_name, field);

                        let kind = if *is_mutable {
                            BorrowKind::Mutable
                        } else {
                            BorrowKind::Immutable
                        };

                        if let Some(var_info) = variables.get_mut(name) {
                            var_info.ownership = OwnershipState::Borrowed(kind.clone());
                            if *is_mutable {
                                if let VariableType::Owned(type_name) = &var_info.ty {
                                    var_info.ty = VariableType::MutableReference(type_name.clone());
                                }
                            } else {
                                if let VariableType::Owned(type_name) = &var_info.ty {
                                    var_info.ty = VariableType::Reference(type_name.clone());
                                }
                            }
                        }

                        statements.push(IrStatement::BorrowField {
                            object: obj_name.clone(),
                            field: field.clone(),
                            to: name.clone(),
                            kind,
                            line,
                        });
                    }
                },

                _ => return Ok(None),
            }

            Ok(Some(statements))
        }
        Statement::Assignment { lhs, rhs, location } => {
            let line = location.line as usize;
            // Check if lhs is a dereference: *ptr = value
            if let crate::parser::Expression::Dereference(ptr_expr) = lhs {
                // Dereference assignment: *ptr = value
                if let crate::parser::Expression::Variable(ptr_var) = ptr_expr.as_ref() {
                    // Extract the RHS variable
                    let _value_var = match rhs {
                        crate::parser::Expression::Variable(v) => v.clone(),
                        _ => return Ok(None), // For now, only handle simple cases
                    };

                    // Create a UseVariable statement to check that ptr is valid
                    return Ok(Some(vec![IrStatement::UseVariable {
                        var: ptr_var.clone(),
                        operation: "dereference_write".to_string(),
                    }]));
                }
                return Ok(None);
            }

            // Check if lhs is a function call (e.g., *ptr via operator*)
            if let crate::parser::Expression::FunctionCall { name, args } = lhs {
                debug_println!("DEBUG IR: Assignment LHS is function call: {}", name);
                // Check if this is operator* (dereference for smart pointers)
                if is_dereference_operator(&name) {
                    debug_println!("DEBUG IR: Detected operator* on LHS, args: {:?}", args);
                    // This is a dereference assignment via operator*
                    // The first argument is the object being dereferenced
                    if let Some(crate::parser::Expression::Variable(ptr_var)) = args.first() {
                        debug_println!("DEBUG IR: Creating UseVariable for dereference_write on '{}'", ptr_var);
                        // Create a UseVariable statement to check that ptr is valid
                        return Ok(Some(vec![IrStatement::UseVariable {
                            var: ptr_var.clone(),
                            operation: "dereference_write (via operator*)".to_string(),
                        }]));
                    }
                }
                // Other method calls on LHS are not supported for now
                debug_println!("DEBUG IR: Unsupported function call on LHS");
                return Ok(None);
            }

            // Check if LHS is a field access (e.g., this.value = 42)
            if let crate::parser::Expression::MemberAccess { object, field } = lhs {
                debug_println!("DEBUG IR: Field write assignment: {}.{} = ...",
                    if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                    field);

                if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                    // Generate UseField statement for write operation
                    return Ok(Some(vec![
                        IrStatement::UseField {
                            object: obj_name.clone(),
                            field: field.clone(),
                            operation: "write".to_string(),
                        }
                    ]));
                } else {
                    return Ok(None);
                }
            }

            // Regular assignment (not a dereference)
            let lhs_var = match lhs {
                crate::parser::Expression::Variable(v) => v,
                _ => return Ok(None), // Skip complex lhs for now
            };

            // SPECIAL CASE: Check if LHS is an RAII type (Box, Rc, Arc, etc.)
            // For RAII types, assignment is operator= which:
            // 1. Drops the old value (checked if borrowed)
            // 2. Moves new value in
            // This applies when RHS is Move or creates a new object
            let lhs_is_raii = if let Some(lhs_info) = variables.get(lhs_var) {
                match &lhs_info.ty {
                    VariableType::Owned(type_name) => is_raii_type(type_name),
                    _ => false,
                }
            } else {
                false
            };

            // Track if we need to prepend a Drop check for RAII reassignment
            let mut prepend_drop = false;

            if lhs_is_raii {
                debug_println!("DEBUG IR: Assignment to RAII type '{}', this is operator= (drops old value)", lhs_var);

                // For RAII types, assignment is operator= which drops the old value.
                // We need to check if LHS is borrowed before allowing this.

                // Handle Move expression: box = std::move(other)
                if let crate::parser::Expression::Move { inner, .. } = rhs {
                    match inner.as_ref() {
                        crate::parser::Expression::Variable(from_var) => {
                            debug_println!("DEBUG IR: RAII assignment with std::move: generating Move from '{}' to '{}'", from_var, lhs_var);

                            // Generate Move statement - this will check if LHS is borrowed!
                            // Move already handles the drop implicitly
                            return Ok(Some(vec![IrStatement::Move {
                                from: from_var.clone(),
                                to: lhs_var.clone(),
                        line: 0,
                    }]));
                        }
                        _ => {
                            // Move of complex expression - continue to regular handling
                            debug_println!("DEBUG IR: Move of complex expression");
                        }
                    }
                }

                // For other cases (constructor calls like Box::make), we need to:
                // 1. Generate a Drop check (to verify not borrowed)
                // 2. Generate the actual assignment IR
                debug_println!("DEBUG IR: Will prepend Drop check for RAII assignment");
                prepend_drop = true;
            }

            let assignment_ir = match rhs {
                crate::parser::Expression::Dereference(ptr_expr) => {
                    // Dereference read: lhs = *ptr
                    if let crate::parser::Expression::Variable(ptr_var) = ptr_expr.as_ref() {
                        // Create a UseVariable statement to check that ptr is valid
                        Ok(Some(vec![IrStatement::UseVariable {
                            var: ptr_var.clone(),
                            operation: "dereference_read".to_string(),
                        }]))
                    } else {
                        Ok(None)
                    }
                }
                crate::parser::Expression::Variable(rhs_var) => {
                    // Check if this is a move or a copy
                    if let Some(rhs_info) = variables.get(rhs_var) {
                        match &rhs_info.ty {
                            VariableType::UniquePtr(_) => {
                                // This is a move
                                Ok(Some(vec![IrStatement::Move {
                                    from: rhs_var.clone(),
                                    to: lhs_var.clone(),
                        line: 0,
                    }]))
                            }
                            _ => {
                                // Regular assignment (copy)
                                Ok(Some(vec![IrStatement::Assign {
                                    lhs: lhs_var.clone(),
                                    rhs: IrExpression::Variable(rhs_var.clone()),
                        line: 0,
                    }]))
                            }
                        }
                    } else {
                        Ok(None)
                    }
                }
                // NEW: Handle field access (not a move) - including nested fields
                crate::parser::Expression::MemberAccess { .. } => {
                    // Use helper to extract full path for nested member access
                    if let Some((obj_path, field_name)) = extract_member_path(rhs) {
                        debug_println!("DEBUG IR: Processing MemberAccess read from '{}.{}'", obj_path, field_name);
                        Ok(Some(vec![
                            IrStatement::UseField {
                                object: obj_path.clone(),
                                field: field_name.clone(),
                                operation: "read".to_string(),
                            },
                            IrStatement::Assign {
                                lhs: lhs_var.clone(),
                                rhs: IrExpression::Variable(format!("{}.{}", obj_path, field_name)),
                                line,
                            }
                        ]))
                    } else {
                        debug_println!("DEBUG IR: MemberAccess could not be parsed");
                        Ok(None)
                    }
                }
                crate::parser::Expression::Move { inner, .. } => {
                    debug_println!("DEBUG IR: Processing Move expression in assignment");
                    // This is an explicit std::move call
                    match inner.as_ref() {
                        crate::parser::Expression::Variable(var) => {
                            debug_println!("DEBUG IR: Creating IrStatement::Move from '{}' to '{}'", var, lhs_var);
                            // Transfer type from source if needed
                            let source_type = variables.get(var).map(|info| info.ty.clone());
                            if let Some(var_info) = variables.get_mut(lhs_var) {
                                if let Some(ty) = source_type {
                                    var_info.ty = ty;
                                }
                            }
                            Ok(Some(vec![IrStatement::Move {
                                from: var.clone(),
                                to: lhs_var.clone(),
                        line: 0,
                    }]))
                        }
                        // NEW: Handle std::move(obj.field) including nested fields
                        crate::parser::Expression::MemberAccess { .. } => {
                            // Use helper to extract full path for nested member access
                            if let Some((obj_path, field_name)) = extract_member_path(inner.as_ref()) {
                                debug_println!("DEBUG IR: Creating MoveField for field '{}' of object '{}'", field_name, obj_path);
                                Ok(Some(vec![IrStatement::MoveField {
                                    object: obj_path,
                                    field: field_name,
                                    to: lhs_var.clone(),
                        line: 0,
                    }]))
                            } else {
                                debug_println!("DEBUG IR: MemberAccess could not be parsed");
                                Ok(None)
                            }
                        }
                        _ => {
                            debug_println!("DEBUG IR: Move expression doesn't contain a variable or member access");
                            Ok(None)
                        }
                    }
                }
                crate::parser::Expression::FunctionCall { name, args } => {
                    // Convert function call arguments, handling moves
                    let mut statements = Vec::new();
                    let mut arg_names = Vec::new();
                    let mut temp_counter = 0;

                    // Check if this is a method call (operator* or other methods)
                    // Methods can be: qualified (Class::method), operators (operator*, operator bool), or have :: in name
                    let is_method_call = name.contains("::") || name.starts_with("operator");

                    for (i, arg) in args.iter().enumerate() {
                        match arg {
                            // Track literals as temporaries for lifetime analysis
                            crate::parser::Expression::Literal(lit) => {
                                let temp_name = format!("_temp_literal_{}_{}", temp_counter, lit);
                                temp_counter += 1;
                                arg_names.push(temp_name);
                            }
                            // Track string literals - they have static lifetime
                            crate::parser::Expression::StringLiteral(lit) => {
                                let temp_name = format!("_temp_string_literal_{}", temp_counter);
                                temp_counter += 1;
                                arg_names.push(temp_name);
                            }
                            // Track binary expressions as temporaries (e.g., a + b)
                            crate::parser::Expression::BinaryOp { .. } => {
                                let temp_name = format!("_temp_expr_{}", temp_counter);
                                temp_counter += 1;
                                arg_names.push(temp_name);
                            }
                            crate::parser::Expression::Variable(var) => {
                                // For method calls, the first arg is the receiver object
                                if is_method_call && i == 0 {
                                    // Check if this is operator* (dereference)
                                    if is_dereference_operator(&name) {
                                        statements.push(IrStatement::UseVariable {
                                            var: var.clone(),
                                            operation: "dereference_read (via operator*)".to_string(),
                                        });
                                    } else {
                                        // Other method calls also use the receiver
                                        statements.push(IrStatement::UseVariable {
                                            var: var.clone(),
                                            operation: format!("call method '{}'", name),
                                        });
                                    }
                                }
                                arg_names.push(var.clone());
                            }
                            crate::parser::Expression::Move { inner, .. } => {
                                // Handle std::move in constructor/function arguments
                                debug_println!("DEBUG IR: Processing Move in assignment RHS function call");
                                match inner.as_ref() {
                                    crate::parser::Expression::Variable(var) => {
                                        debug_println!("DEBUG IR: Move(Variable) in assignment: {}", var);

                                        // CRITICAL FIX: When Move is the receiver of a method call (first argument),
                                        // use the temporary as the receiver instead of the original variable.
                                        // This allows calling && methods on rvalue expressions like std::move(c).consume()
                                        let temp_name = format!("_moved_{}", var);
                                        statements.push(IrStatement::Move {
                                            from: var.clone(),
                                            to: temp_name.clone(),
                        line: 0,
                    });

                                        if is_method_call && i == 0 {
                                            // Use the temporary as the receiver for rvalue method calls
                                            debug_println!("DEBUG IR: Move as method receiver - using temporary '{}' instead of '{}'", temp_name, var);
                                            arg_names.push(temp_name);
                                        } else {
                                            // For non-receiver arguments, use the original variable name
                                            arg_names.push(var.clone());
                                        }
                                    }
                                    crate::parser::Expression::MemberAccess { .. } => {
                                        // Use helper to extract full path for nested member access
                                        if let Some((obj_path, field_name)) = extract_member_path(inner.as_ref()) {
                                            debug_println!("DEBUG IR: Move(MemberAccess) in assignment: {}.{}", obj_path, field_name);
                                            statements.push(IrStatement::MoveField {
                                                object: obj_path.clone(),
                                                field: field_name.clone(),
                                                to: lhs_var.clone(),  // Move to the LHS variable
                                                line: 0,  // Line not easily available in this nested context
                                            });
                                            arg_names.push(format!("{}.{}", obj_path, field_name));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            crate::parser::Expression::FunctionCall { name: recv_name, args: recv_args } if is_method_call && i == 0 => {
                                // Receiver is a method call itself (e.g., ptr->method() where ptr-> is operator->)
                                debug_println!("DEBUG IR: Receiver is FunctionCall: {}", recv_name);

                                // Check if this is operator-> (pointer dereference for method call)
                                if is_member_access_operator(&recv_name) {
                                    // Extract the actual pointer variable from operator-> args
                                    for recv_arg in recv_args {
                                        if let crate::parser::Expression::Variable(var) = recv_arg {
                                            debug_println!("DEBUG IR: Found pointer variable in operator->: {}", var);
                                            statements.push(IrStatement::UseVariable {
                                                var: var.clone(),
                                                operation: format!("call method '{}' via operator->", name),
                                            });
                                        }
                                    }
                                }
                                arg_names.push(format!("_result_of_{}", recv_name));
                            }
                            crate::parser::Expression::Move { inner, .. } => {
                                if let crate::parser::Expression::Variable(var) = inner.as_ref() {
                                    // Mark as moved before the call
                                    statements.push(IrStatement::Move {
                                        from: var.clone(),
                                        to: format!("_temp_move_{}", var),
                                        line: 0,  // Line not easily available in this nested context
                                    });
                                    arg_names.push(var.clone());
                                }
                            }
                            // NEW: Handle field access as function argument (including nested)
                            crate::parser::Expression::MemberAccess { .. } => {
                                // Use helper to extract full path for nested member access
                                if let Some((obj_path, field_name)) = extract_member_path(arg) {
                                    debug_println!("DEBUG IR: MemberAccess as function argument in assignment: {}.{}", obj_path, field_name);
                                    // Generate UseField statement to check if field is valid
                                    statements.push(IrStatement::UseField {
                                        object: obj_path.clone(),
                                        field: field_name.clone(),
                                        operation: "use in function call".to_string(),
                                    });
                                    arg_names.push(format!("{}.{}", obj_path, field_name));
                                }
                            }
                            _ => {}
                        }
                    }

                    statements.push(IrStatement::CallExpr {
                        func: name.clone(),
                        args: arg_names,
                        result: Some(lhs_var.clone()),
                        receiver_is_temporary: false,  // TODO: detect temporaries
                    });

                    Ok(Some(statements))
                }
                // REASSIGNMENT FIX: Handle literal assignments (e.g., x = 42)
                // This generates IR so that ownership can be properly restored
                crate::parser::Expression::Literal(value) => {
                    debug_println!("DEBUG IR: Literal assignment: {} = {}", lhs_var, value);
                    Ok(Some(vec![IrStatement::Assign {
                        lhs: lhs_var.clone(),
                        rhs: IrExpression::Literal(value.clone()),
                        line: 0,
                    }]))
                }
                // String literal assignment (e.g., const char* s = "hello")
                // String literals have static lifetime - this is safe
                crate::parser::Expression::StringLiteral(value) => {
                    debug_println!("DEBUG IR: String literal assignment: {} = \"{}\"", lhs_var, value);
                    Ok(Some(vec![IrStatement::Assign {
                        lhs: lhs_var.clone(),
                        rhs: IrExpression::Literal(value.clone()),  // Treat as literal for IR
                        line,
                    }]))
                }
                // Lambda expression: generate LambdaCapture statement for safety checking
                crate::parser::Expression::Lambda { captures } => {
                    debug_println!("DEBUG IR: Lambda assignment: {} = [captures]", lhs_var);
                    let capture_infos: Vec<LambdaCaptureInfo> = captures.iter().map(|c| {
                        use crate::parser::ast_visitor::LambdaCaptureKind;
                        match c {
                            LambdaCaptureKind::DefaultRef => LambdaCaptureInfo {
                                name: "<default>".to_string(),
                                is_ref: true,
                                is_this: false,
                            },
                            LambdaCaptureKind::DefaultCopy => LambdaCaptureInfo {
                                name: "<default>".to_string(),
                                is_ref: false,
                                is_this: false,
                            },
                            LambdaCaptureKind::ByRef(name) => LambdaCaptureInfo {
                                name: name.clone(),
                                is_ref: true,
                                is_this: false,
                            },
                            LambdaCaptureKind::ByCopy(name) => LambdaCaptureInfo {
                                name: name.clone(),
                                is_ref: false,
                                is_this: false,
                            },
                            LambdaCaptureKind::Init { name, is_move } => LambdaCaptureInfo {
                                name: name.clone(),
                                is_ref: false, // Init captures are by value
                                is_this: false,
                            },
                            LambdaCaptureKind::This => LambdaCaptureInfo {
                                name: "this".to_string(),
                                is_ref: true, // 'this' capture is a pointer, essentially by-ref
                                is_this: true,
                            },
                            LambdaCaptureKind::ThisCopy => LambdaCaptureInfo {
                                name: "this".to_string(),
                                is_ref: false, // *this capture is by value
                                is_this: true,
                            },
                        }
                    }).collect();

                    Ok(Some(vec![IrStatement::LambdaCapture {
                        captures: capture_infos,
                    }]))
                }
                // NEW: Handle pointer initialization from address-of: T* p = &x
                // This creates a borrow from x to p (pointer borrows the address of x)
                crate::parser::Expression::AddressOf(inner) => {
                    debug_println!("DEBUG IR: AddressOf in assignment: {} = &...", lhs_var);
                    match inner.as_ref() {
                        crate::parser::Expression::Variable(target_var) => {
                            debug_println!("DEBUG IR: Creating pointer borrow from '{}' to '{}'", target_var, lhs_var);

                            // Determine mutability from LHS pointer type
                            // If LHS is `const T*` -> Immutable, otherwise -> Mutable
                            let kind = if let Some(var_info) = variables.get(lhs_var) {
                                // Check if this is a const pointer (const T*)
                                match &var_info.ty {
                                    VariableType::Owned(type_name) if type_name.starts_with("const ") => {
                                        BorrowKind::Immutable
                                    }
                                    _ => {
                                        // Non-const pointer defaults to mutable borrow
                                        // In safe C++, T* p = &x means we might modify *p
                                        BorrowKind::Mutable
                                    }
                                }
                            } else {
                                // Default to mutable for unknown types
                                BorrowKind::Mutable
                            };

                            Ok(Some(vec![IrStatement::Borrow {
                                from: target_var.clone(),
                                to: lhs_var.clone(),
                                kind,
                                line,
                                is_pointer: true,  // Mark as pointer borrow
                            }]))
                        }
                        // Handle &obj.field (address of a field)
                        crate::parser::Expression::MemberAccess { object, field } => {
                            if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                                debug_println!("DEBUG IR: Creating pointer borrow from '{}.{}' to '{}'", obj_name, field, lhs_var);

                                // Same mutability logic as above
                                let kind = if let Some(var_info) = variables.get(lhs_var) {
                                    match &var_info.ty {
                                        VariableType::Owned(type_name) if type_name.starts_with("const ") => {
                                            BorrowKind::Immutable
                                        }
                                        _ => BorrowKind::Mutable
                                    }
                                } else {
                                    BorrowKind::Mutable
                                };

                                // For field borrows, we track against the whole object
                                // (partial borrow tracking is handled separately)
                                Ok(Some(vec![IrStatement::Borrow {
                                    from: obj_name.clone(),
                                    to: lhs_var.clone(),
                                    kind,
                                    line,
                                    is_pointer: true,
                                }]))
                            } else {
                                Ok(None)
                            }
                        }
                        _ => Ok(None)
                    }
                }
                _ => Ok(None)
            };

            // If we need to prepend a Drop check for RAII reassignment, do it now
            if prepend_drop {
                debug_println!("DEBUG IR: Prepending Drop check to assignment IR");
                match assignment_ir {
                    Ok(Some(mut stmts)) => {
                        // Prepend Drop to the existing statements
                        stmts.insert(0, IrStatement::Drop(lhs_var.clone()));
                        Ok(Some(stmts))
                    }
                    Ok(None) => {
                        // No assignment IR generated, just return Drop
                        Ok(Some(vec![IrStatement::Drop(lhs_var.clone())]))
                    }
                    Err(e) => Err(e)
                }
            } else {
                assignment_ir
            }
        }
        Statement::FunctionCall { name, args, location } => {
            debug_println!("DEBUG IR: Processing FunctionCall statement: {} with {} args", name, args.len());
            let line = location.line as usize;
            // Standalone function call (no assignment)
            let mut statements = Vec::new();
            let mut arg_names = Vec::new();
            let mut temp_counter = 0;

            // Check if this is a method call (has :: or is an operator)
            // Methods can be: qualified (Class::method), operators (operator*, operator bool), or have :: in name
            let is_method_call = name.contains("::") || name.starts_with("operator");

            // Special handling for operator= (assignment operators)
            // box1 = std::move(box2) becomes operator=(box1, Move(box2))
            // We need to treat this as: Move { from: box2, to: box1 }
            if is_assignment_operator(&name) {
                debug_println!("DEBUG IR: Detected operator= call");
                if args.len() == 2 {
                    // First arg is LHS (destination), second is RHS (source)
                    if let crate::parser::Expression::Variable(lhs) = &args[0] {
                        // Check if LHS is an RAII type
                        let lhs_is_raii = if let Some(lhs_info) = variables.get(lhs) {
                            match &lhs_info.ty {
                                VariableType::Owned(type_name) => is_raii_type(type_name),
                                _ => false,
                            }
                        } else {
                            false
                        };

                        // Handle Move RHS
                        if let crate::parser::Expression::Move { inner: rhs_inner, .. } = &args[1] {
                            debug_println!("DEBUG IR: operator= with Move: {} = Move(...)", lhs);
                            if let crate::parser::Expression::Variable(rhs) = rhs_inner.as_ref() {
                                debug_println!("DEBUG IR: Creating Move from '{}' to '{}' for operator=", rhs, lhs);
                                return Ok(Some(vec![IrStatement::Move {
                                    from: rhs.clone(),
                                    to: lhs.clone(),
                        line: 0,
                    }]));
                            }
                        }

                        // For RAII types with non-move RHS, we need to check borrows before drop
                        if lhs_is_raii {
                            debug_println!("DEBUG IR: operator= on RAII type '{}' - generating Drop check", lhs);
                            // Generate Drop check - the FunctionCall itself will be processed below
                            statements.push(IrStatement::Drop(lhs.clone()));
                        }
                    }
                }
            }

            // Process arguments, looking for std::move
            for (i, arg) in args.iter().enumerate() {
                match arg {
                    crate::parser::Expression::Variable(var) => {
                        // Generate UseVariable for all function call arguments
                        // This enables use-after-move detection for any variable passed to a function
                        let operation = if is_method_call && i == 0 {
                            // For method receivers, check if this is operator* (dereference)
                            if is_dereference_operator(&name) {
                                "dereference (via operator*)".to_string()
                            } else {
                                format!("call method '{}'", name)
                            }
                        } else {
                            // For regular function arguments
                            format!("pass to function '{}'", name)
                        };

                        statements.push(IrStatement::UseVariable {
                            var: var.clone(),
                            operation,
                        });
                        arg_names.push(var.clone());
                    }
                    crate::parser::Expression::Move { inner, .. } => {
                        // Handle std::move in function arguments
                        match inner.as_ref() {
                            crate::parser::Expression::Variable(var) => {
                                debug_println!("DEBUG IR: Move(Variable) as direct argument: {}", var);

                                // CRITICAL FIX: When Move is the receiver of a method call (first argument),
                                // use the temporary as the receiver instead of the original variable.
                                // This allows calling && methods on rvalue expressions like std::move(c).consume()
                                let temp_name = format!("_moved_{}", var);
                                statements.push(IrStatement::Move {
                                    from: var.clone(),
                                    to: temp_name.clone(),
                        line: 0,
                    });

                                if is_method_call && i == 0 {
                                    // Use the temporary as the receiver for rvalue method calls
                                    debug_println!("DEBUG IR: Move as method receiver - using temporary '{}' instead of '{}'", temp_name, var);
                                    arg_names.push(temp_name);
                                } else {
                                    // For non-receiver arguments, use the original variable name
                                    arg_names.push(var.clone());
                                }
                            }
                            crate::parser::Expression::MemberAccess { .. } => {
                                // Use helper to extract full path for nested member access
                                if let Some((obj_path, field_name)) = extract_member_path(inner.as_ref()) {
                                    debug_println!("DEBUG IR: Move(MemberAccess) as direct argument: {}.{}", obj_path, field_name);
                                    statements.push(IrStatement::MoveField {
                                        object: obj_path.clone(),
                                        field: field_name.clone(),
                                        to: format!("_moved_{}", field_name),
                                        line,
                                    });
                                    arg_names.push(format!("{}.{}", obj_path, field_name));
                                }
                            }
                            _ => {}
                        }
                    }
                    crate::parser::Expression::FunctionCall { name: inner_name, args: inner_args } => {
                        debug_println!("DEBUG IR: Nested FunctionCall in argument: {}", inner_name);

                        // Check if this is the receiver of a method call (i == 0)
                        if is_method_call && i == 0 {
                            // Check if this is operator-> (pointer dereference for method call)
                            if is_member_access_operator(&inner_name) {
                                // Extract the actual pointer variable from operator-> args
                                for inner_arg in inner_args {
                                    if let crate::parser::Expression::Variable(var) = inner_arg {
                                        debug_println!("DEBUG IR: Found pointer variable in operator->: {}", var);
                                        statements.push(IrStatement::UseVariable {
                                            var: var.clone(),
                                            operation: format!("call method '{}' via operator->", name),
                                        });
                                    }
                                }
                            }
                        }

                        // Recursively check for moves in nested function call
                        for inner_arg in inner_args {
                            if let crate::parser::Expression::Move { inner: move_inner, .. } = inner_arg {
                                match move_inner.as_ref() {
                                    crate::parser::Expression::Variable(var) => {
                                        debug_println!("DEBUG IR: Found Move(Variable) in nested call: {}", var);
                                        statements.push(IrStatement::Move {
                                            from: var.clone(),
                                            to: format!("_moved_{}", var),
                                            line,
                                        });
                                    }
                                    crate::parser::Expression::MemberAccess { .. } => {
                                        // Use helper to extract full path for nested member access
                                        if let Some((obj_path, field_name)) = extract_member_path(move_inner.as_ref()) {
                                            debug_println!("DEBUG IR: Found Move(MemberAccess) in nested call: {}.{}", obj_path, field_name);
                                            statements.push(IrStatement::MoveField {
                                                object: obj_path,
                                                field: field_name.clone(),
                                                to: format!("_moved_{}", field_name),
                                                line,
                                            });
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        // Use placeholder for nested call result
                        arg_names.push(format!("_result_of_{}", inner_name));
                    }
                    // NEW: Handle field access as function argument (including nested)
                    crate::parser::Expression::MemberAccess { .. } => {
                        // Use helper to extract full path for nested member access
                        if let Some((obj_path, field_name)) = extract_member_path(arg) {
                            debug_println!("DEBUG IR: MemberAccess as function argument: {}.{}", obj_path, field_name);

                            // When field is receiver of a method call, we need to check for conflicts
                            // with existing borrows but NOT create a persistent borrow
                            // (method receiver borrows are temporary, ending when the call returns)
                            if is_method_call && i == 0 {
                                // For method receiver: generate UseField with method name for better error messages
                                // Also check for conflicts with existing borrows of this field
                                statements.push(IrStatement::UseField {
                                    object: obj_path.clone(),
                                    field: field_name.clone(),
                                    operation: format!("call method '{}' on field", name),
                                });
                            } else {
                                // For regular function argument: just check if field is valid
                                statements.push(IrStatement::UseField {
                                    object: obj_path.clone(),
                                    field: field_name.clone(),
                                    operation: "use in function call".to_string(),
                                });
                            }
                            arg_names.push(format!("{}.{}", obj_path, field_name));
                        }
                    }
                    // Track literals as temporaries for lifetime analysis
                    crate::parser::Expression::Literal(lit) => {
                        let temp_name = format!("_temp_literal_{}_{}", temp_counter, lit);
                        temp_counter += 1;
                        arg_names.push(temp_name);
                    }
                    // Track string literals - they have static lifetime
                    crate::parser::Expression::StringLiteral(lit) => {
                        let temp_name = format!("_temp_string_literal_{}", temp_counter);
                        temp_counter += 1;
                        arg_names.push(temp_name);
                    }
                    // Track binary expressions as temporaries (e.g., a + b)
                    crate::parser::Expression::BinaryOp { .. } => {
                        let temp_name = format!("_temp_expr_{}", temp_counter);
                        temp_counter += 1;
                        arg_names.push(temp_name);
                    }
                    _ => {}
                }
            }

            statements.push(IrStatement::CallExpr {
                func: name.clone(),
                args: arg_names,
                result: None,
                receiver_is_temporary: false,  // TODO: detect temporaries
            });

            Ok(Some(statements))
        }
        Statement::Return(expr) => {
            let mut statements = Vec::new();

            let value = expr.as_ref().and_then(|e| {
                extract_return_source(e, &mut statements)
            });

            statements.push(IrStatement::Return { value , line: 0 });
            Ok(Some(statements))
        }
        Statement::EnterScope => {
            *current_scope_level += 1;
            debug_println!("DEBUG IR: EnterScope - now at level {}", current_scope_level);
            Ok(Some(vec![IrStatement::EnterScope]))
        }
        Statement::ExitScope => {
            debug_println!("DEBUG IR: ExitScope - leaving level {}", current_scope_level);
            debug_println!("DEBUG IR: Total variables: {}", variables.len());
            for (name, info) in variables.iter() {
                debug_println!("DEBUG IR:   Variable '{}': scope_level={}, has_destructor={}, is_static={}",
                    name, info.scope_level, info.has_destructor, info.is_static);
            }

            // Find ALL variables declared at this scope level (including references)
            // We need to clear borrows for all variables, not just RAII types
            let mut vars_to_drop: Vec<(String, usize, bool)> = variables
                .iter()
                .filter(|(_, info)| {
                    info.scope_level == *current_scope_level &&
                    !info.is_static  // Static variables are never dropped
                })
                .map(|(name, info)| (name.clone(), info.declaration_index, info.has_destructor))
                .collect();

            // Sort by reverse declaration order (highest index first)
            // This ensures variables drop in reverse order of declaration
            vars_to_drop.sort_by(|a, b| b.1.cmp(&a.1));

            debug_println!("DROP ORDER: Processing {} variables at scope end in reverse declaration order", vars_to_drop.len());
            for (name, decl_idx, has_dest) in &vars_to_drop {
                debug_println!("DROP ORDER:   '{}' (declaration_index={}, has_destructor={})", name, decl_idx, has_dest);
            }

            // Create ImplicitDrop statements in reverse declaration order
            // This clears borrows for ALL variables, and marks RAII types as moved
            let mut statements = Vec::new();
            for (var, _, has_dest) in vars_to_drop {
                debug_println!("DEBUG IR: Inserting ImplicitDrop for '{}' at scope level {} (has_destructor={})",
                    var, current_scope_level, has_dest);
                statements.push(IrStatement::ImplicitDrop {
                    var,
                    scope_level: *current_scope_level,
                    has_destructor: has_dest,
                });
            }

            // Add the ExitScope marker after the drops
            statements.push(IrStatement::ExitScope);

            *current_scope_level = current_scope_level.saturating_sub(1);
            Ok(Some(statements))
        }
        Statement::EnterLoop => {
            Ok(Some(vec![IrStatement::EnterLoop]))
        }
        Statement::ExitLoop => {
            Ok(Some(vec![IrStatement::ExitLoop]))
        }
        Statement::EnterUnsafe => {
            Ok(Some(vec![IrStatement::EnterUnsafe]))
        }
        Statement::ExitUnsafe => {
            Ok(Some(vec![IrStatement::ExitUnsafe]))
        }
        Statement::If { condition, then_branch, else_branch, .. } => {
            // Convert if statement to IR
            // First, process the condition (which might contain uses like `if (ptr)`)
            let mut condition_ir = Vec::new();

            // Extract uses from the condition expression
            match condition {
                crate::parser::Expression::FunctionCall { name, args } => {
                    // Method call in condition (e.g., if (ptr.operator bool()))
                    let is_method_call = name.contains("::") || name.starts_with("operator");

                    for (i, arg) in args.iter().enumerate() {
                        if let crate::parser::Expression::Variable(var) = arg {
                            // For method calls, first arg is the receiver
                            if is_method_call && i == 0 {
                                debug_println!("DEBUG IR: Creating UseVariable for '{}' in condition (method: {})", var, name);
                                condition_ir.push(IrStatement::UseVariable {
                                    var: var.clone(),
                                    operation: format!("call method '{}' in condition", name),
                                });
                            }
                        }
                    }
                }
                crate::parser::Expression::Variable(var) => {
                    // Direct variable use in condition
                    condition_ir.push(IrStatement::UseVariable {
                        var: var.clone(),
                        operation: "use in condition".to_string(),
                    });
                }
                _ => {
                    // Other expression types - ignore for now
                }
            }

            // Convert then branch
            let mut then_ir = Vec::new();
            for stmt in then_branch {
                if let Some(ir_stmts) = convert_statement(stmt, variables, current_scope_level)? {
                    then_ir.extend(ir_stmts);
                }
            }

            // Convert else branch if present
            let else_ir = if let Some(else_stmts) = else_branch {
                let mut else_ir = Vec::new();
                for stmt in else_stmts {
                    if let Some(ir_stmts) = convert_statement(stmt, variables, current_scope_level)? {
                        else_ir.extend(ir_stmts);
                    }
                }
                Some(else_ir)
            } else {
                None
            };

            // Return condition uses followed by the If statement
            let mut result = condition_ir;
            result.push(IrStatement::If {
                then_branch: then_ir,
                else_branch: else_ir,
            });
            Ok(Some(result))
        }
        Statement::ExpressionStatement { expr, .. } => {
            // Handle expression statements (dereference, method calls, assignments, etc.)
            match expr {
                crate::parser::Expression::Dereference(inner) => {
                    // Extract the variable being dereferenced
                    if let crate::parser::Expression::Variable(var) = inner.as_ref() {
                        Ok(Some(vec![IrStatement::UseVariable {
                            var: var.clone(),
                            operation: "dereference".to_string(),
                        }]))
                    } else {
                        Ok(None)
                    }
                }
                crate::parser::Expression::AddressOf(inner) => {
                    // Address-of doesn't use the value, so no moved-state check needed
                    Ok(None)
                }
                // Handle assignment expressions (e.g., value = 42;)
                crate::parser::Expression::BinaryOp { left, op, right } if op == "=" => {
                    debug_println!("DEBUG IR: ExpressionStatement assignment: op={}", op);

                    // Check if LHS is a field access (e.g., this.value = 42)
                    if let crate::parser::Expression::MemberAccess { object, field } = left.as_ref() {
                        debug_println!("DEBUG IR: ExpressionStatement field write: {}.{} = ...",
                            if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                            field);

                        if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                            // Generate UseField statement for write operation
                            return Ok(Some(vec![
                                IrStatement::UseField {
                                    object: obj_name.clone(),
                                    field: field.clone(),
                                    operation: "write".to_string(),
                                }
                            ]));
                        }
                    }

                    // For other assignments, fall through
                    Ok(None)
                }
                _ => Ok(None),
            }
        }
        Statement::PackExpansion { pack_name, operation, .. } => {
            // Phase 4: Convert pack expansion to IR
            debug_println!("DEBUG IR: PackExpansion: pack='{}', operation='{}'", pack_name, operation);
            Ok(Some(vec![IrStatement::PackExpansion {
                pack_name: pack_name.clone(),
                operation: operation.clone(),
            }]))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Function, Variable, SourceLocation};

    fn create_test_function(name: &str) -> Function {
        Function {
            name: name.to_string(),
            parameters: vec![],
            return_type: "void".to_string(),
            body: vec![],
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 1,
                column: 1,
            },
            is_method: false,
            method_qualifier: None,
            class_name: None,
            template_parameters: vec![],
            safety_annotation: None,
            has_explicit_safety_annotation: false,
        }
    }

    fn create_test_variable(name: &str, type_name: &str, is_unique_ptr: bool) -> Variable {
        Variable {
            name: name.to_string(),
            type_name: type_name.to_string(),
            is_reference: false,
            is_pointer: false,
            is_const: false,
            is_unique_ptr,
            is_shared_ptr: false,
            is_static: false,
            is_mutable: false,
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 1,
                column: 1,
            },
            is_pack: false,
            pack_element_type: None,
        }
    }

    #[test]
    fn test_build_empty_ir() {
        let ast = crate::parser::CppAst::new();
        let result = build_ir(ast);
        
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 0);
    }

    #[test]
    fn test_build_ir_with_function() {
        let mut ast = crate::parser::CppAst::new();
        ast.functions.push(create_test_function("test_func"));
        
        let result = build_ir(ast);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "test_func");
    }

    #[test]
    fn test_variable_type_classification() {
        let unique_var = create_test_variable("ptr", "std::unique_ptr<int>", true);
        let mut ast = crate::parser::CppAst::new();
        let mut func = create_test_function("test");
        func.parameters.push(unique_var);
        ast.functions.push(func);
        
        let result = build_ir(ast);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        let var_info = ir.functions[0].variables.get("ptr").unwrap();
        assert!(matches!(var_info.ty, VariableType::UniquePtr(_)));
    }

    #[test]
    fn test_ownership_state_initialization() {
        let var = create_test_variable("x", "int", false);
        let mut ast = crate::parser::CppAst::new();
        let mut func = create_test_function("test");
        func.parameters.push(var);
        ast.functions.push(func);
        
        let result = build_ir(ast);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        let var_info = ir.functions[0].variables.get("x").unwrap();
        assert_eq!(var_info.ownership, OwnershipState::Owned);
    }

    #[test]
    fn test_lifetime_creation() {
        let lifetime = Lifetime {
            name: "a".to_string(),
            scope_start: 0,
            scope_end: 10,
        };

        assert_eq!(lifetime.name, "a");
        assert_eq!(lifetime.scope_start, 0);
        assert_eq!(lifetime.scope_end, 10);
    }
}

// Phase 1: Conversion functions from parsed annotations to IR lifetime types

/// Convert LifetimeAnnotation to ParameterLifetime for IR
fn convert_param_lifetime(annotation: &crate::parser::annotations::LifetimeAnnotation) -> Option<ParameterLifetime> {
    use crate::parser::annotations::LifetimeAnnotation;

    match annotation {
        LifetimeAnnotation::Ref(lifetime_name) => {
            Some(ParameterLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: false,
                is_owned: false,
            })
        }
        LifetimeAnnotation::MutRef(lifetime_name) => {
            Some(ParameterLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: true,
                is_owned: false,
            })
        }
        LifetimeAnnotation::Ptr(lifetime_name) => {
            // Mutable pointer - similar to MutRef
            Some(ParameterLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: true,
                is_owned: false,
            })
        }
        LifetimeAnnotation::ConstPtr(lifetime_name) => {
            // Const pointer - similar to Ref
            Some(ParameterLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: false,
                is_owned: false,
            })
        }
        LifetimeAnnotation::Owned => {
            Some(ParameterLifetime {
                lifetime_name: String::new(),  // No specific lifetime for owned
                is_mutable: false,
                is_owned: true,
            })
        }
        LifetimeAnnotation::Lifetime(_) => {
            // Bare lifetime parameter like 'a - not applicable to parameter type
            None
        }
    }
}

/// Convert LifetimeAnnotation to ReturnLifetime for IR
fn convert_return_lifetime(annotation: &crate::parser::annotations::LifetimeAnnotation) -> Option<ReturnLifetime> {
    use crate::parser::annotations::LifetimeAnnotation;

    match annotation {
        LifetimeAnnotation::Ref(lifetime_name) => {
            Some(ReturnLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: false,
                is_owned: false,
            })
        }
        LifetimeAnnotation::MutRef(lifetime_name) => {
            Some(ReturnLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: true,
                is_owned: false,
            })
        }
        LifetimeAnnotation::Ptr(lifetime_name) => {
            // Mutable pointer return - similar to MutRef
            Some(ReturnLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: true,
                is_owned: false,
            })
        }
        LifetimeAnnotation::ConstPtr(lifetime_name) => {
            // Const pointer return - similar to Ref
            Some(ReturnLifetime {
                lifetime_name: lifetime_name.clone(),
                is_mutable: false,
                is_owned: false,
            })
        }
        LifetimeAnnotation::Owned => {
            Some(ReturnLifetime {
                lifetime_name: String::new(),  // No specific lifetime for owned
                is_mutable: false,
                is_owned: true,
            })
        }
        LifetimeAnnotation::Lifetime(_) => {
            // Bare lifetime parameter like 'a - not applicable to return type
            None
        }
    }
}

/// Populate IrFunction lifetime fields from FunctionSignature annotations
pub fn populate_lifetime_info(
    ir_func: &mut IrFunction,
    signature: &crate::parser::annotations::FunctionSignature
) {
    debug_println!("DEBUG IR LIFETIME: Populating lifetime info for function '{}'", ir_func.name);

    // Extract all unique lifetime names from parameters and return type
    let mut lifetime_names = std::collections::HashSet::new();

    // Collect lifetime names from parameters
    for param_lifetime_opt in &signature.param_lifetimes {
        if let Some(param_lifetime) = param_lifetime_opt {
            if let Some(name) = extract_lifetime_name_from_annotation(param_lifetime) {
                lifetime_names.insert(name);
            }
        }
    }

    // Collect lifetime name from return type
    if let Some(return_lifetime) = &signature.return_lifetime {
        if let Some(name) = extract_lifetime_name_from_annotation(return_lifetime) {
            lifetime_names.insert(name);
        }
    }

    // Collect lifetime names from bounds
    for bound in &signature.lifetime_bounds {
        lifetime_names.insert(bound.longer.clone());
        lifetime_names.insert(bound.shorter.clone());
    }

    // Create LifetimeParam entries
    for name in lifetime_names {
        debug_println!("DEBUG IR LIFETIME:   Lifetime parameter: '{}'", name);
        ir_func.lifetime_params.insert(
            name.clone(),
            LifetimeParam { name }
        );
    }

    // Convert parameter lifetimes
    for param_lifetime_opt in &signature.param_lifetimes {
        let converted = param_lifetime_opt.as_ref()
            .and_then(|lt| convert_param_lifetime(lt));

        if let Some(ref param_lt) = converted {
            debug_println!("DEBUG IR LIFETIME:   Parameter lifetime: '{}' (mutable={}, owned={})",
                param_lt.lifetime_name, param_lt.is_mutable, param_lt.is_owned);
        }

        ir_func.param_lifetimes.push(converted);
    }

    // Convert return lifetime
    ir_func.return_lifetime = signature.return_lifetime.as_ref()
        .and_then(|lt| convert_return_lifetime(lt));

    if let Some(ref ret_lt) = ir_func.return_lifetime {
        debug_println!("DEBUG IR LIFETIME:   Return lifetime: '{}' (mutable={}, owned={})",
            ret_lt.lifetime_name, ret_lt.is_mutable, ret_lt.is_owned);
    }

    // Convert lifetime constraints
    for bound in &signature.lifetime_bounds {
        debug_println!("DEBUG IR LIFETIME:   Lifetime constraint: '{}': '{}'", bound.longer, bound.shorter);
        ir_func.lifetime_constraints.push(LifetimeConstraint {
            longer: bound.longer.clone(),
            shorter: bound.shorter.clone(),
        });
    }

    debug_println!("DEBUG IR LIFETIME: Populated {} lifetime params, {} param lifetimes, {} constraints",
        ir_func.lifetime_params.len(), ir_func.param_lifetimes.len(), ir_func.lifetime_constraints.len());
}

/// Extract lifetime name from a LifetimeAnnotation
fn extract_lifetime_name_from_annotation(annotation: &crate::parser::annotations::LifetimeAnnotation) -> Option<String> {
    use crate::parser::annotations::LifetimeAnnotation;

    match annotation {
        LifetimeAnnotation::Ref(name) => Some(name.clone()),
        LifetimeAnnotation::MutRef(name) => Some(name.clone()),
        LifetimeAnnotation::Ptr(name) => Some(name.clone()),
        LifetimeAnnotation::ConstPtr(name) => Some(name.clone()),
        LifetimeAnnotation::Lifetime(name) => Some(name.trim_start_matches('\'').to_string()),
        LifetimeAnnotation::Owned => None,
    }
}