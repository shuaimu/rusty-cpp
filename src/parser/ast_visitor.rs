use clang::{Entity, EntityKind, Type, TypeKind};
use crate::debug_println;

/// Get the qualified name of an entity (including namespace/class context)
pub fn get_qualified_name(entity: &Entity) -> String {
    let simple_name = entity.get_name().unwrap_or_else(|| "anonymous".to_string());
    
    // Try to build qualified name by walking up the semantic parents
    let mut parts = vec![simple_name.clone()];
    let mut current = entity.get_semantic_parent();
    
    while let Some(parent) = current {
        match parent.get_kind() {
            EntityKind::Namespace | EntityKind::ClassDecl | EntityKind::StructDecl | EntityKind::ClassTemplate => {
                if let Some(parent_name) = parent.get_name() {
                    if !parent_name.is_empty() {
                        parts.push(parent_name);
                    }
                }
            }
            _ => {}
        }
        current = parent.get_semantic_parent();
    }
    
    // Reverse to get the correct order (namespace::class::method)
    parts.reverse();
    
    // Join with :: but skip if we only have the simple name
    if parts.len() > 1 {
        parts.join("::")
    } else {
        simple_name
    }
}

#[derive(Debug, Clone)]
pub struct CppAst {
    pub functions: Vec<Function>,
    pub global_variables: Vec<Variable>,
}

