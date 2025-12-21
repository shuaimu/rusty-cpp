use std::path::Path;
use std::fs::File;
use std::io::{BufRead, BufReader};
use clang::Entity;
use crate::debug_println;

/// Helper function to check if a string starts with a safety annotation
/// Accepts annotations with any suffix: @safe, @safe-XXX, @safe: note, etc.
/// But rejects partial matches like @safety or @safeguard
/// The annotation MUST be at the start of the text (already trimmed)
fn contains_annotation(text: &str, annotation: &str) -> bool {
    // The annotation must be at the start of the (already trimmed) text
    if !text.starts_with(annotation) {
        return false;
    }

    // Check what comes AFTER the annotation
    let after_annotation = annotation.len();
    if after_annotation >= text.len() {
        // End of string - exact match
        return true;
    }

    // Check the next character - it should NOT be alphanumeric
    // This prevents matching @safety when looking for @safe
    let next_char = text.chars().nth(after_annotation);
    if let Some(ch) = next_char {
        !ch.is_alphanumeric()
    } else {
        true
    }
}

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
    pub source_file: Option<String>, // The source file where annotations were parsed from
}


impl SafetyContext {
    pub fn new() -> Self {
        Self {
            file_default: SafetyMode::Undeclared,
            function_overrides: Vec::new(),
            source_file: None,
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

    /// Check if a file path is from the source file where annotations were parsed
    /// Returns true if the file path matches the source file, false otherwise
    pub fn is_from_source_file(&self, file_path: &str) -> bool {
        if let Some(ref source) = self.source_file {
            // Compare file paths - handle both absolute and relative paths
            // Check if either path ends with the other (to handle different path prefixes)
            file_path == source ||
            file_path.ends_with(source) ||
            source.ends_with(file_path) ||
            // Also check just the filename in case paths differ
            std::path::Path::new(file_path).file_name() == std::path::Path::new(source).file_name()
        } else {
            // No source file set - assume everything is from source (backward compatibility)
            true
        }
    }

    /// Get the safety mode of a specific function
    pub fn get_function_safety(&self, func_name: &str) -> SafetyMode {
        let query = FunctionSignature::from_name_only(func_name.to_string());

        // First check for exact match with function-specific override
        for (sig, mode) in &self.function_overrides {
            if sig.matches(&query) {
                return *mode;
            }

            let sig_is_qualified = sig.name.contains("::");
            let func_is_qualified = func_name.contains("::");

            // Bug #8 fix: Careful suffix matching to avoid namespace collisions
            // REMOVED Case 1: Qualified stored name, unqualified lookup - NO LONGER MATCH
            //         This was causing false positives: an unqualified "get" would incorrectly
            //         match "rusty::Cell::get" or any other qualified ::get annotation.
            //         e.g., stored "rusty::Cell::get", lookup "get" -> NO MATCH (could be any get)
            // Case 2: Both qualified - allow suffix matching on either side
            //         e.g., stored "rrr::Timer::start", lookup "Timer::start" -> MATCH
            // Case 3: Unqualified stored, qualified lookup - DON'T match (bug #8 scenario)
            //         e.g., stored "Node", lookup "yaml::Node" -> NO MATCH (different namespaces)
            // Note: If sig_is_qualified && !func_is_qualified, we DON'T match anymore.
            //       This is stricter but prevents false matches from unqualified external function calls.
            if sig_is_qualified && func_is_qualified {
                // Both are qualified - allow suffix matching on either side
                if sig.name.ends_with(&format!("::{}", func_name)) || func_name.ends_with(&format!("::{}", sig.name)) {
                    return *mode;
                }
            }
            // Note: if !sig_is_qualified && func_is_qualified, we DON'T match
            // This prevents "Node" from matching "yaml::Node" (bug #8)
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

                    // Bug #8 fix: Careful suffix matching
                    let sig_is_qualified = sig.name.contains("::");
                    let class_is_qualified = class_name.contains("::");

                    // Note: If sig_is_qualified && !class_is_qualified, we DON'T match anymore.
                    // This prevents an unqualified "Node" from matching "yaml::Node" annotation.
                    if sig_is_qualified && class_is_qualified {
                        if sig.name.ends_with(&format!("::{}", class_name)) || class_name.ends_with(&format!("::{}", sig.name)) {
                            return *mode;
                        }
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

            // Bug #8 fix: Careful suffix matching to avoid namespace collisions
            let sig_is_qualified = sig.name.contains("::");
            let class_is_qualified = class_name.contains("::");

            // REMOVED Case 1: Qualified stored, unqualified lookup - NO LONGER MATCH
            //         This was causing false positives: an unqualified "Node" would incorrectly
            //         match "rusty::Node" or any other qualified ::Node annotation.
            // Case 2: Both qualified - allow suffix matching on either side
            //         e.g., stored "rusty::Node", lookup "ns::rusty::Node" -> MATCH
            // Case 3: Unqualified stored, qualified lookup - DON'T match (bug #8)
            //         e.g., stored "Node", lookup "yaml::Node" -> NO MATCH
            // Note: If sig_is_qualified && !class_is_qualified, we DON'T match anymore.
            if sig_is_qualified && class_is_qualified {
                // Both are qualified - suffix matching is sound
                if sig.name.ends_with(&format!("::{}", class_name)) {
                    debug_println!("DEBUG SAFETY: Suffix match for class '{}' (stored as '{}') -> {:?}", class_name, sig.name, mode);
                    return *mode;
                }

                if class_name.ends_with(&format!("::{}", sig.name)) {
                    debug_println!("DEBUG SAFETY: Prefix match for class '{}' (query has more qualifiers) -> {:?}", class_name, mode);
                    return *mode;
                }
            }
            // Note: if !sig_is_qualified && class_is_qualified, we DON'T match (bug #8)
        }

        debug_println!("DEBUG SAFETY: No match for class '{}', using file default: {:?}", class_name, self.file_default);
        // Fall back to file default
        self.file_default
    }

    /// Get the safety mode of a class, considering its source file location
    ///
    /// IMPORTANT: file_default only applies to classes from the source file being analyzed.
    /// Classes from other files (system headers, external libraries) are treated as Undeclared
    /// unless they have an explicit annotation.
    ///
    /// This fixes the namespace collision bug where a user's @safe namespace annotation
    /// was incorrectly applying to STL classes from system headers.
    pub fn get_class_safety_for_file(&self, class_name: &str, class_file: &str) -> SafetyMode {
        let query = FunctionSignature::from_name_only(class_name.to_string());

        debug_println!("DEBUG SAFETY: Looking up class '{}' from file '{}'", class_name, class_file);

        // Check for explicit annotation (exact match or qualified match)
        for (sig, mode) in &self.function_overrides {
            if sig.matches(&query) {
                debug_println!("DEBUG SAFETY: Exact match for class '{}' -> {:?}", class_name, mode);
                return *mode;
            }

            let sig_is_qualified = sig.name.contains("::");
            let class_is_qualified = class_name.contains("::");

            if sig_is_qualified && class_is_qualified {
                if sig.name.ends_with(&format!("::{}", class_name)) {
                    debug_println!("DEBUG SAFETY: Suffix match for class '{}' -> {:?}", class_name, mode);
                    return *mode;
                }

                if class_name.ends_with(&format!("::{}", sig.name)) {
                    debug_println!("DEBUG SAFETY: Prefix match for class '{}' -> {:?}", class_name, mode);
                    return *mode;
                }
            }
        }

        // No explicit annotation found
        // Only apply file_default if the class is from the source file
        if self.is_from_source_file(class_file) {
            debug_println!("DEBUG SAFETY: Class '{}' is from source file, using file default: {:?}",
                class_name, self.file_default);
            self.file_default
        } else {
            // Class is from another file (header, system library, etc.)
            // Treat as Undeclared - user must explicitly annotate external types
            debug_println!("DEBUG SAFETY: Class '{}' is NOT from source file '{}', treating as Undeclared",
                class_name, class_file);
            SafetyMode::Undeclared
        }
    }

    /// Get the safety mode of a function, considering its source file location
    ///
    /// IMPORTANT: file_default only applies to functions from the source file being analyzed.
    /// Functions from other files are treated as Undeclared unless explicitly annotated.
    pub fn get_function_safety_for_file(&self, func_name: &str, func_file: &str) -> SafetyMode {
        let query = FunctionSignature::from_name_only(func_name.to_string());

        // Check for explicit function-specific override
        for (sig, mode) in &self.function_overrides {
            if sig.matches(&query) {
                return *mode;
            }

            let sig_is_qualified = sig.name.contains("::");
            let func_is_qualified = func_name.contains("::");

            if sig_is_qualified && func_is_qualified {
                if sig.name.ends_with(&format!("::{}", func_name)) || func_name.ends_with(&format!("::{}", sig.name)) {
                    return *mode;
                }
            }
        }

        // Check class-level annotation for methods
        if func_name.contains("::") {
            if let Some(last_colon) = func_name.rfind("::") {
                let class_name = &func_name[..last_colon];

                let class_query = FunctionSignature::from_name_only(class_name.to_string());
                for (sig, mode) in &self.function_overrides {
                    if sig.matches(&class_query) {
                        return *mode;
                    }

                    let sig_is_qualified = sig.name.contains("::");
                    let class_is_qualified = class_name.contains("::");

                    if sig_is_qualified && class_is_qualified {
                        if sig.name.ends_with(&format!("::{}", class_name)) || class_name.ends_with(&format!("::{}", sig.name)) {
                            return *mode;
                        }
                    }
                }
            }
        }

        // No explicit annotation found
        // Only apply file_default if the function is from the source file
        if self.is_from_source_file(func_file) {
            self.file_default
        } else {
            SafetyMode::Undeclared
        }
    }

    /// Check if a function should be checked, considering its source file location
    pub fn should_check_function_for_file(&self, func_name: &str, func_file: &str) -> bool {
        self.get_function_safety_for_file(func_name, func_file) == SafetyMode::Safe
    }
}

/// Parse safety annotations from a C++ file using the unified rule:
/// @safe/@unsafe attaches to the next statement/block/function/namespace
pub fn parse_safety_annotations(path: &Path) -> Result<SafetyContext, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open file for safety parsing: {}", e))?;

    let reader = BufReader::new(file);
    let mut context = SafetyContext::new();

    // Store the source file path for later reference
    // This is used to only apply file_default to code from this file
    context.source_file = path.to_str().map(|s| s.to_string());

    let mut pending_annotation: Option<SafetyMode> = None;
    let mut in_comment_block = false;
    let mut _current_line = 0;

    let mut accumulated_line = String::new();
    let mut accumulating_for_annotation = false;

    // Bug #8 fix: Track class context for method annotations
    let mut class_context_stack: Vec<String> = Vec::new();
    let mut brace_depth = 0;
    
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
            if contains_annotation(cleaned, "@safe") {
                pending_annotation = Some(SafetyMode::Safe);
            } else if contains_annotation(cleaned, "@unsafe") {
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
                if contains_annotation(comment_content, "@safe") {
                    pending_annotation = Some(SafetyMode::Safe);
                } else if contains_annotation(comment_content, "@unsafe") {
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
            if contains_annotation(comment_text, "@safe") {
                pending_annotation = Some(SafetyMode::Safe);
            } else if contains_annotation(comment_text, "@unsafe") {
                pending_annotation = Some(SafetyMode::Unsafe);
            }
            continue;
        }
        
        // Skip empty lines and preprocessor directives
        if trimmed.is_empty() || trimmed.starts_with("#") {
            continue;
        }

        // Bug #8 fix: Track braces to know when we exit a class
        // Note: This is a simplified tracking that doesn't handle strings/comments perfectly
        // but works for typical C++ code with annotations
        let opens = trimmed.matches('{').count() as i32;
        let closes = trimmed.matches('}').count() as i32;
        brace_depth += opens - closes;

        // Pop class context when we exit its scope
        if brace_depth <= 0 && !class_context_stack.is_empty() {
            class_context_stack.pop();
            brace_depth = 0; // Reset to handle nested classes properly
        }

        // Bug #8 fix: Track class declarations even without annotations
        // This ensures method annotations get qualified with class name
        // NOTE: Only push non-annotated classes here; annotated classes are pushed
        // in the annotation handling section below
        let is_class_line = is_class_declaration(trimmed);
        let needs_class_tracking = is_class_line && pending_annotation.is_none() && !accumulating_for_annotation;
        if needs_class_tracking {
            if let Some(class_name) = extract_class_name(trimmed) {
                // Calculate brace depth for this class declaration
                let class_brace_depth = trimmed.matches('{').count() as i32
                            - trimmed.matches('}').count() as i32;
                // Only push class to context if it's NOT complete on the same line
                // A class complete on one line (like `struct Foo { int x; };`) has brace_depth == 0
                // and should not be pushed to context stack
                if class_brace_depth > 0 {
                    class_context_stack.push(class_name);
                    brace_depth = class_brace_depth;
                }
            }
        }

        // Track namespace declarations (even without annotations) for qualified name building
        // This ensures function annotations inside namespaces get the proper qualified name
        let is_namespace_line = (trimmed.starts_with("namespace ") || trimmed.contains(" namespace "))
                                && !trimmed.contains("using ")
                                && trimmed.contains('{');
        let needs_namespace_tracking = is_namespace_line && pending_annotation.is_none() && !accumulating_for_annotation;
        if needs_namespace_tracking {
            if let Some(ns_name) = extract_namespace_name(trimmed) {
                debug_println!("DEBUG SAFETY: Entering namespace '{}' for context", ns_name);
                class_context_stack.push(ns_name);
                // Reset brace depth to track this namespace's scope
                brace_depth = trimmed.matches('{').count() as i32
                            - trimmed.matches('}').count() as i32;
            }
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
            // For classes: needs "class"/"struct" keyword and opening brace
            // For functions: needs parentheses
            let is_namespace_decl = accumulated_line.starts_with("namespace") ||
                                   (accumulated_line.contains("namespace") && !accumulated_line.contains("using"));
            let is_class_decl = is_class_declaration(&accumulated_line);
            let should_check_annotation = if is_namespace_decl || is_class_decl {
                accumulated_line.contains('{')
            } else {
                accumulated_line.contains('(') &&
                (accumulated_line.contains(')') || accumulated_line.contains('{'))
            };

            // CRITICAL FIX: Check if this is a forward declaration
            // Forward declarations (class Foo;) should consume the annotation without applying it
            // This prevents the annotation from carrying over to the next declaration
            let is_forward_decl = is_forward_declaration(&accumulated_line);

            if is_forward_decl && pending_annotation.is_some() {
                // Forward declarations should NOT have annotations (they have no body)
                // Consume the annotation without applying it to prevent it from affecting
                // subsequent declarations (especially the full class definition)
                debug_println!("DEBUG SAFETY: Ignoring annotation on forward declaration: {}",
                               &accumulated_line);
                pending_annotation.take();  // Consume the annotation
                accumulated_line.clear();
                accumulating_for_annotation = false;
                continue;  // Skip to next line
            }

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
                        // Also push namespace to context stack for qualifying nested function annotations
                        if let Some(ns_name) = extract_namespace_name(&accumulated_line) {
                            debug_println!("DEBUG SAFETY: Entering annotated namespace '{}' for context", ns_name);
                            class_context_stack.push(ns_name);
                            brace_depth = accumulated_line.matches('{').count() as i32
                                        - accumulated_line.matches('}').count() as i32;
                        }
                    } else if is_class_declaration(&accumulated_line) {
                        // Class/struct declaration - extract class name and store annotation
                        if let Some(class_name) = extract_class_name(&accumulated_line) {
                            // Bug #8 fix: Build qualified class name using context
                            let qualified_name = if class_context_stack.is_empty() {
                                class_name.clone()
                            } else {
                                format!("{}::{}", class_context_stack.join("::"), class_name)
                            };
                            let signature = FunctionSignature::from_name_only(qualified_name.clone());
                            context.function_overrides.push((signature, annotation));
                            debug_println!("DEBUG SAFETY: Set class '{}' to {:?}", qualified_name, annotation);

                            // Push class to context for nested methods
                            // Only push if the class is NOT complete on the same line
                            let class_brace_depth = accumulated_line.matches('{').count() as i32
                                        - accumulated_line.matches('}').count() as i32;
                            if class_brace_depth > 0 {
                                class_context_stack.push(class_name.clone());
                                brace_depth = class_brace_depth;
                            }
                        }
                    } else if is_function_declaration(&accumulated_line) {
                        // Function declaration - extract function signature (name + params) and apply ONLY to this function
                        if let Some(func_name) = extract_function_name(&accumulated_line) {
                            // Bug #8 fix: Build qualified function name using class context
                            let qualified_name = if class_context_stack.is_empty() {
                                func_name.clone()
                            } else {
                                format!("{}::{}", class_context_stack.join("::"), func_name)
                            };
                            let param_types = extract_parameter_types(&accumulated_line);
                            let signature = FunctionSignature::new(qualified_name.clone(), param_types.clone());
                            context.function_overrides.push((signature, annotation));

                            if let Some(ref params) = param_types {
                                debug_println!("DEBUG SAFETY: Set function '{}({})' to {:?}",
                                             qualified_name, params.join(", "), annotation);
                            } else {
                                debug_println!("DEBUG SAFETY: Set function '{}' to {:?}", qualified_name, annotation);
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

/// Check if a line is a forward declaration (class/struct with ; but no {)
/// Forward declarations should not have annotations applied to them
fn is_forward_declaration(line: &str) -> bool {
    let has_class_or_struct = line.starts_with("class ") || line.starts_with("struct ") ||
                              line.contains(" class ") || line.contains(" struct ");
    let has_semicolon = line.trim_end().ends_with(';');
    let has_brace = line.contains('{');

    // Must have class/struct keyword, must end with semicolon, must NOT have opening brace
    has_class_or_struct && has_semicolon && !has_brace
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

/// Extract namespace name from a namespace declaration
fn extract_namespace_name(line: &str) -> Option<String> {
    // Look for "namespace Name {"
    // Handle multi-line declarations by replacing newlines with spaces
    let normalized = line.replace('\n', " ").replace('\r', " ");

    // Find "namespace " keyword
    if let Some(pos) = normalized.find("namespace ") {
        let after_keyword = &normalized[pos + "namespace ".len()..];
        // Namespace name is the first word after "namespace"
        let parts: Vec<&str> = after_keyword.split_whitespace().collect();
        if let Some(name) = parts.first() {
            // Remove opening brace if attached
            let name = name.split('{').next().unwrap_or(name);
            if !name.is_empty() {
                return Some(name.to_string());
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
/// Bug fix: Only match @safe/@unsafe at the START of comment lines (or after prefix like //, /*, *)
/// This prevents false matches like "No @safe annotation" being treated as @safe
#[allow(dead_code)]
pub fn parse_entity_safety(entity: &Entity) -> Option<SafetyMode> {
    if let Some(comment) = entity.get_comment() {
        // Parse each line of the comment and check for annotations at the start
        for line in comment.lines() {
            let trimmed = line.trim();
            // Remove common comment prefixes
            let content = if trimmed.starts_with("///") {
                trimmed[3..].trim()
            } else if trimmed.starts_with("//") {
                trimmed[2..].trim()
            } else if trimmed.starts_with("/*") {
                trimmed[2..].trim()
            } else if trimmed.starts_with("*") {
                trimmed[1..].trim()
            } else {
                trimmed
            };

            // Use contains_annotation to properly check for annotations at start of line
            if contains_annotation(content, "@safe") {
                return Some(SafetyMode::Safe);
            } else if contains_annotation(content, "@unsafe") {
                return Some(SafetyMode::Unsafe);
            }
        }
        None
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