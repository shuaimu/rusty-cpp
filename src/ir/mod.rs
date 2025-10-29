use crate::parser::{CppAst, MethodQualifier};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use crate::debug_println;

#[derive(Debug, Clone)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
    #[allow(dead_code)]
    pub ownership_graph: OwnershipGraph,
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    #[allow(dead_code)]
    pub name: String,
    pub cfg: ControlFlowGraph,
    pub variables: HashMap<String, VariableInfo>,
    pub return_type: String,  // Return type from AST
    // Method information for tracking 'this' pointer
    pub is_method: bool,
    pub method_qualifier: Option<MethodQualifier>,
    pub class_name: Option<String>,
    // Template information
    pub template_parameters: Vec<String>,  // e.g., ["T", "U"] for template<typename T, typename U>
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
    },
    Move {
        from: String,
        to: String,
    },
    Borrow {
        from: String,
        to: String,
        kind: BorrowKind,
    },
    CallExpr {
        func: String,
        args: Vec<String>,
        result: Option<String>,
    },
    Return {
        value: Option<String>,
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
    },
    // Implicit drop at scope end (for RAII types)
    ImplicitDrop {
        var: String,
        scope_level: usize,
        has_destructor: bool,  // True if variable is RAII type (should be marked as moved)
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum IrExpression {
    Variable(String),
    Move(String),
    Borrow(String, BorrowKind),
    New(String),  // Allocation
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

    // Conservative: assume user-defined classes might have destructors
    // In the future, we could parse class definitions to check
    false
}

#[allow(dead_code)]
pub fn build_ir(ast: CppAst) -> Result<IrProgram, String> {
    let mut functions = Vec::new();
    let ownership_graph = DiGraph::new();
    
    for func in ast.functions {
        let ir_func = convert_function(&func)?;
        functions.push(ir_func);
    }
    
    Ok(IrProgram {
        functions,
        ownership_graph,
    })
}

pub fn build_ir_with_safety_context(
    ast: CppAst,
    _safety_context: crate::parser::safety_annotations::SafetyContext
) -> Result<IrProgram, String> {
    let mut functions = Vec::new();
    let ownership_graph = DiGraph::new();
    
    for func in ast.functions {
        let ir_func = convert_function(&func)?;
        functions.push(ir_func);
    }
    
    Ok(IrProgram {
        functions,
        ownership_graph,
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
        is_method: func.is_method,
        method_qualifier: func.method_qualifier.clone(),
        class_name: func.class_name.clone(),
        template_parameters: func.template_parameters.clone(),
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
            Ok(None)
        }
        Statement::ReferenceBinding { name, target, is_mutable, .. } => {
            let mut statements = Vec::new();

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
                    });
                },

                // Reference to function call result: create CallExpr with result
                crate::parser::Expression::FunctionCall { name: func_name, args } => {
                    let mut arg_names = Vec::new();

                    // Process arguments
                    for arg in args {
                        match arg {
                            crate::parser::Expression::Variable(var) => {
                                arg_names.push(var.clone());
                            }
                            crate::parser::Expression::Move(inner) => {
                                if let crate::parser::Expression::Variable(var) = inner.as_ref() {
                                    statements.push(IrStatement::Move {
                                        from: var.clone(),
                                        to: format!("_moved_{}", var),
                                    });
                                    arg_names.push(var.clone());
                                }
                            }
                            _ => {}
                        }
                    }

                    // Special handling for operator* (dereference)
                    // When we have: int& r = *box;
                    // This creates a reference that borrows from the box
                    if func_name.contains("::operator*") || func_name == "operator*" {
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
                        statements.push(IrStatement::CallExpr {
                            func: func_name.clone(),
                            args: arg_names,
                            result: Some(name.clone()),
                        });

                        // Update the reference variable's ownership state
                        if let Some(var_info) = variables.get_mut(name) {
                            let kind = if *is_mutable {
                                BorrowKind::Mutable
                            } else {
                                BorrowKind::Immutable
                            };
                            var_info.ownership = OwnershipState::Borrowed(kind);
                        }
                    }
                },

                // Reference to a field: create a field borrow
                crate::parser::Expression::MemberAccess { object, field } => {
                    debug_println!("DEBUG IR: ReferenceBinding to field: {}.{}",
                        if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                        field);

                    if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
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

                        // Generate BorrowField IR statement
                        statements.push(IrStatement::BorrowField {
                            object: obj_name.clone(),
                            field: field.clone(),
                            to: name.clone(),
                            kind,
                        });
                    }
                },

                _ => return Ok(None),
            }

            Ok(Some(statements))
        }
        Statement::Assignment { lhs, rhs, .. } => {
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
                if name.contains("::operator*") || name == "operator*" {
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
                if let crate::parser::Expression::Move(inner) = rhs {
                    match inner.as_ref() {
                        crate::parser::Expression::Variable(from_var) => {
                            debug_println!("DEBUG IR: RAII assignment with std::move: generating Move from '{}' to '{}'", from_var, lhs_var);

                            // Generate Move statement - this will check if LHS is borrowed!
                            // Move already handles the drop implicitly
                            return Ok(Some(vec![IrStatement::Move {
                                from: from_var.clone(),
                                to: lhs_var.clone(),
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
                                }]))
                            }
                            _ => {
                                // Regular assignment (copy)
                                Ok(Some(vec![IrStatement::Assign {
                                    lhs: lhs_var.clone(),
                                    rhs: IrExpression::Variable(rhs_var.clone()),
                                }]))
                            }
                        }
                    } else {
                        Ok(None)
                    }
                }
                // NEW: Handle field access (not a move)
                crate::parser::Expression::MemberAccess { object, field } => {
                    debug_println!("DEBUG IR: Processing MemberAccess read from '{}.{}'",
                        if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                        field);
                    // Extract object name
                    if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                        Ok(Some(vec![
                            IrStatement::UseField {
                                object: obj_name.clone(),
                                field: field.clone(),
                                operation: "read".to_string(),
                            },
                            IrStatement::Assign {
                                lhs: lhs_var.clone(),
                                rhs: IrExpression::Variable(format!("{}.{}", obj_name, field)),
                            }
                        ]))
                    } else {
                        debug_println!("DEBUG IR: MemberAccess object is not a simple variable");
                        Ok(None)
                    }
                }
                crate::parser::Expression::Move(inner) => {
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
                            }]))
                        }
                        // NEW: Handle std::move(obj.field)
                        crate::parser::Expression::MemberAccess { object, field } => {
                            debug_println!("DEBUG IR: Creating MoveField for field '{}' of object", field);
                            // Extract object name
                            if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                                Ok(Some(vec![IrStatement::MoveField {
                                    object: obj_name.clone(),
                                    field: field.clone(),
                                    to: lhs_var.clone(),
                                }]))
                            } else {
                                debug_println!("DEBUG IR: MemberAccess object is not a simple variable");
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

                    // Check if this is a method call (operator* or other methods)
                    // Methods can be: qualified (Class::method), operators (operator*, operator bool), or have :: in name
                    let is_method_call = name.contains("::") || name.starts_with("operator");

                    for (i, arg) in args.iter().enumerate() {
                        match arg {
                            crate::parser::Expression::Variable(var) => {
                                // For method calls, the first arg is the receiver object
                                if is_method_call && i == 0 {
                                    // Check if this is operator* (dereference)
                                    if name.contains("::operator*") || name == "operator*" {
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
                            crate::parser::Expression::Move(inner) => {
                                // Handle std::move in constructor/function arguments
                                debug_println!("DEBUG IR: Processing Move in assignment RHS function call");
                                match inner.as_ref() {
                                    crate::parser::Expression::Variable(var) => {
                                        debug_println!("DEBUG IR: Move(Variable) in assignment: {}", var);
                                        statements.push(IrStatement::Move {
                                            from: var.clone(),
                                            to: format!("_moved_{}", var),
                                        });
                                        arg_names.push(var.clone());
                                    }
                                    crate::parser::Expression::MemberAccess { object, field } => {
                                        debug_println!("DEBUG IR: Move(MemberAccess) in assignment: {}.{}",
                                            if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                                            field);
                                        if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                                            statements.push(IrStatement::MoveField {
                                                object: obj_name.clone(),
                                                field: field.clone(),
                                                to: lhs_var.clone(),  // Move to the LHS variable
                                            });
                                            arg_names.push(format!("{}.{}", obj_name, field));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            crate::parser::Expression::FunctionCall { name: recv_name, args: recv_args } if is_method_call && i == 0 => {
                                // Receiver is a method call itself (e.g., ptr->method() where ptr-> is operator->)
                                debug_println!("DEBUG IR: Receiver is FunctionCall: {}", recv_name);

                                // Check if this is operator-> (pointer dereference for method call)
                                if recv_name.contains("::operator->") || recv_name == "operator->" {
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
                            crate::parser::Expression::Move(inner) => {
                                if let crate::parser::Expression::Variable(var) = inner.as_ref() {
                                    // Mark as moved before the call
                                    statements.push(IrStatement::Move {
                                        from: var.clone(),
                                        to: format!("_temp_move_{}", var),
                                    });
                                    arg_names.push(var.clone());
                                }
                            }
                            // NEW: Handle field access as function argument
                            crate::parser::Expression::MemberAccess { object, field } => {
                                debug_println!("DEBUG IR: MemberAccess as function argument in assignment: {}.{}",
                                    if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                                    field);
                                // Generate UseField statement to check if field is valid
                                if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                                    statements.push(IrStatement::UseField {
                                        object: obj_name.clone(),
                                        field: field.clone(),
                                        operation: "use in function call".to_string(),
                                    });
                                    arg_names.push(format!("{}.{}", obj_name, field));
                                }
                            }
                            _ => {}
                        }
                    }

                    statements.push(IrStatement::CallExpr {
                        func: name.clone(),
                        args: arg_names,
                        result: Some(lhs_var.clone()),
                    });

                    Ok(Some(statements))
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
        Statement::FunctionCall { name, args, .. } => {
            debug_println!("DEBUG IR: Processing FunctionCall statement: {} with {} args", name, args.len());
            // Standalone function call (no assignment)
            let mut statements = Vec::new();
            let mut arg_names = Vec::new();

            // Check if this is a method call (has :: or is an operator)
            // Methods can be: qualified (Class::method), operators (operator*, operator bool), or have :: in name
            let is_method_call = name.contains("::") || name.starts_with("operator");

            // Special handling for operator= (assignment operators)
            // box1 = std::move(box2) becomes operator=(box1, Move(box2))
            // We need to treat this as: Move { from: box2, to: box1 }
            if name.contains("::operator=") || name == "operator=" {
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
                        if let crate::parser::Expression::Move(rhs_inner) = &args[1] {
                            debug_println!("DEBUG IR: operator= with Move: {} = Move(...)", lhs);
                            if let crate::parser::Expression::Variable(rhs) = rhs_inner.as_ref() {
                                debug_println!("DEBUG IR: Creating Move from '{}' to '{}' for operator=", rhs, lhs);
                                return Ok(Some(vec![IrStatement::Move {
                                    from: rhs.clone(),
                                    to: lhs.clone(),
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
                        // For method calls, the first arg is the receiver object
                        if is_method_call && i == 0 {
                            // Check if this is operator* (dereference)
                            let operation = if name.contains("::operator*") || name == "operator*" {
                                "dereference (via operator*)".to_string()
                            } else {
                                format!("call method '{}'", name)
                            };

                            statements.push(IrStatement::UseVariable {
                                var: var.clone(),
                                operation,
                            });
                        }
                        arg_names.push(var.clone());
                    }
                    crate::parser::Expression::Move(inner) => {
                        // Handle std::move in function arguments
                        match inner.as_ref() {
                            crate::parser::Expression::Variable(var) => {
                                debug_println!("DEBUG IR: Move(Variable) as direct argument: {}", var);
                                // First mark the variable as moved
                                statements.push(IrStatement::Move {
                                    from: var.clone(),
                                    to: format!("_moved_{}", var), // Temporary marker
                                });
                                arg_names.push(var.clone());
                            }
                            crate::parser::Expression::MemberAccess { object, field } => {
                                debug_println!("DEBUG IR: Move(MemberAccess) as direct argument: {}.{}",
                                    if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                                    field);
                                if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                                    statements.push(IrStatement::MoveField {
                                        object: obj_name.clone(),
                                        field: field.clone(),
                                        to: format!("_moved_{}", field),
                                    });
                                    arg_names.push(format!("{}.{}", obj_name, field));
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
                            if inner_name.contains("::operator->") || inner_name == "operator->" {
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
                            if let crate::parser::Expression::Move(move_inner) = inner_arg {
                                match move_inner.as_ref() {
                                    crate::parser::Expression::Variable(var) => {
                                        debug_println!("DEBUG IR: Found Move(Variable) in nested call: {}", var);
                                        statements.push(IrStatement::Move {
                                            from: var.clone(),
                                            to: format!("_moved_{}", var),
                                        });
                                    }
                                    crate::parser::Expression::MemberAccess { object, field } => {
                                        debug_println!("DEBUG IR: Found Move(MemberAccess) in nested call: {}.{}",
                                            if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                                            field);
                                        if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                                            statements.push(IrStatement::MoveField {
                                                object: obj_name.clone(),
                                                field: field.clone(),
                                                to: format!("_moved_{}", field),
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
                    // NEW: Handle field access as function argument
                    crate::parser::Expression::MemberAccess { object, field } => {
                        debug_println!("DEBUG IR: MemberAccess as function argument: {}.{}",
                            if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                            field);
                        // Generate UseField statement to check if field is valid
                        if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                            statements.push(IrStatement::UseField {
                                object: obj_name.clone(),
                                field: field.clone(),
                                operation: "use in function call".to_string(),
                            });
                            arg_names.push(format!("{}.{}", obj_name, field));
                        }
                    }
                    _ => {}
                }
            }

            statements.push(IrStatement::CallExpr {
                func: name.clone(),
                args: arg_names,
                result: None,
            });

            Ok(Some(statements))
        }
        Statement::Return(expr) => {
            let mut statements = Vec::new();

            let value = expr.as_ref().and_then(|e| {
                match e {
                    crate::parser::Expression::Variable(var) => {
                        Some(var.clone())
                    }
                    crate::parser::Expression::Move(inner) => {
                        // Handle return std::move(...)
                        debug_println!("DEBUG IR: Processing Move in return statement");
                        match inner.as_ref() {
                            crate::parser::Expression::Variable(var) => {
                                debug_println!("DEBUG IR: Return Move(Variable): {}", var);
                                // Generate Move statement
                                statements.push(IrStatement::Move {
                                    from: var.clone(),
                                    to: format!("_returned_{}", var),
                                });
                                Some(var.clone())
                            }
                            crate::parser::Expression::MemberAccess { object, field } => {
                                debug_println!("DEBUG IR: Return Move(MemberAccess): {}.{}",
                                    if let crate::parser::Expression::Variable(obj) = object.as_ref() { obj } else { "complex" },
                                    field);
                                if let crate::parser::Expression::Variable(obj_name) = object.as_ref() {
                                    // Generate MoveField statement
                                    statements.push(IrStatement::MoveField {
                                        object: obj_name.clone(),
                                        field: field.clone(),
                                        to: format!("_returned_{}", field),
                                    });
                                    Some(format!("{}.{}", obj_name, field))
                                } else {
                                    None
                                }
                            }
                            _ => None
                        }
                    }
                    crate::parser::Expression::FunctionCall { name: _, args } => {
                        // For return statements with implicit constructor calls (e.g., return ptr;)
                        // the variable might be in the arguments
                        if let Some(crate::parser::Expression::Variable(var)) = args.first() {
                            Some(var.clone())
                        } else {
                            None
                        }
                    }
                    _ => None
                }
            });

            statements.push(IrStatement::Return { value });
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