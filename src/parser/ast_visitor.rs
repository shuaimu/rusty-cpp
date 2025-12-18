use clang::{Entity, EntityKind, Type, TypeKind};
use crate::debug_println;

/// Check if a function name is std::move or a namespace-qualified move
fn is_move_function(name: &str) -> bool {
    name == "move" || name == "std::move" || name.ends_with("::move")
}

/// Check if a function name is std::forward or a namespace-qualified forward
fn is_forward_function(name: &str) -> bool {
    name == "forward" || name == "std::forward" || name.ends_with("::forward")
}

/// Check if an entity has an @unsafe annotation by reading source file
fn check_for_unsafe_annotation(entity: &Entity) -> bool {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    // Try get_comment() first (works for some entity types)
    if let Some(comment) = entity.get_comment() {
        if comment.contains("@unsafe") {
            return true;
        }
    }

    // For CompoundStmt and other entities, check the source file directly
    let location = match entity.get_location() {
        Some(loc) => loc,
        None => return false,
    };

    let file_location = location.get_file_location();
    let file = match file_location.file {
        Some(f) => f,
        None => return false,
    };

    let file_path = file.get_path();
    let block_line = file_location.line as usize;

    // Read the source file and check the line before the block
    let file_handle = match File::open(&file_path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = BufReader::new(file_handle);
    let mut current_line = 0;
    let mut prev_line = String::new();

    for line_result in reader.lines() {
        current_line += 1;
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Check if we're at the block's line
        if current_line == block_line {
            // Check the previous line for @unsafe annotation
            let trimmed = prev_line.trim();
            if trimmed.starts_with("//") && trimmed.contains("@unsafe") {
                debug_println!("DEBUG UNSAFE: Found @unsafe annotation for block at line {}", block_line);
                return true;
            }
            // Also check for /* @unsafe */ style comments
            if trimmed.contains("/*") && trimmed.contains("@unsafe") && trimmed.contains("*/") {
                debug_println!("DEBUG UNSAFE: Found @unsafe annotation for block at line {}", block_line);
                return true;
            }
            return false;
        }

        prev_line = line;
    }

    false
}

/// Check if a field declaration has the 'mutable' keyword
/// Read the source code around the declaration to detect 'mutable'
fn check_for_mutable_keyword(entity: &Entity) -> bool {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let location = match entity.get_location() {
        Some(loc) => loc,
        None => return false,
    };

    let file_location = location.get_file_location();
    let file = match file_location.file {
        Some(f) => f,
        None => return false,
    };

    let file_path = file.get_path();
    let decl_line = file_location.line as usize;

    // Read the source file and check the line with the declaration
    let file_handle = match File::open(&file_path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let reader = BufReader::new(file_handle);
    let mut current_line = 0;

    for line_result in reader.lines() {
        current_line += 1;
        let line = match line_result {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Check if we're at the declaration line
        if current_line == decl_line {
            // Check if the line contains the 'mutable' keyword
            // Look for word boundary to avoid matching "immutable" or similar
            let trimmed = line.trim();
            let has_mutable = trimmed.starts_with("mutable ") ||
                             trimmed.contains(" mutable ") ||
                             (trimmed == "mutable");

            debug_println!("DEBUG MUTABLE: Line {}: '{}' -> has_mutable = {}",
                decl_line, trimmed, has_mutable);

            return has_mutable;
        }
    }

    false
}

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
    // RAII Phase 2: Track if class has a destructor
    pub has_destructor: bool,              // True if class has ~ClassName()
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
    pub is_mutable: bool,                      // C++ mutable keyword (for interior mutability)
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
    // Lambda expression with captures (for safety checking)
    LambdaExpr {
        captures: Vec<LambdaCaptureKind>,
        location: SourceLocation,
    },
}

/// Represents a lambda capture
#[derive(Debug, Clone)]
pub enum LambdaCaptureKind {
    /// [&] - default reference capture
    DefaultRef,
    /// [=] - default copy capture
    DefaultCopy,
    /// [&x] - explicit reference capture
    ByRef(String),
    /// [x] - explicit copy capture
    ByCopy(String),
    /// [x = expr] - init capture (includes move captures)
    Init { name: String, is_move: bool },
    /// [this] - captures this pointer
    This,
    /// [*this] - captures this by copy (C++17)
    ThisCopy,
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
    // Lambda expression with captures
    Lambda {
        captures: Vec<LambdaCaptureKind>,
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

    // Use qualified name for ALL functions (methods AND free functions in namespaces)
    // This ensures:
    // 1. Methods get qualified names like "MyClass::method"
    // 2. Namespaced functions get qualified names like "network::send_message"
    // 3. External library functions get qualified names like "YAML::detail::node_data::get"
    // This prevents false matches where unqualified "get" incorrectly matches "rusty::Cell::get"
    let name = get_qualified_name(entity);
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

    // Extract template parameters from:
    // 1. ClassTemplate parent (for template class methods)
    // 2. FunctionTemplate entity itself (for free template functions)
    // 3. FunctionTemplate parent (fallback for nested cases)
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
    } else if entity.get_kind() == EntityKind::FunctionTemplate {
        // Entity IS a FunctionTemplate - extract parameters directly from it
        debug_println!("TEMPLATE: Free template function (entity is FunctionTemplate), extracting parameters");
        extract_template_parameters(entity)
    } else {
        // For free functions, check if parent is a FunctionTemplate (fallback)
        if let Some(parent) = entity.get_semantic_parent() {
            if parent.get_kind() == EntityKind::FunctionTemplate {
                debug_println!("TEMPLATE: Free template function, extracting parameters from FunctionTemplate parent");
                extract_template_parameters(&parent)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
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

    // Bug #8 fix: Use qualified name for classes to prevent namespace collision
    // e.g., "yaml::Node" instead of just "Node"
    let name = get_qualified_name(entity);
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
    let mut has_destructor = false;  // RAII Phase 2: Track destructors

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
            EntityKind::Destructor => {
                // RAII Phase 2: Mark class as having a destructor
                has_destructor = true;
                debug_println!("DEBUG PARSE: Class '{}' has user-defined destructor", name);
                let method = extract_function(&child);
                methods.push(method);
            }
            EntityKind::Method | EntityKind::Constructor => {
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

    debug_println!("DEBUG PARSE: Class '{}' has {} members, {} methods, {} base classes, has_destructor={}",
        name, members.len(), methods.len(), base_classes.len(), has_destructor);

    Class {
        name,
        template_parameters,
        is_template,
        members,
        methods,
        base_classes,
        location,
        has_destructor,  // RAII Phase 2
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

    // Check if this is a mutable field (C++ mutable keyword)
    // We need to read the source code to check for the 'mutable' keyword
    // because libclang doesn't expose mutable as a storage class
    let is_mutable = check_for_mutable_keyword(entity);
    debug_println!("DEBUG MUTABLE: Field '{}' is_mutable = {}", name, is_mutable);

    Variable {
        name,
        type_name,
        is_reference,
        is_pointer,
        is_const,
        is_unique_ptr,
        is_shared_ptr,
        is_static,
        is_mutable,
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
                    
                    if let Some(_n) = ref_entity.get_name() {
                        // Build qualified name for ALL functions (methods AND free functions)
                        // This ensures:
                        // 1. Methods get qualified names like "MyClass::method"
                        // 2. Namespaced functions get qualified names like "mylib::dangerous_op"
                        // This prevents false matches between same-named functions in different namespaces
                        name = get_qualified_name(&ref_entity);
                    }
                }
                
                // If this looks like a variable declaration, skip it
                if is_likely_var_decl && children.is_empty() {
                    debug_println!("DEBUG AST: Skipping variable declaration disguised as CallExpr: {}", name);
                    continue;
                }
                
                // Try to extract the function name from children
                // CRITICAL: EXCLUDE entities that reference variables/parameters
                // (which caused the "rhs"/"schema"/"vv" bugs where args were mistaken for function names)
                for (idx, c) in children.iter().enumerate() {
                    debug_println!("DEBUG AST: CallExpr child[{}] kind: {:?}, name: {:?}, display_name: {:?}, reference: {:?}",
                        idx, c.get_kind(), c.get_name(), c.get_display_name(),
                        c.get_reference().map(|r| (r.get_kind(), r.get_name())));

                    if c.get_kind() == EntityKind::UnexposedExpr || c.get_kind() == EntityKind::DeclRefExpr {
                        // Check if this entity references a variable/parameter - if so, skip it
                        if let Some(ref_entity) = c.get_reference() {
                            let ref_kind = ref_entity.get_kind();
                            // EXCLUDE variables and parameters - these are arguments, not function names
                            if ref_kind == EntityKind::VarDecl || ref_kind == EntityKind::ParmDecl {
                                debug_println!("DEBUG AST: Skipping variable/parameter reference: {:?}", ref_kind);
                                continue;
                            }
                            // This is a function reference - use qualified name
                            if name == "unknown" {
                                let qualified = get_qualified_name(&ref_entity);
                                debug_println!("DEBUG AST: Got qualified name from child reference: {}", qualified);
                                name = qualified;
                            }
                        } else {
                            // No reference (template-dependent) - use unqualified name as fallback
                            if let Some(n) = c.get_name() {
                                if name == "unknown" {
                                    debug_println!("DEBUG AST: Got name from child (template-dependent): {}", n);
                                    name = n;
                                }
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
                                    if let Some(_n) = ref_entity.get_name() {
                                        // Use qualified name for ALL function calls
                                        name = get_qualified_name(&ref_entity);
                                        name_providing_child_idx = Some(i);
                                        break;
                                    }
                                } else {
                                    debug_println!("DEBUG AST: MemberRefExpr has NO reference!");
                                }
                            }
                            EntityKind::DeclRefExpr | EntityKind::UnexposedExpr => {
                                // CRITICAL FIX: EXCLUDE variables/parameters
                                // (which caused the "rhs"/"schema"/"vv" bugs)
                                if let Some(ref_entity) = c.get_reference() {
                                    let ref_kind = ref_entity.get_kind();
                                    if ref_kind == EntityKind::VarDecl || ref_kind == EntityKind::ParmDecl {
                                        continue; // Skip variables/parameters
                                    }
                                    // Use qualified name for function calls
                                    let qualified = get_qualified_name(&ref_entity);
                                    name = qualified;
                                    name_providing_child_idx = Some(i);
                                    break;
                                } else if let Some(n) = c.get_name() {
                                    // No reference (template-dependent) - use unqualified name
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
                            debug_println!("DEBUG AST: Extracting receiver from MemberRefExpr");
                            let member_children: Vec<Entity> = c.get_children().into_iter().collect();
                            if !member_children.is_empty() {
                                // Has children - extract receiver from first child
                                if let Some(receiver_expr) = extract_expression(&member_children[0]) {
                                    // Check if receiver type is a pointer (means -> was used)
                                    // ptr->method() is semantically (*ptr).method(), so wrap in Dereference
                                    let is_arrow = member_children[0].get_type()
                                        .map(|t| matches!(t.get_kind(), TypeKind::Pointer))
                                        .unwrap_or(false);

                                    if is_arrow {
                                        debug_println!("DEBUG AST: Arrow method call (stmt) - receiver is pointer: (*{:?})", receiver_expr);
                                        args.push(Expression::Dereference(Box::new(receiver_expr)));
                                    } else {
                                        debug_println!("DEBUG AST: Dot method call (stmt) - receiver: {:?}", receiver_expr);
                                        args.push(receiver_expr);
                                    }
                                }
                            } else {
                                // No children - the receiver is a simple variable/parameter
                                // Extract it from the MemberRefExpr's name or display name
                                if let Some(member_name) = c.get_name() {
                                    // The name might be like "m" for m.content_size()
                                    // But we need to be careful - the name might be the method name
                                    // Check if this is different from our extracted function name
                                    if member_name != name && !name.ends_with(&member_name) {
                                        debug_println!("DEBUG AST: Found receiver from MemberRefExpr name: {}", member_name);
                                        args.push(Expression::Variable(member_name));
                                    }
                                } else if let Some(display) = c.get_display_name() {
                                    // Try display name as fallback
                                    if display != name && !name.ends_with(&display) && !display.is_empty() {
                                        debug_println!("DEBUG AST: Found receiver from MemberRefExpr display: {}", display);
                                        args.push(Expression::Variable(display));
                                    }
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
                                    if is_forward_function(&callee_name) {
                                        operation = "forward".to_string();
                                    } else if is_move_function(&callee_name) {
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

                // Check if this block is preceded by @unsafe comment
                let is_unsafe = check_for_unsafe_annotation(&child);
                if is_unsafe {
                    debug_println!("DEBUG UNSAFE: Found @unsafe block");
                    statements.push(Statement::EnterUnsafe);
                }

                statements.extend(extract_compound_statement(&child));

                if is_unsafe {
                    statements.push(Statement::ExitUnsafe);
                }

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
            // Check if this references a field declaration (member variable)
            // If so, it's an implicit this->field access
            if let Some(ref_entity) = entity.get_reference() {
                if ref_entity.get_kind() == EntityKind::FieldDecl {
                    // This is a member field access - convert to this.field
                    if let Some(field_name) = entity.get_name() {
                        debug_println!("DEBUG: DeclRefExpr to FieldDecl '{}' - converting to this.{}", field_name, field_name);
                        return Some(Expression::MemberAccess {
                            object: Box::new(Expression::Variable("this".to_string())),
                            field: field_name,
                        });
                    }
                }
            }
            // Regular variable reference
            entity.get_name().map(Expression::Variable)
        }
        EntityKind::ThisExpr => {
            // The 'this' pointer in C++ methods
            Some(Expression::Variable("this".to_string()))
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
            let mut method_name_from_callexpr = false;
            if let Some(ref_entity) = entity.get_reference() {
                debug_println!("DEBUG AST: CallExpr itself references: {:?}", ref_entity.get_name());

                // Check if it references a type (struct/class/typedef)
                // BUT: A CallExpr with 0 children is likely a constructor/declaration
                if children.is_empty() {
                    if let Some(n) = ref_entity.get_name() {
                        name = n;
                    }
                    debug_println!("DEBUG AST: CallExpr with 0 children referencing '{}' - likely a variable declaration", name);
                    return None;  // Not a function call, it's a variable declaration
                }

                // Build qualified name for all functions to avoid namespace collisions
                // This ensures ns1::func and ns2::func are distinguished
                if ref_entity.get_kind() == EntityKind::Method
                    || ref_entity.get_kind() == EntityKind::Constructor
                    || ref_entity.get_kind() == EntityKind::FunctionDecl
                    || ref_entity.get_kind() == EntityKind::FunctionTemplate {
                    name = get_qualified_name(&ref_entity);
                    if ref_entity.get_kind() == EntityKind::Method || ref_entity.get_kind() == EntityKind::Constructor {
                        method_name_from_callexpr = true;
                    }
                    debug_println!("DEBUG AST: Function name extracted from CallExpr reference: {}", name);
                } else if let Some(n) = ref_entity.get_name() {
                    name = n;
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

            // CRITICAL FIX: If the method name was extracted from CallExpr's own reference
            // (common for template class methods), we need to find the MemberRefExpr child
            // and mark it as the name provider so the receiver can be extracted
            if method_name_from_callexpr {
                for (i, c) in children.iter().enumerate() {
                    if c.get_kind() == EntityKind::MemberRefExpr {
                        if let Some(ref_entity) = c.get_reference() {
                            if ref_entity.get_kind() == EntityKind::Method {
                                debug_println!("DEBUG AST: Found MemberRefExpr at index {} for method call", i);
                                name_providing_child_idx = Some(i);
                                break;
                            }
                        }
                    }
                }
            }

            // Pass 1: Find the name-providing child
            if name_providing_child_idx.is_none() {
                if name == "unknown" {
                    for (i, c) in children.iter().enumerate() {
                        match c.get_kind() {
                            EntityKind::DeclRefExpr | EntityKind::UnexposedExpr => {
                                // CRITICAL FIX: EXCLUDE variables/parameters
                                // (which caused the "rhs"/"schema"/"vv" bugs)
                                if let Some(ref_entity) = c.get_reference() {
                                    let ref_kind = ref_entity.get_kind();
                                    if ref_kind == EntityKind::VarDecl || ref_kind == EntityKind::ParmDecl {
                                        continue; // Skip variables/parameters
                                    }
                                    // Bug #8 fix: Use qualified name for free functions too
                                    // This ensures namespace::function is captured correctly
                                    let n = get_qualified_name(&ref_entity);
                                    debug_println!("DEBUG AST: Got function name '{}' from reference (kind: {:?})", n, ref_entity.get_kind());
                                    name = n;
                                    name_providing_child_idx = Some(i);
                                    break;
                                }
                                // No reference - might be template-dependent, use name directly
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
                                // Check if receiver type is a pointer (means -> was used)
                                // ptr->method() is semantically (*ptr).method(), so wrap in Dereference
                                let is_arrow = member_children[0].get_type()
                                    .map(|t| matches!(t.get_kind(), TypeKind::Pointer))
                                    .unwrap_or(false);

                                if is_arrow {
                                    debug_println!("DEBUG AST: Arrow method call - receiver is pointer: (*{:?})", recv_expr);
                                    args.push(Expression::Dereference(Box::new(recv_expr)));
                                } else {
                                    debug_println!("DEBUG AST: Dot method call - extracted receiver: {:?}", recv_expr);
                                    args.push(recv_expr);
                                }
                            }
                        } else {
                            // No children - the receiver might be implicit or a simple variable
                            // For a MemberRefExpr like m.content_size(), we need to extract "m"
                            // Unfortunately libclang doesn't always give us this directly
                            // We'll need to handle this case differently
                            debug_println!("DEBUG AST: MemberRefExpr has no children, cannot extract receiver");
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
            if is_move_function(&name) {
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
        EntityKind::ParenExpr => {
            // Parenthesized expression - just extract the inner expression
            // e.g., (*n) in (*n).value_ - we want to get the Dereference(n) inside
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            if !children.is_empty() {
                debug_println!("DEBUG: ParenExpr has child: kind={:?}", children[0].get_kind());
                return extract_expression(&children[0]);
            }
            None
        }
        // C++ cast expressions - extract the inner expression
        // static_cast<T*>(ptr), dynamic_cast<T*>(ptr), reinterpret_cast<T*>(ptr), const_cast<T*>(ptr)
        // These are transparent for borrow checking - we care about what's being cast
        EntityKind::StaticCastExpr
        | EntityKind::DynamicCastExpr
        | EntityKind::ReinterpretCastExpr
        | EntityKind::ConstCastExpr
        | EntityKind::CStyleCastExpr => {
            // C++ cast expressions - extract the inner expression being cast
            // The cast itself is transparent for borrow checking
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            // Find the expression being cast (not the type reference)
            for child in &children {
                if child.get_kind() != EntityKind::TypeRef {
                    if let Some(expr) = extract_expression(child) {
                        return Some(expr);
                    }
                }
            }
            None
        }
        EntityKind::UnaryOperator => {
            // Check if it's address-of (&) or dereference (*)
            // Other unary operators (!, ~, -, +) should be treated as simple expressions
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            if !children.is_empty() {
                if let Some(inner) = extract_expression(&children[0]) {
                    // Try to determine the operator type
                    // LibClang doesn't give us the operator directly, but we can check the types
                    if let Some(result_type) = entity.get_type() {
                        let type_str = type_to_string(&result_type);

                        if let Some(child_type) = children[0].get_type() {
                            let child_type_str = type_to_string(&child_type);

                            // If child is pointer and result is not, it's dereference (*)
                            if child_type_str.contains('*') && !type_str.contains('*') {
                                return Some(Expression::Dereference(Box::new(inner)));
                            }
                            // If child is not pointer but result is, it's address-of (&)
                            else if !child_type_str.contains('*') && type_str.contains('*') {
                                return Some(Expression::AddressOf(Box::new(inner)));
                            }
                            // Otherwise, it's a non-pointer unary operator (!, ~, -, +)
                            // These don't affect ownership/borrowing, so just return the inner expression
                            // wrapped in a BinaryOp with the operator for completeness
                            else {
                                // For borrow checking purposes, these operators are transparent
                                // Just return the inner expression since we don't need to track
                                // the arithmetic/logical operation
                                return Some(inner);
                            }
                        }
                    }
                    // If we couldn't get types, return inner expression (conservative: don't assume pointer op)
                    return Some(inner);
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
                // First child is the object being accessed (explicit object.field or ptr->field)
                if let Some(object_expr) = extract_expression(&children[0]) {
                    // Check if object type is a pointer (means -> was used, not .)
                    // ptr->field is semantically (*ptr).field, so wrap in Dereference
                    let is_arrow = children[0].get_type()
                        .map(|t| matches!(t.get_kind(), TypeKind::Pointer))
                        .unwrap_or(false);

                    if is_arrow {
                        debug_println!("DEBUG: MemberRefExpr arrow access: (*{:?}).{}", object_expr, field_name);
                        return Some(Expression::MemberAccess {
                            object: Box::new(Expression::Dereference(Box::new(object_expr))),
                            field: field_name,
                        });
                    } else {
                        debug_println!("DEBUG: MemberRefExpr dot access: {:?}.{}", object_expr, field_name);
                        return Some(Expression::MemberAccess {
                            object: Box::new(object_expr),
                            field: field_name,
                        });
                    }
                }
            } else {
                // No children means implicit 'this->field' access in a method
                // 'this' is guaranteed valid inside member functions, so NOT unsafe
                // (Unlike arbitrary raw pointers, 'this' cannot be null/invalid in well-formed code)
                debug_println!("DEBUG: MemberRefExpr implicit 'this' access: this.{}", field_name);
                return Some(Expression::MemberAccess {
                    object: Box::new(Expression::Variable("this".to_string())),
                    field: field_name,
                });
            }
            None
        }
        EntityKind::LambdaExpr => {
            // Extract lambda captures by analyzing the AST structure
            // In libclang, lambda captures appear as:
            // - Reference capture: VariableRef only (no DeclRefExpr for that variable)
            // - Copy capture: VariableRef + DeclRefExpr for the same variable
            // - Default capture ([&] or [=]): No VariableRef/DeclRefExpr, must check type fields
            // - The body is a CompoundStmt
            debug_println!("DEBUG LAMBDA PARSER: Found LambdaExpr!");

            let mut captures = Vec::new();

            // Collect all VariableRef entries (these are the captured variables)
            let mut var_refs: Vec<String> = Vec::new();
            // Collect all DeclRefExpr entries (these indicate copy captures)
            let mut decl_refs: std::collections::HashSet<String> = std::collections::HashSet::new();
            // Track if we found any explicit captures
            let mut has_explicit_captures = false;
            // Track if we found a move call (indicates move capture)
            let mut has_move_call = false;

            for child in entity.get_children() {
                debug_println!("DEBUG LAMBDA child: kind={:?} name={:?}", child.get_kind(), child.get_name());
                match child.get_kind() {
                    EntityKind::VariableRef => {
                        has_explicit_captures = true;
                        if let Some(var_name) = child.get_name() {
                            var_refs.push(var_name);
                        }
                    }
                    EntityKind::DeclRefExpr => {
                        // DeclRefExpr indicates a copy capture (part of copy init expr)
                        if let Some(var_name) = child.get_name() {
                            decl_refs.insert(var_name);
                        }
                    }
                    EntityKind::CallExpr => {
                        // CallExpr with 'move' indicates a move capture [y = std::move(x)]
                        if let Some(name) = child.get_name() {
                            if name == "move" {
                                has_move_call = true;
                            }
                        }
                    }
                    EntityKind::ThisExpr => {
                        // Capturing 'this'
                        has_explicit_captures = true;
                        captures.push(LambdaCaptureKind::This);
                    }
                    _ => {}
                }
            }

            // If no explicit captures found, check if lambda uses default capture
            // by looking at the source code range
            debug_println!("DEBUG LAMBDA: has_explicit_captures={}", has_explicit_captures);
            if !has_explicit_captures {
                // Try to get the source range and parse the capture specifier
                if let Some(range) = entity.get_range() {
                    if let Some(file) = range.get_start().get_file_location().file {
                        if let Ok(content) = std::fs::read_to_string(file.get_path()) {
                            let start_line = range.get_start().get_file_location().line as usize;
                            let start_col = range.get_start().get_file_location().column as usize;
                            debug_println!("DEBUG LAMBDA: Source parsing at line={} col={}", start_line, start_col);

                            if let Some(line) = content.lines().nth(start_line.saturating_sub(1)) {
                                debug_println!("DEBUG LAMBDA: Line content: '{}'", line);
                                // Find the capture list: [...]
                                if let Some(bracket_start) = line.get(start_col.saturating_sub(1)..).and_then(|s| s.find('[')) {
                                    let search_start = start_col.saturating_sub(1) + bracket_start;
                                    if let Some(rest) = line.get(search_start..) {
                                        if let Some(bracket_end) = rest.find(']') {
                                            let capture_list = &rest[1..bracket_end];
                                            debug_println!("DEBUG LAMBDA: Capture list from source: '{}'", capture_list);

                                            // Check for default reference capture [&]
                                            if capture_list.trim() == "&" {
                                                debug_println!("DEBUG LAMBDA: Default reference capture [&] detected");
                                                captures.push(LambdaCaptureKind::DefaultRef);
                                            }
                                            // Check for default copy capture [=]
                                            else if capture_list.trim() == "=" {
                                                debug_println!("DEBUG LAMBDA: Default copy capture [=] detected");
                                                captures.push(LambdaCaptureKind::DefaultCopy);
                                            }
                                            // Check for 'this' capture [this]
                                            else if capture_list.trim() == "this" {
                                                debug_println!("DEBUG LAMBDA: 'this' capture [this] detected");
                                                captures.push(LambdaCaptureKind::This);
                                            }
                                            // Check for '*this' capture [*this]
                                            else if capture_list.trim() == "*this" {
                                                debug_println!("DEBUG LAMBDA: '*this' capture [*this] detected");
                                                captures.push(LambdaCaptureKind::ThisCopy);
                                            }
                                            // Check for init captures [x = expr] or [x = std::move(y)]
                                            else if capture_list.contains('=') && !capture_list.starts_with('&') {
                                                // This is an init capture - safe (copy or move)
                                                debug_println!("DEBUG LAMBDA: Init capture detected");
                                                // Extract variable name before the '='
                                                if let Some(eq_pos) = capture_list.find('=') {
                                                    let var_name = capture_list[..eq_pos].trim().to_string();
                                                    let is_move = capture_list.contains("std::move") ||
                                                                 capture_list.contains("move(");
                                                    captures.push(LambdaCaptureKind::Init {
                                                        name: var_name,
                                                        is_move,
                                                    });
                                                }
                                            }
                                            // Check for explicit reference captures [&x, &y, ...]
                                            else if capture_list.starts_with('&') && !capture_list.contains(',') {
                                                // [&x] - single explicit reference capture
                                                // Already handled by VariableRef detection above
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Determine capture type for each VariableRef (explicit captures)
            // Key insight from libclang patterns:
            // - Reference capture [&x]: VariableRef 'x' only (no DeclRefExpr, no CallExpr)
            // - Copy capture [x]: VariableRef 'x' + DeclRefExpr 'x' (same name)
            // - Init copy capture [y = x]: VariableRef 'y' + DeclRefExpr 'x' (different names, NO overlap!)
            // - Init move capture [y = std::move(x)]: VariableRef 'y' + CallExpr 'move'
            // - Mixed capture [x, &y]: VariableRef 'x' & 'y', DeclRefExpr 'x' only (partial overlap)

            // Check if this is an init capture situation:
            // For init captures [y = x], var_refs and decl_refs have NO overlapping names
            let var_ref_set: std::collections::HashSet<_> = var_refs.iter().cloned().collect();
            let has_any_overlap = var_ref_set.intersection(&decl_refs).next().is_some();
            let is_init_capture_pattern = !decl_refs.is_empty() && !has_any_overlap;

            for var_name in var_refs {
                if decl_refs.contains(&var_name) {
                    // Has corresponding DeclRefExpr with SAME name = copy capture
                    debug_println!("DEBUG LAMBDA: Copy capture of '{}'", var_name);
                    captures.push(LambdaCaptureKind::ByCopy(var_name));
                } else if has_move_call {
                    // Has move() call = init move capture [y = std::move(x)]
                    debug_println!("DEBUG LAMBDA: Init move capture '{}'", var_name);
                    captures.push(LambdaCaptureKind::Init {
                        name: var_name,
                        is_move: true,
                    });
                } else if is_init_capture_pattern {
                    // Has DeclRefExpr with entirely DIFFERENT names = init capture [y = x]
                    // The VariableRef is the new capture name, DeclRefExpr is the source
                    debug_println!("DEBUG LAMBDA: Init copy capture '{}'", var_name);
                    captures.push(LambdaCaptureKind::Init {
                        name: var_name,
                        is_move: false,
                    });
                } else {
                    // No matching DeclRefExpr = reference capture
                    debug_println!("DEBUG LAMBDA: Reference capture of '{}'", var_name);
                    captures.push(LambdaCaptureKind::ByRef(var_name));
                }
            }

            debug_println!("DEBUG LAMBDA: Found lambda with {} captures: {:?}",
                captures.len(), captures);

            Some(Expression::Lambda { captures })
        }
        EntityKind::ArraySubscriptExpr => {
            // Array subscript: arr[i], data[idx], etc.
            // The first child is the array/pointer, second is the index
            // For lifetime purposes, the source is the array (first child)
            let children: Vec<Entity> = entity.get_children().into_iter().collect();
            if !children.is_empty() {
                // First child is the array or pointer being indexed
                if let Some(array_expr) = extract_expression(&children[0]) {
                    debug_println!("DEBUG: ArraySubscriptExpr - array/pointer source: {:?}", array_expr);
                    // Return the array expression as the source
                    // This handles cases like data[i] returning reference to member data
                    return Some(array_expr);
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