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

/// Function signature for disambiguating overloaded functions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub name: String,
    pub param_types: Option<Vec<String>>,  // None means match by name only
}

impl FunctionSignature {
    fn new(name: String, param_types: Option<Vec<String>>) -> Self {
        Self { name, param_types }
    }

    fn from_name_only(name: String) -> Self {
        Self { name, param_types: None }
    }

    /// Check if this signature matches another (handles partial matches)
    fn matches(&self, other: &FunctionSignature) -> bool {
        // Names must match
        if self.name != other.name {
            return false;
        }

        // If either has no param types, match by name only
        match (&self.param_types, &other.param_types) {
            (None, _) | (_, None) => true,
            (Some(a), Some(b)) => a == b,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafetyContext {
    pub file_default: SafetyMode,
    pub function_overrides: Vec<(FunctionSignature, SafetyMode)>, // Function signature -> safety mode
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
                .any(|(sig, _)| {
                    sig.name == *func_name ||
                    sig.name.ends_with(&format!("::{}", func_name)) ||
                    func_name.ends_with(&format!("::{}", sig.name))
                });

            if !already_has_override {
                // Add the header's safety annotation (name only, no param types from header)
                debug_println!("DEBUG SAFETY: Adding header annotation for '{}': {:?}", func_name, safety_mode);
                let signature = FunctionSignature::from_name_only(func_name.clone());
                self.function_overrides.push((signature, safety_mode));
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
        let query = FunctionSignature::from_name_only(func_name.to_string());

        // First check for exact match with function-specific override
        for (sig, mode) in &self.function_overrides {
            if sig.matches(&query) {
                return *mode;
            }

            // Also check if one is a suffix of the other (for namespace::Class::method matching)
            // This handles cases where header has "rrr::Timer::start" and impl has "Timer::start"
            if sig.name.ends_with(&format!("::{}", func_name)) || func_name.ends_with(&format!("::{}", sig.name)) {
                return *mode;
            }
        }

        // If the function is a method (contains "::"), check if the class is annotated
        // E.g., for "rrr::Alarm::add", check if "rrr::Alarm" or "Alarm" is annotated
        if func_name.contains("::") {
            // Try to extract the class name by removing the method name
            if let Some(last_colon) = func_name.rfind("::") {
                let class_name = &func_name[..last_colon];

                // Check if the class has an annotation
                let class_query = FunctionSignature::from_name_only(class_name.to_string());
                for (sig, mode) in &self.function_overrides {
                    if sig.matches(&class_query) {
                        return *mode;
                    }

                    // Also check suffix matching for the class
                    if sig.name.ends_with(&format!("::{}", class_name)) || class_name.ends_with(&format!("::{}", sig.name)) {
                        return *mode;
                    }
                }
            }
        }

        // Fall back to file default
        self.file_default
    }

    /// Get the safety mode of a specific class
    /// This is similar to get_function_safety but specifically handles class-level annotations
    pub fn get_class_safety(&self, class_name: &str) -> SafetyMode {
        let query = FunctionSignature::from_name_only(class_name.to_string());

        debug_println!("DEBUG SAFETY: Looking up class '{}'", class_name);
        debug_println!("DEBUG SAFETY: Stored overrides ({} total):", self.function_overrides.len());
        for (sig, mode) in &self.function_overrides {
            debug_println!("DEBUG SAFETY:   - '{}' -> {:?}", sig.name, mode);
        }

        // Check for exact match
        for (sig, mode) in &self.function_overrides {
            if sig.matches(&query) {
                debug_println!("DEBUG SAFETY: Exact match for class '{}' -> {:?}", class_name, mode);
                return *mode;
            }

            // Check suffix matching (handles namespace::Class vs Class)
            if sig.name.ends_with(&format!("::{}", class_name)) {
                debug_println!("DEBUG SAFETY: Suffix match for class '{}' (stored as '{}') -> {:?}", class_name, sig.name, mode);
                return *mode;
            }

            if class_name.ends_with(&format!("::{}", sig.name)) {
                debug_println!("DEBUG SAFETY: Prefix match for class '{}' (query has more qualifiers) -> {:?}", class_name, mode);
                return *mode;
            }
        }

        debug_println!("DEBUG SAFETY: No match for class '{}', using file default: {:?}", class_name, self.file_default);
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
            
            // Check if we have a complete declaration to apply annotation to
            // For namespaces: just needs to start with "namespace" and have opening brace
            // For functions: needs parentheses
            let is_namespace_decl = accumulated_line.starts_with("namespace") ||
                                   (accumulated_line.contains("namespace") && !accumulated_line.contains("using"));
            let should_check_annotation = if is_namespace_decl {
                accumulated_line.contains('{')
            } else {
                accumulated_line.contains('(') &&
                (accumulated_line.contains(')') || accumulated_line.contains('{'))
            };
            
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
                    } else if is_class_declaration(&accumulated_line) {
                        // Class/struct declaration - extract class name and store annotation
                        if let Some(class_name) = extract_class_name(&accumulated_line) {
                            let signature = FunctionSignature::from_name_only(class_name.clone());
                            context.function_overrides.push((signature, annotation));
                            debug_println!("DEBUG SAFETY: Set class '{}' to {:?}", class_name, annotation);
                        }
                    } else if is_function_declaration(&accumulated_line) {
                        // Function declaration - extract function signature (name + params) and apply ONLY to this function
                        if let Some(func_name) = extract_function_name(&accumulated_line) {
                            let param_types = extract_parameter_types(&accumulated_line);
                            let signature = FunctionSignature::new(func_name.clone(), param_types.clone());
                            context.function_overrides.push((signature, annotation));

                            if let Some(ref params) = param_types {
                                debug_println!("DEBUG SAFETY: Set function '{}({})' to {:?}",
                                             func_name, params.join(", "), annotation);
                            } else {
                                debug_println!("DEBUG SAFETY: Set function '{}' to {:?}", func_name, annotation);
                            }
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

/// Check if a line looks like a class/struct declaration
fn is_class_declaration(line: &str) -> bool {
    // Check if line contains class/struct keyword (at start or with space before)
    let has_class = line.starts_with("class ") || line.starts_with("struct ") ||
                    line.contains(" class ") || line.contains(" struct ");
    // Check if line contains opening brace (may be after newlines in accumulated_line)
    let has_brace = line.contains('{');
    has_class && has_brace
}

/// Extract class name from a class/struct declaration
fn extract_class_name(line: &str) -> Option<String> {
    // Look for "class ClassName" or "struct StructName"
    // Handle multi-line declarations by replacing newlines with spaces
    let normalized = line.replace('\n', " ").replace('\r', " ");

    // Try to find "class " or "struct " - prioritize start of line to avoid matching "friend class"
    // Check patterns in priority order: start first, then middle
    let class_patterns = [
        ("class ", "class "),      // "class " at the start (highest priority)
        ("struct ", "struct "),    // "struct " at the start
        (" class ", " class "),    // " class " in the middle (lower priority)
        (" struct ", " struct "),  // " struct " in the middle
    ];

    for (search_pattern, keyword) in &class_patterns {
        if let Some(pos) = normalized.find(search_pattern) {
            let after_keyword = &normalized[pos + keyword.len()..];
            // Class name is the first word after "class" or "struct"
            let parts: Vec<&str> = after_keyword.split_whitespace().collect();
            if let Some(name) = parts.first() {
                // Remove any template parameters or inheritance markers
                let name = name.split('<').next().unwrap_or(name);
                let name = name.split(':').next().unwrap_or(name);
                let name = name.split('{').next().unwrap_or(name);
                // Sanity check: the extracted name shouldn't be "rusty" (that's from "friend class rusty::Arc")
                // This is a workaround for accumulated_line containing the full class body
                if name != "rusty" && name != "Arc" && name != "std" && !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// Check if a line looks like a function declaration
fn is_function_declaration(line: &str) -> bool {
    // First check if it's a template class/struct declaration (NOT a function)
    // Pattern: "template<...> class ..." or "template<...> struct ..."
    // We need to check if "class" or "struct" appears AFTER the template parameters (after '>')
    if line.starts_with("template") || line.contains(" template") {
        // Find the position of "template<" and match its closing '>'
        if let Some(template_pos) = line.find("template<") {
            // Find the matching '>' for "template<"
            let mut depth = 0;
            let mut template_end = None;
            for (i, ch) in line[template_pos..].chars().enumerate() {
                if ch == '<' {
                    depth += 1;
                } else if ch == '>' {
                    depth -= 1;
                    if depth == 0 {
                        template_end = Some(template_pos + i);
                        break;
                    }
                }
            }

            // Check what comes after the template parameters
            if let Some(end_pos) = template_end {
                let after_template = &line[end_pos + 1..].trim_start();
                // Only return false if "class " or "struct " appears right after template params
                if after_template.starts_with("class ") || after_template.starts_with("struct ") {
                    return false;
                }
            }
        }
    }

    // Simple heuristic - contains parentheses and common return types
    // This is simplified and could be improved
    let has_parens = line.contains('(') && line.contains(')');
    let has_type = line.contains("void") || line.contains("int") ||
                   line.contains("bool") || line.contains("auto") ||
                   line.contains("const") || line.contains("static");

    // Also recognize template functions: they start with a template parameter like "T " or "U "
    // or contain template syntax
    let is_template_function = {
        // Check if line starts with a single capital letter followed by space (template param)
        let trimmed = line.trim_start();
        let starts_with_template_param = trimmed.len() >= 2 &&
            trimmed.chars().next().map_or(false, |c| c.is_uppercase()) &&
            trimmed.chars().nth(1) == Some(' ');

        // Or contains template-related keywords/syntax
        let has_template_syntax = line.contains("template") || line.contains('<') || line.contains('>');

        starts_with_template_param || (has_template_syntax && has_parens)
    };

    has_parens && (has_type || line.contains("::") || is_template_function)
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

/// Extract parameter types from a function declaration line
/// Returns None if parameters cannot be extracted, Some(Vec) otherwise
fn extract_parameter_types(line: &str) -> Option<Vec<String>> {
    // Find the opening and closing parentheses
    let open_paren = line.find('(')?;

    // Find the matching closing parenthesis
    let mut depth = 0;
    let mut close_paren = None;
    for (i, ch) in line[open_paren..].chars().enumerate() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth -= 1;
            if depth == 0 {
                close_paren = Some(open_paren + i);
                break;
            }
        }
    }

    let close_paren = close_paren?;
    let params_str = &line[open_paren + 1..close_paren].trim();

    // Empty parameter list
    if params_str.is_empty() {
        return Some(Vec::new());
    }

    // Split parameters by comma (handling nested templates and parentheses)
    let mut params = Vec::new();
    let mut current_param = String::new();
    let mut angle_depth = 0;
    let mut paren_depth = 0;

    for ch in params_str.chars() {
        match ch {
            '<' => {
                angle_depth += 1;
                current_param.push(ch);
            }
            '>' => {
                angle_depth -= 1;
                current_param.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current_param.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current_param.push(ch);
            }
            ',' if angle_depth == 0 && paren_depth == 0 => {
                // Found a parameter separator
                if !current_param.trim().is_empty() {
                    params.push(normalize_param_type(&current_param));
                }
                current_param.clear();
            }
            _ => {
                current_param.push(ch);
            }
        }
    }

    // Don't forget the last parameter
    if !current_param.trim().is_empty() {
        params.push(normalize_param_type(&current_param));
    }

    Some(params)
}

/// Normalize a parameter type for comparison
/// Removes parameter names, extra whitespace, and standardizes formatting
fn normalize_param_type(param: &str) -> String {
    let trimmed = param.trim();

    // Remove default values (everything after '=')
    let without_default = if let Some(eq_pos) = trimmed.find('=') {
        &trimmed[..eq_pos]
    } else {
        trimmed
    };

    // Split by whitespace to handle "const Type&" or "Type *" etc.
    let tokens: Vec<&str> = without_default.split_whitespace().collect();

    if tokens.is_empty() {
        return String::new();
    }

    // Find the type part (everything except the last token if it looks like a variable name)
    // Heuristic: if last token has no special chars and previous token has <, >, ::, *, or &,
    // it's probably a variable name
    let type_tokens = if tokens.len() > 1 {
        let last = tokens.last().unwrap();
        let second_last = tokens[tokens.len() - 2];

        // If last token looks like a variable name (no special chars) and
        // second-to-last has type characters, drop the last token
        if !last.contains('<') && !last.contains('>') && !last.contains("::") &&
           !last.contains('*') && !last.contains('&') &&
           (second_last.contains('<') || second_last.contains('>') ||
            second_last.contains("::") || second_last.contains('*') ||
            second_last.contains('&')) {
            &tokens[..tokens.len() - 1]
        } else {
            &tokens[..]
        }
    } else {
        &tokens[..]
    };

    // Join tokens with single space
    type_tokens.join(" ")
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