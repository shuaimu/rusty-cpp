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

/// Extract template type parameters from a template entity
///
/// For `template<typename T, typename U>`, this returns ["T", "U"]
/// Works with ClassTemplateDecl and FunctionTemplateDecl
pub fn extract_template_parameters(entity: &Entity) -> Vec<String> {
    use clang::EntityVisitResult;

    let mut params = Vec::new();

    // Visit direct children to find TemplateTypeParameter or NonTypeTemplateParameter
    entity.visit_children(|child, _| {
        match child.get_kind() {
            EntityKind::TemplateTypeParameter | EntityKind::NonTypeTemplateParameter => {
                if let Some(name) = child.get_name() {
                    debug_println!("TEMPLATE: Found type parameter: {}", name);
                    params.push(name);
                }
            }
            _ => {}
        }
        EntityVisitResult::Continue
    });

    params
}

// Phase 3: Class template representation
#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub template_parameters: Vec<String>,  // e.g., ["T", "Args"] for template<typename T, typename... Args>
    pub is_template: bool,
    pub members: Vec<Variable>,            // Member fields
    pub methods: Vec<Function>,            // Member methods
    pub base_classes: Vec<String>,         // Base class names (may contain packs like "Bases...")
    pub location: SourceLocation,
}

#[derive(Debug, Clone)]
pub struct CppAst {
    pub functions: Vec<Function>,
    pub global_variables: Vec<Variable>,
    pub classes: Vec<Class>,  // Phase 3: Track template classes
}

