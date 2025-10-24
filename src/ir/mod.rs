use crate::parser::CppAst;
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
    // Variable usage (for checking moved state)
    UseVariable {
        var: String,
        operation: String, // "dereference", "method_call", etc.
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
    
    // Create entry block and convert statements
    let mut statements = Vec::new();
    
    for stmt in &func.body {
        // Convert the statement
        if let Some(ir_stmts) = convert_statement(stmt, &mut variables)? {
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
        
        variables.insert(
            param.name.clone(),
            VariableInfo {
                name: param.name.clone(),
                ty: var_type,
                ownership,
                lifetime: None,
                is_parameter: true,  // This is a parameter
            },
        );
    }
    
    Ok(IrFunction {
        name: func.name.clone(),
        cfg,
        variables,
        return_type: func.return_type.clone(),
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
) -> Result<Option<Vec<IrStatement>>, String> {
    use crate::parser::Statement;
    
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
            
            variables.insert(
                var.name.clone(),
                VariableInfo {
                    name: var.name.clone(),
                    ty: var_type,
                    ownership,
                    lifetime: None,
                    is_parameter: false,  // This is a local variable
                },
            );
            Ok(None)
        }
        Statement::ReferenceBinding { name, target, is_mutable, .. } => {
            // Convert to a borrow statement
            if let crate::parser::Expression::Variable(target_var) = target {
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
                
                Ok(Some(vec![IrStatement::Borrow {
                    from: target_var.clone(),
                    to: name.clone(),
                    kind,
                }]))
            } else {
                Ok(None)
            }
        }
        Statement::Assignment { lhs, rhs, .. } => {
            // Handle assignments that might involve references or function calls
            match rhs {
                crate::parser::Expression::Variable(rhs_var) => {
                    // Check if this is a move or a copy
                    if let Some(rhs_info) = variables.get(rhs_var) {
                        match &rhs_info.ty {
                            VariableType::UniquePtr(_) => {
                                // This is a move
                                Ok(Some(vec![IrStatement::Move {
                                    from: rhs_var.clone(),
                                    to: lhs.clone(),
                                }]))
                            }
                            _ => {
                                // Regular assignment (copy)
                                Ok(Some(vec![IrStatement::Assign {
                                    lhs: lhs.clone(),
                                    rhs: IrExpression::Variable(rhs_var.clone()),
                                }]))
                            }
                        }
                    } else {
                        Ok(None)
                    }
                }
                crate::parser::Expression::Move(inner) => {
                    debug_println!("DEBUG IR: Processing Move expression in assignment");
                    // This is an explicit std::move call
                    if let crate::parser::Expression::Variable(var) = inner.as_ref() {
                        debug_println!("DEBUG IR: Creating IrStatement::Move from '{}' to '{}'", var, lhs);
                        // Transfer type from source if needed
                        let source_type = variables.get(var).map(|info| info.ty.clone());
                        if let Some(var_info) = variables.get_mut(lhs) {
                            if let Some(ty) = source_type {
                                var_info.ty = ty;
                            }
                        }
                        Ok(Some(vec![IrStatement::Move {
                            from: var.clone(),
                            to: lhs.clone(),
                        }]))
                    } else {
                        debug_println!("DEBUG IR: Move expression doesn't contain a variable");
                        // Handle nested expressions if needed
                        Ok(None)
                    }
                }
                crate::parser::Expression::FunctionCall { name, args } => {
                    // Convert function call arguments, handling moves
                    let mut statements = Vec::new();
                    let mut arg_names = Vec::new();
                    
                    for arg in args {
                        match arg {
                            crate::parser::Expression::Variable(var) => {
                                arg_names.push(var.clone());
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
                            _ => {}
                        }
                    }
                    
                    statements.push(IrStatement::CallExpr {
                        func: name.clone(),
                        args: arg_names,
                        result: Some(lhs.clone()),
                    });
                    
                    Ok(Some(statements))
                }
                _ => Ok(None)
            }
        }
        Statement::FunctionCall { name, args, .. } => {
            // Standalone function call (no assignment)
            let mut statements = Vec::new();
            let mut arg_names = Vec::new();

            // Check if this is a method call (has :: or is an operator)
            let is_method_call = name.contains("::operator") ||
                                 name.contains("::");

            // Process arguments, looking for std::move
            for (i, arg) in args.iter().enumerate() {
                match arg {
                    crate::parser::Expression::Variable(var) => {
                        // For method calls, the first arg is the receiver object
                        if is_method_call && i == 0 {
                            // Check if the receiver has been moved
                            statements.push(IrStatement::UseVariable {
                                var: var.clone(),
                                operation: format!("call method '{}'", name),
                            });
                        }
                        arg_names.push(var.clone());
                    }
                    crate::parser::Expression::Move(inner) => {
                        // Handle std::move in function arguments
                        if let crate::parser::Expression::Variable(var) = inner.as_ref() {
                            // First mark the variable as moved
                            statements.push(IrStatement::Move {
                                from: var.clone(),
                                to: format!("_moved_{}", var), // Temporary marker
                            });
                            arg_names.push(var.clone());
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
            let value = expr.as_ref().and_then(|e| {
                if let crate::parser::Expression::Variable(var) = e {
                    Some(var.clone())
                } else {
                    None
                }
            });
            
            Ok(Some(vec![IrStatement::Return { value }]))
        }
        Statement::EnterScope => {
            Ok(Some(vec![IrStatement::EnterScope]))
        }
        Statement::ExitScope => {
            Ok(Some(vec![IrStatement::ExitScope]))
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
        Statement::If { then_branch, else_branch, .. } => {
            // Convert if statement branches to IR
            let mut then_ir = Vec::new();
            for stmt in then_branch {
                if let Some(ir_stmts) = convert_statement(stmt, variables)? {
                    then_ir.extend(ir_stmts);
                }
            }
            
            let else_ir = if let Some(else_stmts) = else_branch {
                let mut else_ir = Vec::new();
                for stmt in else_stmts {
                    if let Some(ir_stmts) = convert_statement(stmt, variables)? {
                        else_ir.extend(ir_stmts);
                    }
                }
                Some(else_ir)
            } else {
                None
            };
            
            Ok(Some(vec![IrStatement::If {
                then_branch: then_ir,
                else_branch: else_ir,
            }]))
        }
        Statement::ExpressionStatement { expr, .. } => {
            // Handle expression statements (dereference, method calls, etc.)
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
                _ => Ok(None),
            }
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
            location: SourceLocation {
                file: "test.cpp".to_string(),
                line: 1,
                column: 1,
            },
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