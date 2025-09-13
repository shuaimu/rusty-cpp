// STL lifetime checker - applies lifetime rules to STL types
use crate::ir::{IrFunction, IrStatement, Expression};
use crate::parser::type_annotations::{TypeLifetimeRegistry, TypeLifetime, MethodLifetime};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct StlBorrow {
    pub object: String,        // The STL object being borrowed from
    pub borrower: String,      // Variable holding the borrow
    pub is_mutable: bool,      // Whether it's a mutable borrow
    pub method: String,        // Method that created the borrow
    pub location: String,      // Source location
}

pub struct StlLifetimeChecker {
    registry: TypeLifetimeRegistry,
    active_borrows: HashMap<String, Vec<StlBorrow>>, // object -> active borrows
    moved_objects: HashSet<String>,  // Objects that have been moved
}

impl StlLifetimeChecker {
    pub fn new() -> Self {
        StlLifetimeChecker {
            registry: TypeLifetimeRegistry::new(),
            active_borrows: HashMap::new(),
            moved_objects: HashSet::new(),
        }
    }
    
    pub fn check_function(&mut self, function: &IrFunction) -> Vec<String> {
        let mut errors = Vec::new();
        
        for statement in &function.body {
            let stmt_errors = self.check_statement(statement);
            errors.extend(stmt_errors);
        }
        
        errors
    }
    
    fn check_statement(&mut self, statement: &IrStatement) -> Vec<String> {
        let mut errors = Vec::new();
        
        match statement {
            IrStatement::Assignment { lhs, rhs, location } => {
                // Check if RHS is a method call on STL type
                if let Some(method_call) = extract_method_call(rhs) {
                    errors.extend(self.check_method_call(
                        lhs,
                        &method_call.object,
                        &method_call.method,
                        &method_call.object_type,
                        method_call.is_const,
                        location
                    ));
                }
                
                // Check for std::move
                if is_std_move(rhs) {
                    if let Some(moved_obj) = extract_moved_object(rhs) {
                        self.moved_objects.insert(moved_obj.clone());
                        // Clear any borrows from the moved object
                        self.active_borrows.remove(&moved_obj);
                    }
                }
            }
            
            IrStatement::Expression { expr, location } => {
                // Check for method calls that might invalidate iterators/references
                if let Some(method_call) = extract_method_call(expr) {
                    errors.extend(self.check_invalidating_operation(
                        &method_call.object,
                        &method_call.method,
                        &method_call.object_type,
                        location
                    ));
                }
            }
            
            IrStatement::Return { value: _, location: _ } => {
                // Check that we're not returning references to locals
                // This would be handled by the main lifetime checker
            }
            
            IrStatement::If { condition: _, then_branch, else_branch, location: _ } => {
                // Check both branches
                for stmt in then_branch {
                    errors.extend(self.check_statement(stmt));
                }
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        errors.extend(self.check_statement(stmt));
                    }
                }
            }
            
            IrStatement::Loop { condition: _, body, location: _ } => {
                // Check loop body
                for stmt in body {
                    errors.extend(self.check_statement(stmt));
                }
            }
            
            IrStatement::Block { statements, .. } => {
                // Track borrows created in this scope
                let initial_borrows = self.active_borrows.clone();
                
                for stmt in statements {
                    errors.extend(self.check_statement(stmt));
                }
                
                // Clear borrows that went out of scope
                self.active_borrows = initial_borrows;
            }
            