impl CppAst {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            global_variables: Vec::new(),
            classes: Vec::new(),  // Phase 3
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MethodQualifier {
    Const,        // const method (like Rust's &self)
    NonConst,     // regular method (like Rust's &mut self)
    RvalueRef,    // && qualified method (like Rust's self)
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
    // Method information
    pub is_method: bool,
    pub method_qualifier: Option<MethodQualifier>,
    pub class_name: Option<String>,
    // Template information
    pub template_parameters: Vec<String>,  // e.g., ["T", "U"] for template<typename T, typename U>
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
    pub is_static: bool,
    #[allow(dead_code)]
    pub location: SourceLocation,
    // Variadic template support (Phase 1)
    pub is_pack: bool,                         // Is this a parameter pack (e.g., Args... args)?
    pub pack_element_type: Option<String>,     // Type of pack elements (e.g., "Args&&" from "Args&&...")
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
    // Phase 2: Pack expansion statement (fold expressions, pack usage)
    PackExpansion {
        pack_name: String,           // Name of the pack being expanded (e.g., "args")
        operation: String,           // Type of operation: "forward", "move", "use"
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
    // NEW: Member access (obj.field)
    MemberAccess {
        object: Box<Expression>,
        field: String,
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
    let kind = entity.get_kind();
    let is_method = kind == EntityKind::Method || kind == EntityKind::Constructor;

    // Use qualified name for methods to avoid collisions
    let name = if is_method {
        // For methods, try to get the qualified name
        get_qualified_name(entity)
    } else {
        entity.get_name().unwrap_or_else(|| "anonymous".to_string())
    };
    let location = extract_location(entity);

    let mut parameters = Vec::new();
    for child in entity.get_children() {
        if child.get_kind() == EntityKind::ParmDecl {
            let mut param = extract_variable(&child);

            // Phase 1: Detect variadic parameter packs
            // Check if this is a parameter pack by examining the type
            if let Some(param_type) = child.get_type() {
                let type_str = type_to_string(&param_type);

                // Check if type contains "..." (e.g., "Args...", "Args &&...", "const T &...")
                let is_pack = type_str.contains("...") || child.is_variadic();

                if is_pack {
                    param.is_pack = true;

                    // Extract element type by removing "..." from the type string
                    let element_type = type_str.trim_end_matches("...").trim().to_string();
                    param.pack_element_type = Some(element_type.clone());

                    debug_println!("DEBUG PARSE: Found parameter pack '{}' with element type '{}'",
                        param.name, element_type);
                }
            }

            parameters.push(param);
        }
    }

    let return_type = entity
        .get_result_type()
        .map(|t| type_to_string(&t))
        .unwrap_or_else(|| "void".to_string());

    let body = extract_function_body(entity);

    // Detect method qualifier and class name
    let (method_qualifier, class_name) = if is_method {
        let qualifier = detect_method_qualifier(entity);
        let class_name = entity.get_semantic_parent()
            .and_then(|parent| parent.get_name());
        (Some(qualifier), class_name)
    } else {
        (None, None)
    };

    // Extract template parameters from parent if this is a template class method
    let template_parameters = if is_method {
        // Check if parent is a ClassTemplate
        if let Some(parent) = entity.get_semantic_parent() {
            if parent.get_kind() == EntityKind::ClassTemplate {
                debug_println!("TEMPLATE: Method in template class, extracting parameters");
                extract_template_parameters(&parent)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        // For free functions, template params would come from FunctionTemplate parent
        // We'll handle this in Phase 2
        Vec::new()
    };

    Function {
        name,
        parameters,
        return_type,
        body,
        location,
        is_method,
        method_qualifier,
        class_name,
        template_parameters,
    }
}

// Phase 3: Extract class template information
pub fn extract_class(entity: &Entity) -> Class {
    use crate::debug_println;

    let name = entity.get_name().unwrap_or_else(|| "anonymous".to_string());
    let location = extract_location(entity);
    let is_template = entity.get_kind() == EntityKind::ClassTemplate;

    debug_println!("DEBUG PARSE: Extracting class '{}', is_template={}", name, is_template);

    // Extract template parameters from ClassTemplate
    let template_parameters = if is_template {
        extract_template_parameters(entity)
    } else {
        Vec::new()
    };

    debug_println!("DEBUG PARSE: Class '{}' has {} template parameters: {:?}",
        name, template_parameters.len(), template_parameters);

    let mut members = Vec::new();
    let mut methods = Vec::new();
    let mut base_classes = Vec::new();

    // LibClang's get_children() flattens the hierarchy and returns class members directly
    // (FieldDecl, Method, etc.) rather than going through CXXRecordDecl
    for child in entity.get_children() {
        debug_println!("DEBUG PARSE: ClassTemplate child kind: {:?}", child.get_kind());
        match child.get_kind() {
            EntityKind::FieldDecl => {
                // Member field
                let mut member = extract_variable(&child);

                // Phase 3: Check if member type contains pack expansion
                if let Some(field_type) = child.get_type() {
                    let type_str = type_to_string(&field_type);
                    if type_str.contains("...") {
                        debug_println!("DEBUG PARSE: Found member field with pack expansion: '{}' of type '{}'",
                            member.name, type_str);
                        member.is_pack = true;
                        member.pack_element_type = Some(type_str.clone());
                    }
                }

                members.push(member);
            }
            EntityKind::Method | EntityKind::Constructor | EntityKind::Destructor => {
                // Member method
                let method = extract_function(&child);
                methods.push(method);
            }
            EntityKind::FunctionTemplate => {
                // Template method
                if child.is_definition() {
                    let method = extract_function(&child);
                    methods.push(method);
                }
            }
            EntityKind::BaseSpecifier => {
                // Base class
                if let Some(base_type) = child.get_type() {
                    let base_name = type_to_string(&base_type);
                    debug_println!("DEBUG PARSE: Found base class: '{}'", base_name);
                    base_classes.push(base_name);
                }
            }
            _ => {}
        }
    }

    debug_println!("DEBUG PARSE: Class '{}' has {} members, {} methods, {} base classes",
        name, members.len(), methods.len(), base_classes.len());

    Class {
        name,
        template_parameters,
        is_template,
        members,
        methods,
        base_classes,
        location,
    }
}

/// Detect the qualifier of a method (const, non-const, or rvalue-ref)
fn detect_method_qualifier(entity: &Entity) -> MethodQualifier {
    // Check if this is a const method
    let is_const = entity.is_const_method();

    // Check for && qualifier (rvalue reference qualifier)
    // LibClang doesn't expose this directly, so we check the function type
    let has_rvalue_ref_qualifier = if let Some(func_type) = entity.get_type() {
        // Check the display name for && qualifier
        let type_str = type_to_string(&func_type);
        debug_println!("DEBUG METHOD: type_str = {}", type_str);

        // The type string may contain "&&" for rvalue ref qualifier
        // Example: "void () &&" or "void (int) &&"
        type_str.contains(" &&") || type_str.ends_with("&&")
    } else {
        false
    };

    debug_println!("DEBUG METHOD: is_const={}, has_rvalue_ref={}", is_const, has_rvalue_ref_qualifier);

    // Determine the qualifier
    if has_rvalue_ref_qualifier {
        MethodQualifier::RvalueRef
    } else if is_const {
        MethodQualifier::Const
    } else {
        MethodQualifier::NonConst
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

    // Check if this is a static variable
    // In clang, static variables have StorageClass::Static
    let is_static = entity.get_storage_class() == Some(clang::StorageClass::Static);

    Variable {
        name,
        type_name,
        is_reference,
        is_pointer,
        is_const,
        is_unique_ptr,
        is_shared_ptr,
        is_static,
        location,
        is_pack: false,              // Will be set properly for function parameters
        pack_element_type: None,     // Will be set properly for function parameters
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

/// Helper function to extract function name from a CallExpr
/// Returns None if the function name cannot be determined
fn extract_function_name(call_expr: &Entity) -> Option<String> {
    debug_println!("DEBUG: Extracting function name from CallExpr");

    // Try to get name from the first child (usually the callee)
    for child in call_expr.get_children() {
        // Check DeclRefExpr or UnexposedExpr for function name
        if matches!(child.get_kind(), EntityKind::DeclRefExpr | EntityKind::UnexposedExpr) {
            if let Some(ref_entity) = child.get_reference() {
                if let Some(name) = ref_entity.get_name() {
                    debug_println!("DEBUG: Found function name '{}' from reference", name);
                    return Some(name);
                }
            }
            if let Some(name) = child.get_name() {
                debug_println!("DEBUG: Found function name '{}' from child name", name);
                return Some(name);
            }
        }
    }

    None
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
                for (idx, c) in children.iter().enumerate() {
                    debug_println!("DEBUG AST: CallExpr child[{}] kind: {:?}, name: {:?}, display_name: {:?}, reference: {:?}",
                        idx, c.get_kind(), c.get_name(), c.get_display_name(),
                        c.get_reference().map(|r| (r.get_kind(), r.get_name())));

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

                    // Phase 2: Check if this argument is a PackExpansionExpr
                    if c.get_kind() == EntityKind::PackExpansionExpr {
                        debug_println!("DEBUG STMT: Found PackExpansionExpr as function argument");

                        let mut pack_name = String::new();
                        let mut operation = "use".to_string();

                        // Look for the pack name and operation type
                        for pack_child in c.get_children() {
                            // Check if it's a CallExpr (could be std::forward or std::move)
                            if pack_child.get_kind() == EntityKind::CallExpr {
                                if let Some(callee_name) = extract_function_name(&pack_child) {
                                    debug_println!("DEBUG STMT: PackExpansion contains call to: {}", callee_name);
                                    if callee_name.contains("forward") {
                                        operation = "forward".to_string();
                                    } else if callee_name.contains("move") {
                                        operation = "move".to_string();
                                    }
                                }

                                // Find pack name inside the call
                                // Skip the first child (function name) and look for parameter references
                                debug_println!("DEBUG STMT: Searching for pack name in CallExpr children (count: {})",
                                    pack_child.get_children().len());
                                for call_child in pack_child.get_children() {
                                    debug_println!("DEBUG STMT: CallExpr child kind: {:?}", call_child.get_kind());
                                    if call_child.get_kind() == EntityKind::DeclRefExpr {
                                        if let Some(ref_entity) = call_child.get_reference() {
                                            debug_println!("DEBUG STMT: DeclRefExpr references entity kind: {:?}", ref_entity.get_kind());
                                            // Only accept if it references a parameter (ParmDecl), not a function
                                            if ref_entity.get_kind() == EntityKind::ParmDecl {
                                                if let Some(name) = ref_entity.get_name() {
                                                    debug_println!("DEBUG STMT: Found pack name '{}' inside CallExpr", name);
                                                    pack_name = name;
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // Direct DeclRefExpr (pack used without forward/move)
                            else if pack_child.get_kind() == EntityKind::DeclRefExpr {
                                debug_println!("DEBUG STMT: Found direct DeclRefExpr in PackExpansionExpr");
                                if let Some(ref_entity) = pack_child.get_reference() {
                                    if let Some(name) = ref_entity.get_name() {
                                        debug_println!("DEBUG STMT: Pack name from direct DeclRefExpr: '{}'", name);
                                        pack_name = name;
                                    }
                                }
                            }
                        }

                        if !pack_name.is_empty() {
                            debug_println!("DEBUG STMT: Pack expansion detected: pack='{}', operation='{}'",
                                pack_name, operation);
                            statements.push(Statement::PackExpansion {
                                pack_name,
                                operation,
                                location: extract_location(&c),
                            });
                        }
                    }
                    // Regular argument extraction
                    else if let Some(expr) = extract_expression(&c) {
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
                // Also check if this contains a PackExpansionExpr (fold expression)

                // First check for pack expansions in children
                let has_pack_expansion = child.get_children().iter()
                    .any(|c| c.get_kind() == EntityKind::PackExpansionExpr);

                if has_pack_expansion {
                    debug_println!("DEBUG STMT: UnexposedExpr contains PackExpansionExpr (fold expression)");
                    // Process PackExpansionExpr children
                    for ue_child in child.get_children() {
                        if ue_child.get_kind() == EntityKind::PackExpansionExpr {
                            // Recursively process the pack expansion
                            let pack_stmts = extract_compound_statement(&ue_child);
                            statements.extend(pack_stmts);
                        }
                    }
                } else {
                    // Regular UnexposedExpr handling
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
            for (idx, c) in children.iter().enumerate() {
                debug_println!("DEBUG AST: CallExpr child[{}] kind: {:?}, name: {:?}, display_name: {:?}, reference: {:?}",
                    idx, c.get_kind(), c.get_name(), c.get_display_name(),
                    c.get_reference().map(|r| (r.get_kind(), r.get_name())));
                    
                // For member function calls, check for MemberRefExpr first
                if c.get_kind() == EntityKind::MemberRefExpr {
                    // MemberRefExpr can be either a method call OR a field access
                    // We need to distinguish between them
                    if let Some(ref_entity) = c.get_reference() {
                        debug_println!("DEBUG AST: MemberRefExpr references: {:?}", ref_entity.get_name());
                        if let Some(n) = ref_entity.get_name() {
                            if name == "unknown" {
                                // Build qualified name for member functions and constructors
                                if ref_entity.get_kind() == EntityKind::Method || ref_entity.get_kind() == EntityKind::Constructor {
                                    name = get_qualified_name(&ref_entity);
                                } else if ref_entity.get_kind() == EntityKind::FieldDecl {
                                    // This is a field access, not a function call
                                    // Don't use it as the function name - it will be extracted as MemberAccess
                                    debug_println!("DEBUG AST: MemberRefExpr is field access, not function name");
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
            
            // Two-pass approach:
            // Pass 1: Identify which child provides the function name
            // Pass 2: Extract arguments, handling MemberRefExpr specially if it's the name provider

            let mut name_providing_child_idx: Option<usize> = None;

            // Pass 1: Find the name-providing child
            if name_providing_child_idx.is_none() {
                if name == "unknown" {
                    for (i, c) in children.iter().enumerate() {
                        match c.get_kind() {
                            EntityKind::DeclRefExpr | EntityKind::UnexposedExpr => {
                                // CRITICAL FIX: Check reference FIRST before name
                                // For template-dependent functions (like std::move in templates),
                                // the function name is in the reference, not the name field
                                if let Some(ref_entity) = c.get_reference() {
                                    if let Some(n) = ref_entity.get_name() {
                                        debug_println!("DEBUG AST: Got function name '{}' from reference (kind: {:?})", n, ref_entity.get_kind());
                                        name = n;
                                        name_providing_child_idx = Some(i);
                                        break;
                                    }
                                }

                                // Fallback: check name field (for non-template cases)
                                if let Some(n) = c.get_name() {
                                    debug_println!("DEBUG AST: Got function name '{}' from name field", n);
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

            // Pass 2: Extract arguments, handling MemberRefExpr specially if it's the name provider
            for (i, c) in children.into_iter().enumerate() {
                // If this child provided the function name
                if Some(i) == name_providing_child_idx {
                    // For method calls (MemberRefExpr is name provider), extract the receiver
                    if c.get_kind() == EntityKind::MemberRefExpr {
                        debug_println!("DEBUG AST: MemberRefExpr is name provider - extracting receiver");
                        // Extract receiver from MemberRefExpr's children
                        let member_children = c.get_children();
                        if !member_children.is_empty() {
                            // First child of MemberRefExpr is the receiver object
                            if let Some(recv_expr) = extract_expression(&member_children[0]) {
                                debug_println!("DEBUG AST: Extracted receiver from MemberRefExpr: {:?}", recv_expr);
                                args.push(recv_expr);
                            }
                        }
                    }
                    // Skip the name-providing child itself
                    continue;
                }

                // Extract all other children as normal arguments
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
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            debug_println!("DEBUG EXTRACT: UnexposedExpr with name={:?}, {} children",
                entity.get_name(), children.len());

            // Check if this UnexposedExpr has a reference (might be a method call)
            if let Some(ref_entity) = entity.get_reference() {
                debug_println!("  DEBUG EXTRACT: UnexposedExpr has reference: kind={:?}, name={:?}",
                    ref_entity.get_kind(), ref_entity.get_name());
            }

            // If there are exactly 2 children, this might be a binary operation (e.g., assignment)
            if children.len() == 2 {
                debug_println!("  DEBUG EXTRACT: UnexposedExpr with 2 children - checking for binary op");
                if let (Some(left), Some(right)) =
                    (extract_expression(&children[0]), extract_expression(&children[1])) {
                    debug_println!("  DEBUG EXTRACT: Extracted both children, treating as assignment");
                    // UnexposedExpr with 2 operands is typically an assignment in a const method
                    // (C++ allows it syntactically even though it's a semantic error)
                    return Some(Expression::BinaryOp {
                        left: Box::new(left),
                        op: "=".to_string(),
                        right: Box::new(right),
                    });
                }
            }

            // Otherwise, try to extract single child expression
            for child in children {
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
        EntityKind::MemberRefExpr => {
            // NEW: Parse member access expressions (obj.field)
            // MemberRefExpr is used for both field access and method calls
            // For field access, we extract: object from first child, field name from reference
            debug_println!("DEBUG: Found MemberRefExpr");

            // Get the field/member name from the entity's reference or name
            let field_name = if let Some(ref_entity) = entity.get_reference() {
                debug_println!("DEBUG: MemberRefExpr references kind={:?}, name={:?}",
                    ref_entity.get_kind(), ref_entity.get_name());
                // Check if it's a field (not a method)
                if ref_entity.get_kind() == EntityKind::FieldDecl {
                    ref_entity.get_name().unwrap_or_else(|| "unknown_field".to_string())
                } else {
                    // It's a method call, not field access - return None to let CallExpr handle it
                    debug_println!("DEBUG: MemberRefExpr is method, not field");
                    return None;
                }
            } else if let Some(name) = entity.get_name() {
                debug_println!("DEBUG: MemberRefExpr has name={}", name);
                name
            } else {
                debug_println!("DEBUG: MemberRefExpr has no reference or name");
                "unknown_field".to_string()
            };

            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            if !children.is_empty() {
                // First child is the object being accessed (explicit object.field)
                if let Some(object_expr) = extract_expression(&children[0]) {
                    debug_println!("DEBUG: MemberRefExpr explicit access: object={:?}, field={}", object_expr, field_name);
                    return Some(Expression::MemberAccess {
                        object: Box::new(object_expr),
                        field: field_name,
                    });
                }
            } else {
                // No children means implicit 'this->field' access in a method
                debug_println!("DEBUG: MemberRefExpr implicit 'this' access: this.{}", field_name);
                return Some(Expression::MemberAccess {
                    object: Box::new(Expression::Variable("this".to_string())),
                    field: field_name,
                });
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