impl CppAst {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            global_variables: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Variable>,
    #[allow(dead_code)]
    pub return_type: String,
    #[allow(dead_code)]
    pub body: Vec<Statement>,
    #[allow(dead_code)]
    pub location: SourceLocation,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub type_name: String,
    pub is_reference: bool,
    #[allow(dead_code)]
    pub is_pointer: bool,
    pub is_const: bool,
    pub is_unique_ptr: bool,
    #[allow(dead_code)]
    pub is_shared_ptr: bool,
    #[allow(dead_code)]
    pub location: SourceLocation,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Statement {
    VariableDecl(Variable),
    Assignment {
        lhs: Expression,  // Changed to Expression to support dereference: *ptr = value
        rhs: Expression,
        location: SourceLocation,
    },
    ReferenceBinding {
        name: String,
        target: Expression,
        is_mutable: bool,
        location: SourceLocation,
    },
    Return(Option<Expression>),
    FunctionCall {
        name: String,
        args: Vec<Expression>,
        location: SourceLocation,
    },
    Block(Vec<Statement>),
    // Scope markers
    EnterScope,
    ExitScope,
    // Loop markers
    EnterLoop,
    ExitLoop,
    // Safety markers
    EnterUnsafe,
    ExitUnsafe,
    // Conditional statements
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
        location: SourceLocation,
    },
    // Expression statements (e.g., standalone dereference, method calls)
    ExpressionStatement {
        expr: Expression,
        location: SourceLocation,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Expression {
    Variable(String),
    Move(Box<Expression>),
    Dereference(Box<Expression>),
    AddressOf(Box<Expression>),
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    Literal(String),
    BinaryOp {
        left: Box<Expression>,
        op: String,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    #[allow(dead_code)]
    pub file: String,
    #[allow(dead_code)]
    pub line: u32,
    #[allow(dead_code)]
    pub column: u32,
}

pub fn extract_function(entity: &Entity) -> Function {
    // Use qualified name for methods to avoid collisions
    let name = if entity.get_kind() == EntityKind::Method || entity.get_kind() == EntityKind::Constructor {
        // For methods, try to get the qualified name
        get_qualified_name(entity)
    } else {
        entity.get_name().unwrap_or_else(|| "anonymous".to_string())
    };
    let location = extract_location(entity);
    
    let mut parameters = Vec::new();
    for child in entity.get_children() {
        if child.get_kind() == EntityKind::ParmDecl {
            parameters.push(extract_variable(&child));
        }
    }
    
    let return_type = entity
        .get_result_type()
        .map(|t| type_to_string(&t))
        .unwrap_or_else(|| "void".to_string());
    
    let body = extract_function_body(entity);
    
    Function {
        name,
        parameters,
        return_type,
        body,
        location,
    }
}

pub fn extract_variable(entity: &Entity) -> Variable {
    let name = entity.get_name().unwrap_or_else(|| "anonymous".to_string());
    let location = extract_location(entity);
    
    let type_info = entity.get_type().unwrap();
    let type_name = type_to_string(&type_info);
    
    let is_reference = matches!(type_info.get_kind(), TypeKind::LValueReference | TypeKind::RValueReference);
    let is_pointer = matches!(type_info.get_kind(), TypeKind::Pointer);
    
    // For references, check if the pointee type is const
    let is_const = if is_reference {
        if let Some(pointee) = type_info.get_pointee_type() {
            pointee.is_const_qualified()
        } else {
            type_info.is_const_qualified()
        }
    } else {
        type_info.is_const_qualified()
    };
    
    let is_unique_ptr = type_name.contains("unique_ptr");
    let is_shared_ptr = type_name.contains("shared_ptr");
    
    Variable {
        name,
        type_name,
        is_reference,
        is_pointer,
        is_const,
        is_unique_ptr,
        is_shared_ptr,
        location,
    }
}

fn extract_function_body(entity: &Entity) -> Vec<Statement> {
    let mut statements = Vec::new();
    
    for child in entity.get_children() {
        if child.get_kind() == EntityKind::CompoundStmt {
            statements.extend(extract_compound_statement(&child));
        }
    }
    
    statements
}

fn extract_compound_statement(entity: &Entity) -> Vec<Statement> {
    let mut statements = Vec::new();

    for child in entity.get_children() {
        debug_println!("DEBUG STMT: Compound child kind: {:?}", child.get_kind());
        match child.get_kind() {
            EntityKind::DeclStmt => {
                for decl_child in child.get_children() {
                    if decl_child.get_kind() == EntityKind::VarDecl {
                        let var = extract_variable(&decl_child);
                        
                        // Always add the variable declaration first
                        statements.push(Statement::VariableDecl(var.clone()));
                        
                        // Check if this variable has an initializer
                        for init_child in decl_child.get_children() {
                            if let Some(expr) = extract_expression(&init_child) {
                                
                                // Check if this is a reference binding
                                if var.is_reference {
                                    statements.push(Statement::ReferenceBinding {
                                        name: var.name.clone(),
                                        target: expr,
                                        is_mutable: !var.is_const,
                                        location: extract_location(&decl_child),
                                    });
                                } else {
                                    // Regular assignment/initialization
                                    statements.push(Statement::Assignment {
                                        lhs: Expression::Variable(var.name.clone()),
                                        rhs: expr,
                                        location: extract_location(&decl_child),
                                    });
                                }
                                break;
                            }
                        }
                    }
                }
            }
            EntityKind::BinaryOperator => {
                // Handle assignments
                let children: Vec<Entity> = child.get_children().into_iter().collect();
                debug_println!("DEBUG STMT: BinaryOperator has {} children", children.len());
                if children.len() == 2 {
                    debug_println!("DEBUG STMT: BinaryOperator child[0] kind: {:?}", children[0].get_kind());
                    debug_println!("DEBUG STMT: BinaryOperator child[1] kind: {:?}", children[1].get_kind());
                    let lhs_expr = extract_expression(&children[0]);
                    let rhs_expr = extract_expression(&children[1]);
                    debug_println!("DEBUG STMT: BinaryOperator LHS: {:?}", lhs_expr);
                    debug_println!("DEBUG STMT: BinaryOperator RHS: {:?}", rhs_expr);
                    if let (Some(lhs), Some(rhs)) = (lhs_expr, rhs_expr) {
                        debug_println!("DEBUG STMT: Creating Assignment statement");
                        statements.push(Statement::Assignment {
                            lhs,  // Now supports dereference: *ptr = value
                            rhs,
                            location: extract_location(&child),
                        });
                    } else {
                        debug_println!("DEBUG STMT: Failed to extract expressions from BinaryOperator");
                    }
                }
            }
            EntityKind::CallExpr => {
                let children: Vec<Entity> = child.get_children().into_iter().collect();
                let mut name = "unknown".to_string();
                let mut args = Vec::new();
                
                // Check if this might be a variable declaration disguised as a CallExpr
                // This happens with constructs like "struct timeval now;" or "ClassName obj;"
                let mut is_likely_var_decl = false;
                
                // Debug: Log all CallExprs
                debug_println!("DEBUG AST: Found CallExpr with {} children", children.len());
                
                // First check if the CallExpr itself has a reference
                if let Some(ref_entity) = child.get_reference() {
                    debug_println!("DEBUG AST: CallExpr references entity kind: {:?}, name: {:?}", 
                        ref_entity.get_kind(), ref_entity.get_name());
                    
                    // Check if it references a type (struct/class/typedef)
                    if ref_entity.get_kind() == EntityKind::StructDecl || 
                       ref_entity.get_kind() == EntityKind::ClassDecl ||
                       ref_entity.get_kind() == EntityKind::TypedefDecl ||
                       ref_entity.get_kind() == EntityKind::TypeAliasDecl {
                        // This is likely a variable declaration, not a function call
                        debug_println!("DEBUG AST: CallExpr appears to be a variable declaration of type {:?}", 
                            ref_entity.get_name());
                        is_likely_var_decl = true;
                    }
                    
                    if let Some(n) = ref_entity.get_name() {
                        // Build qualified name for member functions
                        if ref_entity.get_kind() == EntityKind::Method {
                            name = get_qualified_name(&ref_entity);
                        } else {
                            name = n;
                        }
                    }
                }
                
                // If this looks like a variable declaration, skip it
                if is_likely_var_decl && children.is_empty() {
                    debug_println!("DEBUG AST: Skipping variable declaration disguised as CallExpr: {}", name);
                    continue;
                }
                
                // Try to extract the function name from children
                for c in &children {
                    if c.get_kind() == EntityKind::UnexposedExpr || c.get_kind() == EntityKind::DeclRefExpr {
                        if let Some(n) = c.get_name() {
                            if name == "unknown" {
                                debug_println!("DEBUG AST: Got name from child: {}", n);
                                name = n;
                            }
                        }
                    }
                }
                
                // Check if this is a type name being used as a constructor/declaration
                // Common pattern: TypeName varname; is parsed as CallExpr
                if children.len() == 1 && name != "unknown" {
                    // Check if the name matches known type patterns
                    if name.ends_with("val") || name.ends_with("spec") || 
                       name.starts_with("struct") || name.starts_with("class") {
                        debug_println!("DEBUG AST: Likely variable declaration based on name pattern: {}", name);
                        is_likely_var_decl = true;
                    }
                }
                
                // If this looks like a variable declaration, skip it
                if is_likely_var_decl {
                    debug_println!("DEBUG AST: Skipping variable declaration disguised as CallExpr: {}", name);
                    continue;
                }
                
                // Two-pass approach: identify name-providing child, then extract args
                let mut name_providing_child_idx: Option<usize> = None;

                // Pass 1: If name still unknown, find it; otherwise identify which child has it
                if name == "unknown" {
                    for (i, c) in children.iter().enumerate() {
                        debug_println!("DEBUG AST: Child {}: kind={:?}, name={:?}", i, c.get_kind(), c.get_name());
                        match c.get_kind() {
                            EntityKind::MemberRefExpr => {
                                debug_println!("DEBUG AST: Found MemberRefExpr!");
                                if let Some(ref_entity) = c.get_reference() {
                                    debug_println!("DEBUG AST: MemberRefExpr has reference: kind={:?}, name={:?}",
                                        ref_entity.get_kind(), ref_entity.get_name());
                                    if let Some(n) = ref_entity.get_name() {
                                        if ref_entity.get_kind() == EntityKind::Method {
                                            name = get_qualified_name(&ref_entity);
                                        } else {
                                            name = n;
                                        }
                                        name_providing_child_idx = Some(i);
                                        break;
                                    }
                                } else {
                                    debug_println!("DEBUG AST: MemberRefExpr has NO reference!");
                                }
                            }
                            EntityKind::DeclRefExpr | EntityKind::UnexposedExpr => {
                                if let Some(n) = c.get_name() {
                                    name = n;
                                    name_providing_child_idx = Some(i);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Name already known, find which child provides it
                    debug_println!("DEBUG AST: Name already known: '{}', searching {} children", name, children.len());
                    for (i, c) in children.iter().enumerate() {
                        debug_println!("DEBUG AST: Child {}: kind={:?}, name={:?}", i, c.get_kind(), c.get_name());
                        match c.get_kind() {
                            EntityKind::MemberRefExpr => {
                                // For .method() calls, MemberRefExpr contains the method name
                                debug_println!("DEBUG AST: Exploring MemberRefExpr for receiver object:");
                                debug_println!("  - name: {:?}", c.get_name());
                                debug_println!("  - display_name: {:?}", c.get_display_name());
                                debug_println!("  - num children: {}", c.get_children().len());

                                // Check if MemberRefExpr has children
                                for (child_idx, member_child) in c.get_children().iter().enumerate() {
                                    debug_println!("    - MemberRefExpr child {}: kind={:?}, name={:?}",
                                        child_idx, member_child.get_kind(), member_child.get_name());
                                }

                                // Check semantic parent
                                if let Some(semantic_parent) = c.get_semantic_parent() {
                                    debug_println!("  - semantic_parent: kind={:?}, name={:?}",
                                        semantic_parent.get_kind(), semantic_parent.get_name());
                                }

                                // Check lexical parent
                                if let Some(lexical_parent) = c.get_lexical_parent() {
                                    debug_println!("  - lexical_parent: kind={:?}, name={:?}",
                                        lexical_parent.get_kind(), lexical_parent.get_name());
                                }

                                if let Some(child_name) = c.get_name() {
                                    debug_println!("DEBUG AST: Checking if '{}' matches name '{}'", child_name, name);
                                    if name.ends_with(&child_name) || name == child_name {
                                        debug_println!("DEBUG AST: Match found! Setting name_providing_child_idx = {}", i);
                                        name_providing_child_idx = Some(i);
                                        break;
                                    }
                                }
                            }
                            EntityKind::DeclRefExpr | EntityKind::UnexposedExpr => {
                                if let Some(child_name) = c.get_name() {
                                    debug_println!("DEBUG AST: Checking if '{}' matches name '{}'", child_name, name);
                                    if name.ends_with(&child_name) || name == child_name {
                                        debug_println!("DEBUG AST: Match found! Setting name_providing_child_idx = {}", i);
                                        name_providing_child_idx = Some(i);
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    debug_println!("DEBUG AST: name_providing_child_idx = {:?}", name_providing_child_idx);
                }

                // Pass 2: Extract arguments, skipping name-providing child
                for (i, c) in children.into_iter().enumerate() {
                    if Some(i) == name_providing_child_idx {
                        // For MemberRefExpr, extract the receiver from its children
                        if c.get_kind() == EntityKind::MemberRefExpr {
                            debug_println!("DEBUG AST: Extracting receiver from MemberRefExpr children");
                            for member_child in c.get_children() {
                                if let Some(receiver_expr) = extract_expression(&member_child) {
                                    debug_println!("DEBUG AST: Found receiver: {:?}", receiver_expr);
                                    args.push(receiver_expr);
                                    break; // Only take the first child (the receiver)
                                }
                            }
                        }
                        continue;  // Skip the MemberRefExpr itself
                    }

                    // Extract as argument
                    if let Some(expr) = extract_expression(&c) {
                        args.push(expr);
                    }
                }
                
                debug_println!("DEBUG STMT: Creating FunctionCall statement: name='{}', args={:?}", name, args);
                statements.push(Statement::FunctionCall {
                    name,
                    args,
                    location: extract_location(&child),
                });
            }
            EntityKind::ReturnStmt => {
                // Extract the return value expression
                let return_expr = child.get_children()
                    .into_iter()
                    .find_map(|c| extract_expression(&c));
                statements.push(Statement::Return(return_expr));
            }
            EntityKind::CompoundStmt => {
                // Regular nested block scope - add scope markers
                statements.push(Statement::EnterScope);
                statements.extend(extract_compound_statement(&child));
                statements.push(Statement::ExitScope);
            }
            EntityKind::ForStmt | EntityKind::WhileStmt | EntityKind::DoStmt => {
                // Loop detected - add loop markers
                statements.push(Statement::EnterLoop);
                // Extract loop body (usually a compound statement)
                for loop_child in child.get_children() {
                    if loop_child.get_kind() == EntityKind::CompoundStmt {
                        statements.extend(extract_compound_statement(&loop_child));
                    }
                }
                statements.push(Statement::ExitLoop);
            }
            EntityKind::IfStmt => {
                // Extract if statement
                let children: Vec<Entity> = child.get_children().into_iter().collect();
                let mut condition = Expression::Literal("true".to_string());
                let mut then_branch = Vec::new();
                let mut else_branch = None;
                
                // Parse the if statement structure
                let mut i = 0;
                while i < children.len() {
                    let child_kind = children[i].get_kind();
                    
                    if child_kind == EntityKind::UnexposedExpr || child_kind == EntityKind::BinaryOperator {
                        // This is likely the condition
                        if let Some(expr) = extract_expression(&children[i]) {
                            condition = expr;
                        }
                    } else if child_kind == EntityKind::CompoundStmt {
                        // This is a branch
                        if then_branch.is_empty() {
                            then_branch = extract_compound_statement(&children[i]);
                        } else {
                            else_branch = Some(extract_compound_statement(&children[i]));
                        }
                    }
                    i += 1;
                }
                
                statements.push(Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                    location: extract_location(&child),
                });
            }
            EntityKind::UnaryOperator => {
                // Handle standalone dereference operations
                if let Some(expr) = extract_expression(&child) {
                    // Only add as statement if it's a dereference or address-of
                    match &expr {
                        Expression::Dereference(_) | Expression::AddressOf(_) => {
                            statements.push(Statement::ExpressionStatement {
                                expr,
                                location: extract_location(&child),
                            });
                        }
                        _ => {} // Ignore other unary operators for now
                    }
                }
            }
            EntityKind::UnexposedExpr => {
                // UnexposedExpr can contain function calls or other expressions
                // Try to extract it as an expression and create appropriate statement
                if let Some(expr) = extract_expression(&child) {
                    match expr {
                        Expression::FunctionCall { name, args } => {
                            statements.push(Statement::FunctionCall {
                                name,
                                args,
                                location: extract_location(&child),
                            });
                        }
                        _ => {
                            // Other expression types - add as expression statement
                            statements.push(Statement::ExpressionStatement {
                                expr,
                                location: extract_location(&child),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    statements
}

fn extract_expression(entity: &Entity) -> Option<Expression> {
    match entity.get_kind() {
        EntityKind::DeclRefExpr => {
            entity.get_name().map(Expression::Variable)
        }
        EntityKind::CallExpr => {
            // Extract function call as expression
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            let mut name = "unknown".to_string();
            let mut args = Vec::new();
            
            // Check if this might be a variable declaration disguised as a CallExpr
            // This happens with constructs like "struct timeval now;" or "ClassName obj;"
            // The pattern is: CallExpr with 0 children that references a type name
            
            // First check if the CallExpr itself has a reference
            if let Some(ref_entity) = entity.get_reference() {
                debug_println!("DEBUG AST: CallExpr itself references: {:?}", ref_entity.get_name());
                
                if let Some(n) = ref_entity.get_name() {
                    name = n;
                }
                
                // Check if it references a type (struct/class/typedef)
                // BUT: A CallExpr with 0 children is likely a constructor/declaration
                if children.is_empty() {
                    debug_println!("DEBUG AST: CallExpr with 0 children referencing '{}' - likely a variable declaration", name);
                    return None;  // Not a function call, it's a variable declaration
                }
                
                // Build qualified name for member functions and constructors
                if ref_entity.get_kind() == EntityKind::Method || ref_entity.get_kind() == EntityKind::Constructor {
                    name = get_qualified_name(&ref_entity);
                }
            }
            
            // Debug: print all child entity kinds
            for c in &children {
                debug_println!("DEBUG AST: CallExpr child kind: {:?}, name: {:?}, display_name: {:?}", 
                    c.get_kind(), c.get_name(), c.get_display_name());
                    
                // For member function calls, check for MemberRefExpr first
                if c.get_kind() == EntityKind::MemberRefExpr {
                    // This is definitely a member function call
                    if let Some(ref_entity) = c.get_reference() {
                        debug_println!("DEBUG AST: MemberRefExpr references: {:?}", ref_entity.get_name());
                        if let Some(n) = ref_entity.get_name() {
                            if name == "unknown" {
                                // Build qualified name for member functions and constructors
                                if ref_entity.get_kind() == EntityKind::Method || ref_entity.get_kind() == EntityKind::Constructor {
                                    name = get_qualified_name(&ref_entity);
                                } else {
                                    name = n;
                                }
                            }
                        }
                    }

                    // Debug: Check if MemberRefExpr has children (which might be the receiver object)
                    let member_children = c.get_children();
                    debug_println!("DEBUG AST: MemberRefExpr has {} children", member_children.len());
                    for (i, mc) in member_children.iter().enumerate() {
                        debug_println!("  DEBUG AST: MemberRefExpr child[{}]: kind={:?}, name={:?}",
                            i, mc.get_kind(), mc.get_name());
                    }
                } else if c.get_kind() == EntityKind::UnexposedExpr {
                    // Try to get the referenced entity
                    if let Some(ref_entity) = c.get_reference() {
                        debug_println!("DEBUG AST: UnexposedExpr references: {:?}", ref_entity.get_name());
                        if let Some(n) = ref_entity.get_name() {
                            if name == "unknown" {
                                name = n;
                            }
                        }
                    }
                    // Also try children of UnexposedExpr
                    for ue_child in c.get_children() {
                        debug_println!("DEBUG AST: UnexposedExpr child: kind={:?}, name={:?}, display_name={:?}", 
                            ue_child.get_kind(), ue_child.get_name(), ue_child.get_display_name());
                        
                        // Try to extract member function name from MemberRefExpr
                        if let Some(n) = ue_child.get_name() {
                            if name == "unknown" {
                                name = n;
                            }
                        } else if let Some(dn) = ue_child.get_display_name() {
                            if name == "unknown" && !dn.is_empty() {
                                name = dn;
                            }
                        }
                        
                        // Also check if this child has a reference
                        if let Some(ref_entity) = ue_child.get_reference() {
                            debug_println!("DEBUG AST: Child references: {:?}", ref_entity.get_name());
                            if let Some(n) = ref_entity.get_name() {
                                if name == "unknown" {
                                    name = n;
                                }
                            }
                        }
                    }
                }
            }
            
            // Three-pass approach:
            // Pass 1: Extract receiver from MemberRefExpr (for method calls)
            // Pass 2: Identify which child provides the function name
            // Pass 3: Extract remaining arguments, skipping name-providing child

            let mut receiver: Option<Expression> = None;
            let mut name_providing_child_idx: Option<usize> = None;

            // Pass 1: Look for MemberRefExpr and extract receiver
            for (i, c) in children.iter().enumerate() {
                if c.get_kind() == EntityKind::MemberRefExpr {
                    // Extract receiver from MemberRefExpr's children
                    let member_children = c.get_children();
                    if !member_children.is_empty() {
                        // First child of MemberRefExpr is the receiver object
                        if let Some(recv_expr) = extract_expression(&member_children[0]) {
                            debug_println!("DEBUG AST: Extracted receiver from MemberRefExpr: {:?}", recv_expr);
                            receiver = Some(recv_expr);
                        }
                    }
                    // Mark this as the name-providing child to skip later
                    name_providing_child_idx = Some(i);
                    break;
                }
            }

            // Pass 2: If no MemberRefExpr, use existing logic to find name provider
            if name_providing_child_idx.is_none() {
                if name == "unknown" {
                    for (i, c) in children.iter().enumerate() {
                        match c.get_kind() {
                            EntityKind::DeclRefExpr | EntityKind::UnexposedExpr => {
                                if let Some(n) = c.get_name() {
                                    name = n;
                                    name_providing_child_idx = Some(i);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Name was already extracted, identify which child represents it
                    for (i, c) in children.iter().enumerate() {
                        match c.get_kind() {
                            EntityKind::DeclRefExpr | EntityKind::UnexposedExpr => {
                                if let Some(child_name) = c.get_name() {
                                    if name.ends_with(&child_name) || name == child_name {
                                        name_providing_child_idx = Some(i);
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Add receiver as first argument if present
            if let Some(recv) = receiver {
                args.push(recv);
            }

            // Pass 3: Extract remaining arguments, skipping name-providing child
            for (i, c) in children.into_iter().enumerate() {
                // Skip the child that provided the function name
                if Some(i) == name_providing_child_idx {
                    continue;
                }

                // Extract all other children as arguments
                if let Some(expr) = extract_expression(&c) {
                    args.push(expr);
                }
            }
            
            // Check if this is std::move
            debug_println!("DEBUG: Found function call: name='{}', args_count={}", name, args.len());
            for (i, arg) in args.iter().enumerate() {
                debug_println!("  DEBUG: arg[{}] = {:?}", i, arg);
            }
            if name == "move" || name == "std::move" || name.ends_with("::move") || name.contains("move") {
                debug_println!("DEBUG: Detected move function!");
                // std::move takes one argument and we treat it as a Move expression
                if args.len() == 1 {
                    debug_println!("DEBUG: Creating Move expression");
                    return Some(Expression::Move(Box::new(args.into_iter().next().unwrap())));
                }
            }
            
            Some(Expression::FunctionCall { name, args })
        }
        EntityKind::UnexposedExpr => {
            // UnexposedExpr often wraps other expressions, so look at its children
            debug_println!("DEBUG EXTRACT: UnexposedExpr with name={:?}, {} children",
                entity.get_name(), entity.get_children().len());

            // Check if this UnexposedExpr has a reference (might be a method call)
            if let Some(ref_entity) = entity.get_reference() {
                debug_println!("  DEBUG EXTRACT: UnexposedExpr has reference: kind={:?}, name={:?}",
                    ref_entity.get_kind(), ref_entity.get_name());
            }

            for child in entity.get_children() {
                debug_println!("  DEBUG EXTRACT: Child kind={:?}, name={:?}",
                    child.get_kind(), child.get_name());

                // Check if child has a reference
                if let Some(child_ref) = child.get_reference() {
                    debug_println!("    DEBUG EXTRACT: Child has reference: kind={:?}, name={:?}",
                        child_ref.get_kind(), child_ref.get_name());
                }

                if let Some(expr) = extract_expression(&child) {
                    debug_println!("  DEBUG EXTRACT: Returning {:?}", expr);
                    return Some(expr);
                }
            }
            debug_println!("  DEBUG EXTRACT: Returning None");
            None
        }
        EntityKind::BinaryOperator => {
            // Extract binary operation (e.g., i < 2, x == 0)
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            if children.len() == 2 {
                if let (Some(left), Some(right)) = 
                    (extract_expression(&children[0]), extract_expression(&children[1])) {
                    // Try to get the operator from the entity's spelling
                    let op = entity.get_name().unwrap_or_else(|| "==".to_string());
                    return Some(Expression::BinaryOp {
                        left: Box::new(left),
                        op,
                        right: Box::new(right),
                    });
                }
            }
            None
        }
        EntityKind::IntegerLiteral => {
            // IntegerLiterals often have name=None, try display_name or tokens
            if let Some(name) = entity.get_name() {
                Some(Expression::Literal(name))
            } else if let Some(display) = entity.get_display_name() {
                Some(Expression::Literal(display))
            } else {
                // For integer literals, we can use a placeholder since we don't
                // need the actual value for ownership/borrow checking
                Some(Expression::Literal("0".to_string()))
            }
        }
        EntityKind::UnaryOperator => {
            // Check if it's address-of (&) or dereference (*)
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            if !children.is_empty() {
                if let Some(inner) = extract_expression(&children[0]) {
                    // Try to determine the operator type
                    // LibClang doesn't give us the operator directly, but we can check the types
                    if let Some(result_type) = entity.get_type() {
                        let type_str = type_to_string(&result_type);
                        
                        if let Some(child_type) = children[0].get_type() {
                            let child_type_str = type_to_string(&child_type);
                            
                            // If child is pointer and result is not, it's dereference
                            if child_type_str.contains('*') && !type_str.contains('*') {
                                return Some(Expression::Dereference(Box::new(inner)));
                            }
                            // If child is not pointer but result is, it's address-of
                            else if !child_type_str.contains('*') && type_str.contains('*') {
                                return Some(Expression::AddressOf(Box::new(inner)));
                            }
                        }
                    }
                    // Default to address-of if we can't determine
                    return Some(Expression::AddressOf(Box::new(inner)));
                }
            }
            None
        }
        _ => None
    }
}

fn extract_location(entity: &Entity) -> SourceLocation {
    let location = entity.get_location().unwrap();
    let file_location = location.get_file_location();
    
    SourceLocation {
        file: file_location
            .file
            .map(|f| f.get_path().display().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        line: file_location.line,
        column: file_location.column,
    }
}

fn type_to_string(ty: &Type) -> String {
    ty.get_display_name()
}

#[allow(dead_code)]
fn check_for_unsafe_annotation(_entity: &Entity) -> bool {
    // This function is no longer used since we handle unsafe regions
    // differently using comment annotations that are scanned separately
    // Always return false
    false
}