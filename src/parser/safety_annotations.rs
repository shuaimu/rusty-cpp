use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};
use clang::Entity;
use crate::debug_println;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SafetyMode {
    Safe,       // Enforce borrow checking, strict call rules
    Unsafe,     // Skip borrow checking, explicitly marked as unsafe
    Undeclared, // Not explicitly marked - treated as unsafe but safe functions cannot call them
}

#[derive(Debug, Clone)]
pub struct SafetyContext {
    pub file_default: SafetyMode,
    pub function_overrides: Vec<(String, SafetyMode)>, // Function name -> safety mode
}


impl SafetyContext {
    pub fn new() -> Self {
        Self {
            file_default: SafetyMode::Undeclared,
            function_overrides: Vec::new(),
        }
    }
    
    /// Merge safety annotations from headers into this context
    pub fn merge_header_annotations(&mut self, header_cache: &super::header_cache::HeaderCache) {
        // For each function that has a safety annotation in a header,
        // add it to our overrides if not already present
        for (func_name, &safety_mode) in header_cache.safety_annotations.iter() {
            // Check if we already have an override for this function
            // Need to check both exact match and qualified/unqualified variations
            let already_has_override = self.function_overrides.iter()
                .any(|(name, _)| {
                    name == func_name || 
                    name.ends_with(&format!("::{}", func_name)) || 
                    func_name.ends_with(&format!("::{}", name))
                });
            
            if !already_has_override {
                // Add the header's safety annotation
                debug_println!("DEBUG SAFETY: Adding header annotation for '{}': {:?}", func_name, safety_mode);
                self.function_overrides.push((func_name.clone(), safety_mode));
            } else {
                debug_println!("DEBUG SAFETY: Function '{}' already has annotation, keeping source file version", func_name);
            }
            // If we already have an override from the source file, it takes precedence
        }
    }
    
    /// Check if a specific function should be checked
    pub fn should_check_function(&self, func_name: &str) -> bool {
        self.get_function_safety(func_name) == SafetyMode::Safe
    }
    
    /// Get the safety mode of a specific function
    pub fn get_function_safety(&self, func_name: &str) -> SafetyMode {
        // First check for exact match with function-specific override
        for (name, mode) in &self.function_overrides {
            if name == func_name {
                return *mode;
            }
            
            // Also check if one is a suffix of the other (for namespace::Class::method matching)
            // This handles cases where header has "rrr::Timer::start" and impl has "Timer::start"
            if name.ends_with(&format!("::{}", func_name)) || func_name.ends_with(&format!("::{}", name)) {
                return *mode;
            }
        }
        
        // Fall back to file default
        self.file_default
    }
}

/// Parse safety annotations from a C++ file using the unified rule:
/// @safe/@unsafe attaches to the next statement/block/function/namespace
pub fn parse_safety_annotations(path: &Path) -> Result<SafetyContext, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open file for safety parsing: {}", e))?;
    
    let reader = BufReader::new(file);
    let mut context = SafetyContext::new();
    let mut pending_annotation: Option<SafetyMode> = None;
    let mut in_comment_block = false;
    let mut _current_line = 0;
    
    let mut accumulated_line = String::new();
    let mut accumulating_for_annotation = false;
    
    for line_result in reader.lines() {
        _current_line += 1;
        let line = line_result.map_err(|e| format!("Failed to read line: {}", e))?;
        let trimmed = line.trim();
        
        // Handle multi-line comments
        if in_comment_block {
            if trimmed.contains("*/") {
                in_comment_block = false;
            }
            // Check for annotations in multi-line comments (must be on their own)
            let cleaned = trimmed.trim_start_matches('*').trim();
            if cleaned == "@safe" {
                pending_annotation = Some(SafetyMode::Safe);
            } else if cleaned == "@unsafe" {
                pending_annotation = Some(SafetyMode::Unsafe);
            }
            continue;
        }
        
        // Check for comment start
        if trimmed.starts_with("/*") {
            in_comment_block = true;
            // Check if it's a single-line /* @safe */ or /* @unsafe */ comment
            if let Some(end_pos) = trimmed.find("*/") {
                let comment_content = trimmed[2..end_pos].trim();
                if comment_content == "@safe" {
                    pending_annotation = Some(SafetyMode::Safe);
                } else if comment_content == "@unsafe" {
                    pending_annotation = Some(SafetyMode::Unsafe);
                }
                in_comment_block = false;
            }
            continue;
        }
        
        // Check single-line comments
        if trimmed.starts_with("//") {
            // Only look for annotations that are word boundaries (not part of other text)
            let comment_text = trimmed[2..].trim();
            if comment_text == "@safe" || comment_text.starts_with("@safe ") {
                pending_annotation = Some(SafetyMode::Safe);
            } else if comment_text == "@unsafe" || comment_text.starts_with("@unsafe ") {
                pending_annotation = Some(SafetyMode::Unsafe);
            }
            continue;
        }
        
        // Skip empty lines and preprocessor directives
        if trimmed.is_empty() || trimmed.starts_with("#") {
            continue;
        }
        
        // If we have a pending annotation, start accumulating
        if pending_annotation.is_some() && !accumulating_for_annotation {
            accumulated_line.clear();
            accumulating_for_annotation = true;
        }
        
        // Only accumulate if we're looking for annotation target
        if accumulating_for_annotation {
            if !accumulated_line.is_empty() {
                accumulated_line.push(' ');
            }
            accumulated_line.push_str(trimmed);
            
            // Check if we have a complete function declaration (has parentheses)
            let should_check_annotation = accumulated_line.contains('(') && 
                                         (accumulated_line.contains(')') || accumulated_line.contains('{'));
            
            // If we have a pending annotation and a complete declaration, apply it
            if should_check_annotation {
                if let Some(annotation) = pending_annotation.take() {
                    debug_println!("DEBUG SAFETY: Applying {:?} annotation to: {}", annotation, &accumulated_line);
                    // Check what kind of code element follows
                    if accumulated_line.starts_with("namespace") || 
                       (accumulated_line.contains("namespace") && !accumulated_line.contains("using")) {
                        // Namespace declaration - applies to whole namespace contents
                        context.file_default = annotation;
                        debug_println!("DEBUG SAFETY: Set file default to {:?} (namespace)", annotation);
                    } else if is_function_declaration(&accumulated_line) {
                        // Function declaration - extract function name and apply ONLY to this function
                        if let Some(func_name) = extract_function_name(&accumulated_line) {
                            context.function_overrides.push((func_name.clone(), annotation));
                            debug_println!("DEBUG SAFETY: Set function '{}' to {:?}", func_name, annotation);
                        }
                    } else {
                        // Any other code - annotation was consumed but doesn't apply to whole file
                        // It only applied to this single statement/declaration
                        debug_println!("DEBUG SAFETY: Annotation consumed by single statement: {}", &accumulated_line);
                    }
                    accumulated_line.clear();
                    accumulating_for_annotation = false;
                }
            }
        }
    }
    
    Ok(context)
}

