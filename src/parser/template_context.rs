/// Template Context Tracking
///
/// This module tracks template type parameters (like T, U, V) to distinguish them
/// from regular function calls during safety checking.
///
/// When we see `T x = ...` in a template, `T` is a type parameter, not an undeclared
/// function. This module helps identify such cases.

use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    /// Type parameters in current template scope
    /// e.g., for `template<typename T, typename U>`, this would contain {"T", "U"}
    type_parameters: HashSet<String>,
}

impl TemplateContext {
    /// Create a new empty template context
    pub fn new() -> Self {
        Self {
            type_parameters: HashSet::new(),
        }
    }

    /// Enter a template scope with the given type parameters
    pub fn enter_template(&mut self, params: Vec<String>) {
        self.type_parameters.extend(params);
    }

    /// Exit the current template scope, clearing all type parameters
    pub fn exit_template(&mut self) {
        self.type_parameters.clear();
    }

    /// Check if a name is a template type parameter
    ///
    /// # Arguments
    /// * `name` - The identifier to check (e.g., "T", "U", "Value")
    ///
    /// # Returns
    /// `true` if `name` is a known type parameter in current scope
    pub fn is_type_parameter(&self, name: &str) -> bool {
        self.type_parameters.contains(name)
    }

    /// Get all type parameters in current scope
    pub fn get_type_parameters(&self) -> &HashSet<String> {
        &self.type_parameters
    }

    /// Check if we're currently in a template scope
    pub fn is_in_template(&self) -> bool {
        !self.type_parameters.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_context() {
        let ctx = TemplateContext::new();
        assert!(!ctx.is_type_parameter("T"));
        assert!(!ctx.is_in_template());
    }

    #[test]
    fn test_single_type_parameter() {
        let mut ctx = TemplateContext::new();
        ctx.enter_template(vec!["T".to_string()]);

        assert!(ctx.is_type_parameter("T"));
        assert!(!ctx.is_type_parameter("U"));
        assert!(ctx.is_in_template());
    }

    #[test]
    fn test_multiple_type_parameters() {
        let mut ctx = TemplateContext::new();
        ctx.enter_template(vec![
            "T".to_string(),
            "U".to_string(),
            "Value".to_string(),
        ]);

        assert!(ctx.is_type_parameter("T"));
        assert!(ctx.is_type_parameter("U"));
        assert!(ctx.is_type_parameter("Value"));
        assert!(!ctx.is_type_parameter("NotAParam"));
    }

    #[test]
    fn test_exit_template() {
        let mut ctx = TemplateContext::new();
        ctx.enter_template(vec!["T".to_string()]);
        assert!(ctx.is_type_parameter("T"));

        ctx.exit_template();
        assert!(!ctx.is_type_parameter("T"));
        assert!(!ctx.is_in_template());
    }

    #[test]
    fn test_get_type_parameters() {
        let mut ctx = TemplateContext::new();
        ctx.enter_template(vec!["T".to_string(), "U".to_string()]);

        let params = ctx.get_type_parameters();
        assert_eq!(params.len(), 2);
        assert!(params.contains("T"));
        assert!(params.contains("U"));
    }
}