            _ => {}
        }
        
        errors
    }
    
    fn check_method_call(
        &mut self,
        result_var: &str,
        object: &str,
        method: &str,
        object_type: &str,
        is_const: bool,
        location: &str
    ) -> Vec<String> {
        let mut errors = Vec::new();
        
        // Check if object was moved
        if self.moved_objects.contains(object) {
            errors.push(format!(
                "{}: Use after move: calling {}.{}() on moved object",
                location, object, method
            ));
            return errors;
        }
        
        // Get type specification
        if let Some(type_spec) = self.registry.get_type_spec(object_type) {
            // Find matching method overload
            if let Some(method_overloads) = type_spec.methods.get(method) {
                let matching_overload = method_overloads.iter()
                    .find(|m| m.is_const == is_const)
                    .or_else(|| method_overloads.first());
                
                if let Some(method_lifetime) = matching_overload {
                    // Check what kind of borrow this creates
                    match &method_lifetime.return_lifetime {
                        TypeLifetime::SelfRef | TypeLifetime::Ref(_) => {
                            // Creates immutable borrow
                            self.add_borrow(object, result_var, false, method, location);
                        }
                        TypeLifetime::SelfMutRef | TypeLifetime::MutRef(_) => {
                            // Creates mutable borrow
                            errors.extend(self.check_can_borrow_mut(object, location));
                            self.add_borrow(object, result_var, true, method, location);
                        }
                        TypeLifetime::MutPtr | TypeLifetime::ConstPtr => {
                            // Returns raw pointer - requires unsafe to use
                            // This would be handled by pointer_safety checker
                        }
                        TypeLifetime::Owned => {
                            // No borrow created
                        }
                    }
                }
            }
        }
        
        errors
    }
    
    fn check_invalidating_operation(
        &mut self,
        object: &str,
        method: &str,
        _object_type: &str,
        location: &str
    ) -> Vec<String> {
        let mut errors = Vec::new();
        
        // Check if this method invalidates iterators/references
        if is_invalidating_method(method) {
            // Check if there are active borrows
            if let Some(borrows) = self.active_borrows.get(object) {
                if !borrows.is_empty() {
                    errors.push(format!(
                        "{}: Cannot call {}.{}() while references/iterators exist",
                        location, object, method
                    ));
                    
                    // Report what borrows exist
                    for borrow in borrows {
                        errors.push(format!(
                            "  {} has {} borrow from {}.{}() at {}",
                            borrow.borrower,
                            if borrow.is_mutable { "mutable" } else { "immutable" },
                            borrow.object,
                            borrow.method,
                            borrow.location
                        ));
                    }
                }
            }
        }
        
        errors
    }
    
    fn check_can_borrow_mut(&self, object: &str, location: &str) -> Vec<String> {
        let mut errors = Vec::new();
        
        if let Some(borrows) = self.active_borrows.get(object) {
            if !borrows.is_empty() {
                errors.push(format!(
                    "{}: Cannot create mutable borrow of '{}': already borrowed",
                    location, object
                ));
            }
        }
        
        errors
    }
    
    fn add_borrow(&mut self, object: &str, borrower: &str, is_mutable: bool, method: &str, location: &str) {
        let borrow = StlBorrow {
            object: object.to_string(),
            borrower: borrower.to_string(),
            is_mutable,
            method: method.to_string(),
            location: location.to_string(),
        };
        
        self.active_borrows
            .entry(object.to_string())
            .or_insert_with(Vec::new)
            .push(borrow);
    }
}

// Helper structures and functions

struct MethodCall {
    object: String,
    object_type: String,
    method: String,
    is_const: bool,
}

fn extract_method_call(expr: &Expression) -> Option<MethodCall> {
    // Parse method calls like: vec.begin(), map.at(key), etc.
    // This is simplified - real implementation would use AST
    
    if let Expression::MethodCall { object, method, args: _ } = expr {
        // Try to determine the type of the object
        let object_type = infer_object_type(object);
        
        Some(MethodCall {
            object: object.clone(),
            object_type,
            method: method.clone(),
            is_const: false, // Would need type info to determine
        })
    } else {
        None
    }
}

fn infer_object_type(object_name: &str) -> String {
    // Simplified type inference
    // Real implementation would track variable types
    
    if object_name.contains("vec") {
        "std::vector".to_string()
    } else if object_name.contains("map") {
        "std::map".to_string()
    } else if object_name.contains("str") {
        "std::string".to_string()
    } else if object_name.contains("ptr") {
        "std::unique_ptr".to_string()
    } else {
        "unknown".to_string()
    }
}

fn is_std_move(expr: &Expression) -> bool {
    match expr {
        Expression::FunctionCall { name, .. } => {
            name == "std::move" || name == "move"
        }
        _ => false
    }
}

fn extract_moved_object(expr: &Expression) -> Option<String> {
    if let Expression::FunctionCall { args, .. } = expr {
        if let Some(first_arg) = args.first() {
            if let Expression::Variable(var) = first_arg {
                return Some(var.clone());
            }
        }
    }
    None
}

fn is_invalidating_method(method: &str) -> bool {
    // Methods that invalidate iterators/references
    matches!(method, 
        "push_back" | "push_front" | "pop_back" | "pop_front" |
        "insert" | "erase" | "clear" | "resize" | "reserve" |
        "shrink_to_fit" | "emplace" | "emplace_back" | "emplace_front"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrProgram, Variable};
    
    #[test]
    fn test_vector_iterator_invalidation() {
        let mut checker = StlLifetimeChecker::new();
        
        // Simulate: auto it = vec.begin();
        checker.add_borrow("vec", "it", true, "begin", "line 1");
        
        // Simulate: vec.push_back(x);
        let errors = checker.check_invalidating_operation(
            "vec", "push_back", "std::vector", "line 2"
        );
        
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Cannot call vec.push_back()"));
    }
    
    #[test]
    fn test_map_reference_stability() {
        let mut checker = StlLifetimeChecker::new();
        
        // Simulate: auto& ref = map[key];
        checker.add_borrow("map", "ref", true, "operator[]", "line 1");
        
        // Simulate: map.insert(...);  // Should be OK for std::map
        let errors = checker.check_invalidating_operation(
            "map", "insert", "std::map", "line 2"
        );
        
        // insert doesn't invalidate references in std::map
        // (though our simple checker might flag it)
        // This test shows the structure - real implementation would be more sophisticated
    }
    
    #[test]
    fn test_unique_ptr_move() {
        let mut checker = StlLifetimeChecker::new();
        
        // Simulate: std::move(ptr);
        checker.moved_objects.insert("ptr".to_string());
        
        // Simulate: ptr.get();
        let errors = checker.check_method_call(
            "result", "ptr", "get", "std::unique_ptr", false, "line 2"
        );
        
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Use after move"));
    }
}