/// Check if a line looks like a function declaration
fn is_function_declaration(line: &str) -> bool {
    // Simple heuristic - contains parentheses and common return types
    // This is simplified and could be improved
    let has_parens = line.contains('(') && line.contains(')');
    let has_type = line.contains("void") || line.contains("int") || 
                   line.contains("bool") || line.contains("auto") ||
                   line.contains("const") || line.contains("static");
    
    has_parens && (has_type || line.contains("::"))
}

/// Extract function name from a declaration line (including qualified names)
fn extract_function_name(line: &str) -> Option<String> {
    // Find the function name before the opening parenthesis
    if let Some(paren_pos) = line.find('(') {
        let before_paren = &line[..paren_pos];
        // Split by whitespace and get the last identifier (which may be qualified)
        let parts: Vec<&str> = before_paren.split_whitespace().collect();
        if let Some(last) = parts.last() {
            // Remove any qualifiers like * or & but keep :: for qualified names
            let name = last.trim_start_matches('*').trim_start_matches('&');
            if !name.is_empty() {
                // Check if THIS part (the last token) contains "::"
                // If it does, it's a qualified method name like "MyClass::myMethod"
                // If it doesn't but the line has "::", the "::" is in the return type
                if name.contains("::") {
                    // This is a qualified method name (e.g., "MyClass::myMethod")
                    return Some(name.to_string());
                }
                // Otherwise, return the simple function name
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Extract qualified function name (e.g., "MyClass::myMethod") from a declaration
fn extract_qualified_function_name(before_paren: &str) -> Option<String> {
    // Look for the pattern "ClassName::methodName" 
    // This could be preceded by return type and other qualifiers
    let parts: Vec<&str> = before_paren.split_whitespace().collect();
    
    for part in parts.iter().rev() {
        if part.contains("::") {
            // This is likely our qualified name
            let clean_name = part.trim_start_matches('*').trim_start_matches('&');
            return Some(clean_name.to_string());
        }
    }
    
    None
}

/// Parse safety annotation from entity comment (for clang AST)
#[allow(dead_code)]
pub fn parse_entity_safety(entity: &Entity) -> Option<SafetyMode> {
    if let Some(comment) = entity.get_comment() {
        if comment.contains("@safe") {
            Some(SafetyMode::Safe)
        } else if comment.contains("@unsafe") {
            Some(SafetyMode::Unsafe)
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_namespace_safe_annotation() {
        let code = r#"
// @safe
namespace myapp {
    void func1() {}
    void func2() {}
}
"#;
        
        let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
        file.write_all(code.as_bytes()).unwrap();
        file.flush().unwrap();
        
        let context = parse_safety_annotations(file.path()).unwrap();
        assert_eq!(context.file_default, SafetyMode::Safe);
    }
    
    #[test]
    fn test_function_safe_annotation() {
        let code = r#"
// Default is unsafe
void unsafe_func() {}

// @safe
void safe_func() {
    int x = 42;
}

// @unsafe
void explicit_unsafe() {}
"#;
        
        let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
        file.write_all(code.as_bytes()).unwrap();
        file.flush().unwrap();
        
        let context = parse_safety_annotations(file.path()).unwrap();
        
        assert!(!context.should_check_function("unsafe_func"));
        assert!(context.should_check_function("safe_func"));
        assert!(!context.should_check_function("explicit_unsafe"));
    }
    
    #[test]
    fn test_first_code_element_annotation() {
        let code = r#"
// @safe
int global_var = 42;

void func() {}
"#;
        
        let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
        file.write_all(code.as_bytes()).unwrap();
        file.flush().unwrap();
        
        let context = parse_safety_annotations(file.path()).unwrap();
        // @safe only applies to the next element (global_var), not the whole file
        assert_eq!(context.file_default, SafetyMode::Undeclared);
    }
}