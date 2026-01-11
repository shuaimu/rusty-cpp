use std::collections::{HashMap, HashSet};
use crate::ir::BorrowKind;
use crate::parser::MethodQualifier;

/// Tracks the state of member fields within a method based on 'this' pointer semantics
///
/// Enforces Rust-like rules:
/// - Const methods (&self): Can read fields, cannot modify or move
/// - Non-const methods (&mut self): Can read and modify fields, CANNOT move fields
/// - Rvalue methods (self): Can do anything including moving fields
#[derive(Debug, Clone)]
pub struct ThisPointerTracker {
    /// The qualifier of the current method (const, non-const, or &&)
    method_qualifier: Option<MethodQualifier>,

    /// Fields that have been moved (no longer accessible)
    moved_fields: HashSet<String>,

    /// Fields that are currently borrowed and their borrow kind
    borrowed_fields: HashMap<String, BorrowKind>,
}

impl ThisPointerTracker {
    /// Create a new tracker for a method with the given qualifier
    pub fn new(method_qualifier: Option<MethodQualifier>) -> Self {
        Self {
            method_qualifier,
            moved_fields: HashSet::new(),
            borrowed_fields: HashMap::new(),
        }
    }

    /// Check if we can read a member field
    ///
    /// Rules:
    /// - All method types can read fields (const, non-const, &&)
    /// - Cannot read moved fields
    pub fn can_read_member(&self, field: &str) -> Result<(), String> {
        if self.moved_fields.contains(field) {
            return Err(format!("Cannot read field '{}': field has been moved", field));
        }
        Ok(())
    }

    /// Check if we can modify a member field
    ///
    /// Rules:
    /// - Const methods (&self): CANNOT modify
    /// - Non-const methods (&mut self): CAN modify
    /// - Rvalue methods (self): CAN modify
    /// - Cannot modify moved fields
    /// - Cannot modify immutably borrowed fields
    pub fn can_modify_member(&self, field: &str) -> Result<(), String> {
        if self.moved_fields.contains(field) {
            return Err(format!("Cannot modify field '{}': field has been moved", field));
        }

        // Check method qualifier
        if let Some(MethodQualifier::Const) = self.method_qualifier {
            return Err(format!(
                "Cannot modify field '{}' in const method (use non-const method for &mut self semantics)",
                field
            ));
        }

        // Check if field is borrowed immutably
        if let Some(BorrowKind::Immutable) = self.borrowed_fields.get(field) {
            return Err(format!(
                "Cannot modify field '{}': field is currently borrowed immutably",
                field
            ));
        }

        Ok(())
    }

    /// Check if we can move a member field
    ///
    /// Rules:
    /// - Const methods (&self): CANNOT move
    /// - Non-const methods (&mut self): CANNOT move (key Rust restriction!)
    /// - Rvalue methods (self): CAN move
    /// - Cannot move already-moved fields
    /// - Cannot move borrowed fields
    pub fn can_move_member(&self, field: &str) -> Result<(), String> {
        if self.moved_fields.contains(field) {
            return Err(format!("Cannot move field '{}': field has already been moved", field));
        }

        // Check if field is borrowed
        if self.borrowed_fields.contains_key(field) {
            return Err(format!(
                "Cannot move field '{}': field is currently borrowed",
                field
            ));
        }

        // Check method qualifier - this is the key restriction
        match self.method_qualifier {
            Some(MethodQualifier::Const) => {
                Err(format!(
                    "Cannot move field '{}' from const method (requires && method for self ownership)",
                    field
                ))
            }
            Some(MethodQualifier::NonConst) => {
                Err(format!(
                    "Cannot move field '{}' from &mut self method (use && qualified method for self ownership)",
                    field
                ))
            }
            Some(MethodQualifier::RvalueRef) => {
                // && methods have full ownership - can move
                Ok(())
            }
            None => {
                // Not in a method context - allow for now (free functions)
                Ok(())
            }
        }
    }

    /// Check if we can borrow a member field
    ///
    /// Rules:
    /// - Const methods: Can only create immutable borrows
    /// - Non-const methods: Can create mutable or immutable borrows
    /// - Rvalue methods: Can create any borrow
    /// - Cannot borrow moved fields
    /// - Respect existing borrows (no mutable + immutable, no multiple mutable)
    pub fn can_borrow_member(&self, field: &str, kind: BorrowKind) -> Result<(), String> {
        if self.moved_fields.contains(field) {
            return Err(format!("Cannot borrow field '{}': field has been moved", field));
        }

        // Const methods can only create immutable borrows
        if let Some(MethodQualifier::Const) = self.method_qualifier {
            if matches!(kind, BorrowKind::Mutable) {
                return Err(format!(
                    "Cannot create mutable borrow of field '{}' in const method",
                    field
                ));
            }
        }

        // Check for conflicting borrows
        if let Some(existing_kind) = self.borrowed_fields.get(field) {
            match (existing_kind, kind) {
                (BorrowKind::Mutable, _) => {
                    return Err(format!(
                        "Cannot borrow field '{}': already borrowed mutably",
                        field
                    ));
                }
                (BorrowKind::Immutable, BorrowKind::Mutable) => {
                    return Err(format!(
                        "Cannot borrow field '{}' mutably: already borrowed immutably",
                        field
                    ));
                }
                (BorrowKind::Immutable, BorrowKind::Immutable) => {
                    // Multiple immutable borrows are OK
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Mark a field as moved
    pub fn mark_field_moved(&mut self, field: String) {
        self.moved_fields.insert(field.clone());
        self.borrowed_fields.remove(&field); // Can't be borrowed if moved
    }

    /// Mark a field as borrowed
    pub fn mark_field_borrowed(&mut self, field: String, kind: BorrowKind) {
        self.borrowed_fields.insert(field, kind);
    }
}
