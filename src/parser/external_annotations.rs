// External annotations parser - handles safety and lifetime annotations
// for third-party functions that can't be modified
//
// External functions can be marked as:
// - [safe] - programmer has audited the function and confirmed it follows safety rules
//           (e.g., std::string::length() is safe - no UB, no raw pointers exposed)
// - [unsafe] - function may have unsafe behavior, must be called from @unsafe block
//
// NOTE: The distinction is about programmer audit, not tool verification.
// [safe] external functions can be called directly from @safe code.

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use regex::Regex;

/// Safety level for external functions.
/// - Safe: Programmer has audited and confirmed the function follows safety rules.
///         Can be called directly from @safe code without @unsafe block.
/// - Unsafe: Function may have unsafe behavior. Must be called from @unsafe context.
#[derive(Debug, Clone, PartialEq)]
pub enum ExternalSafety {
    Safe,    // Programmer audited, safe to call from @safe code
    Unsafe,  // Must be called from @unsafe block
}

#[derive(Debug, Clone)]
pub struct ExternalFunctionAnnotation {
    pub name: String,
    pub safety: ExternalSafety,
    pub lifetime_spec: Option<String>, // Raw lifetime specification for future use
    pub param_lifetimes: Vec<String>,  // Parameter lifetime annotations
    pub return_lifetime: Option<String>, // Return type lifetime
    pub lifetime_constraints: Vec<String>, // Where clauses
}

#[derive(Debug, Clone)]
pub struct ExternalProfile {
    pub name: String,
    pub safe_patterns: Vec<String>,
    pub unsafe_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalAnnotations {
    // Explicit function annotations
    pub functions: HashMap<String, ExternalFunctionAnnotation>,

    // Pattern-based whitelists and blacklists
    pub whitelist_patterns: Vec<String>,
    pub blacklist_patterns: Vec<String>,

    // Named profiles for different libraries
    pub profiles: HashMap<String, ExternalProfile>,

    // Currently active profile
    pub active_profile: Option<String>,

    // Unsafe scopes (classes/namespaces marked as entirely unsafe)
    pub unsafe_scopes: Vec<String>,

    // Unsafe types - types whose internal structure should not be analyzed
    // A @safe class can have unsafe_type fields without triggering internal analysis
    pub unsafe_types: Vec<String>,
}

impl ExternalAnnotations {
    pub fn new() -> Self {
        let mut annotations = ExternalAnnotations {
            functions: HashMap::new(),
            whitelist_patterns: Vec::new(),
            blacklist_patterns: Vec::new(),
            profiles: HashMap::new(),
            active_profile: None,
            unsafe_scopes: Vec::new(),
            unsafe_types: Vec::new(),
        };

        // Load default annotations
        annotations.load_defaults();
        annotations
    }
    
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read external annotations: {}", e))?;
        
        let mut annotations = Self::new();
        annotations.parse_content(&content)?;
        Ok(annotations)
    }
    
    fn extract_from_comments(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_comment_block = false;
        let mut comment_content = String::new();
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            // Check for C++ comment with @external
            if trimmed.starts_with("//") {
                let comment = &trimmed[2..].trim();
                if comment.starts_with("@external:") || in_comment_block {
                    // Remove the // prefix and add to result
                    result.push_str(comment);
                    result.push('\n');
                    
                    // Track if we're in a multi-line block
                    if comment.contains('{') {
                        in_comment_block = true;
                    }
                    if comment.contains('}') {
                        in_comment_block = false;
                    }
                }
            }
            // Also handle C-style comments
            else if trimmed.starts_with("/*") {
                // Extract content between /* and */
                if let Some(end) = trimmed.find("*/") {
                    let comment = &trimmed[2..end].trim();
                    if comment.starts_with("@external:") {
                        result.push_str(comment);
                        result.push('\n');
                    }
                }
            }
            // If not in comment, still include non-comment @external blocks
            else if !in_comment_block && trimmed.starts_with("@external:") {
                result.push_str(line);
                result.push('\n');
                if trimmed.contains('{') {
                    in_comment_block = true;
                }
            }
            else if in_comment_block && !trimmed.starts_with("//") {
                // We've left the comment block
                in_comment_block = false;
            }
        }
        
        // If no annotations found in comments, return original content
        if result.is_empty() {
            content.to_string()
        } else {
            result
        }
    }
    
    pub fn parse_content(&mut self, content: &str) -> Result<(), String> {
        // First, try to extract annotations from C++ comments
        let processed_content = self.extract_from_comments(content);
        
        // Parse unified @external blocks (primary syntax)
        self.parse_unified_blocks(&processed_content)?;
        
        // Parse @external_function blocks (detailed syntax)
        self.parse_external_function_blocks(&processed_content)?;
        
        // Parse @external_unsafe for classes/namespaces
        self.parse_unsafe_scopes(&processed_content)?;
        
        // Parse @external_whitelist
        self.parse_whitelist(&processed_content)?;
        
        // Parse @external_blacklist
        self.parse_blacklist(&processed_content)?;
        
        // Parse @external_profile blocks
        self.parse_profiles(&processed_content)?;
        
        Ok(())
    }
    
    fn parse_unified_blocks(&mut self, content: &str) -> Result<(), String> {
        // Parse @external: { func: [safety, lifetime] } blocks
        let unified_re = Regex::new(r"@external:\s*\{([^}]+)\}").unwrap();
        
        for cap in unified_re.captures_iter(content) {
            if let Some(block) = cap.get(1) {
                self.parse_unified_entries(block.as_str())?;
            }
        }
        
        Ok(())
    }
    
    fn parse_unified_entries(&mut self, block: &str) -> Result<(), String> {
        // First, split block into individual entries
        // Entries can be on separate lines OR separated by commas on the same line
        // But we need to be careful not to split inside brackets [...]
        let entries = self.split_entries(block);

        for entry in entries {
            let entry = entry.trim();
            if entry.is_empty() || entry.starts_with("//") {
                continue;
            }

            // Parse entries like: function_name: [safety, lifetime_spec]
            // or: type_name: [unsafe_type]
            // Note: function names can contain :: (e.g., rusty::Option::is_none)
            // So we look for ": [" which is the separator between name and spec
            if let Some(sep_pos) = entry.find(": [") {
                let name = entry[..sep_pos].trim().to_string();
                let spec_str = entry[sep_pos + 2..].trim();  // Skip ": " to get "[...]"

                // Parse [safety, lifetime] or [unsafe_type] array
                if spec_str.starts_with('[') && spec_str.ends_with(']') {
                    let inner = &spec_str[1..spec_str.len()-1];
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

                    if parts.len() >= 1 {
                        // Check for unsafe_type annotation
                        if parts[0] == "unsafe_type" {
                            self.unsafe_types.push(name);
                            continue;
                        }

                        let safety = match parts[0] {
                            "safe" => ExternalSafety::Safe,
                            "unsafe" => ExternalSafety::Unsafe,
                            _ => continue,
                        };

                        let lifetime_spec = if parts.len() >= 2 {
                            Some(parts[1..].join(","))
                        } else {
                            None
                        };

                        let (param_lifetimes, return_lifetime, constraints) =
                            if let Some(ref spec) = lifetime_spec {
                                self.parse_lifetime_specification(spec)
                            } else {
                                (Vec::new(), None, Vec::new())
                            };


                        self.functions.insert(name.clone(), ExternalFunctionAnnotation {
                            name,
                            safety,
                            lifetime_spec,
                            param_lifetimes,
                            return_lifetime,
                            lifetime_constraints: constraints,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Split a block into individual entries, handling:
    /// - Entries on separate lines
    /// - Entries separated by commas on the same line
    /// - Not splitting inside brackets [...]
    fn split_entries(&self, block: &str) -> Vec<String> {
        let mut entries = Vec::new();
        let mut current_entry = String::new();
        let mut bracket_depth = 0;

        for ch in block.chars() {
            match ch {
                '[' => {
                    bracket_depth += 1;
                    current_entry.push(ch);
                }
                ']' => {
                    bracket_depth -= 1;
                    current_entry.push(ch);
                    // If we've closed a bracket and we're at depth 0, this entry might be complete
                    if bracket_depth == 0 {
                        // Check if next non-whitespace char is comma or newline
                        // For now, just mark that we've completed a bracketed section
                    }
                }
                ',' if bracket_depth == 0 => {
                    // Entry separator (outside brackets)
                    let trimmed = current_entry.trim();
                    if !trimmed.is_empty() {
                        entries.push(trimmed.to_string());
                    }
                    current_entry.clear();
                }
                '\n' => {
                    // Newline can also be an entry separator
                    let trimmed = current_entry.trim();
                    if !trimmed.is_empty() && trimmed.contains(": [") && trimmed.contains(']') {
                        entries.push(trimmed.to_string());
                        current_entry.clear();
                    } else {
                        // Continue building current entry (might be multi-line)
                        current_entry.push(' ');
                    }
                }
                _ => {
                    current_entry.push(ch);
                }
            }
        }

        // Don't forget the last entry
        let trimmed = current_entry.trim();
        if !trimmed.is_empty() {
            entries.push(trimmed.to_string());
        }

        entries
    }

    fn parse_external_function_blocks(&mut self, content: &str) -> Result<(), String> {
        // Parse @external_function: name { safety: ..., lifetime: ..., where: ... }
        let func_re = Regex::new(r"@external_function:\s*(\w+)\s*\{([^}]+)\}").unwrap();

        for cap in func_re.captures_iter(content) {
            if let (Some(name), Some(block)) = (cap.get(1), cap.get(2)) {
                let func_name = name.as_str().to_string();
                let block_content = block.as_str();

                // Parse safety field - safe or unsafe
                let safety = if block_content.contains("safety: safe") {
                    ExternalSafety::Safe
                } else if block_content.contains("safety: unsafe") {
                    ExternalSafety::Unsafe
                } else {
                    // Default to unsafe (conservative choice for external code)
                    ExternalSafety::Unsafe
                };
                
                // Parse lifetime field
                let lifetime_re = Regex::new(r"lifetime:\s*([^\n]+)").unwrap();
                let lifetime_spec = lifetime_re.captures(block_content)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().trim().to_string());
                
                // Parse where field
                let where_re = Regex::new(r"where:\s*([^\n]+)").unwrap();
                let where_clause = where_re.captures(block_content)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().trim().to_string());
                
                let (param_lifetimes, return_lifetime, mut constraints) = 
                    if let Some(ref spec) = lifetime_spec {
                        self.parse_lifetime_specification(spec)
                    } else {
                        (Vec::new(), None, Vec::new())
                    };
                
                if let Some(where_str) = where_clause {
                    constraints.push(where_str);
                }
                
                self.functions.insert(func_name.clone(), ExternalFunctionAnnotation {
                    name: func_name,
                    safety,
                    lifetime_spec,
                    param_lifetimes,
                    return_lifetime,
                    lifetime_constraints: constraints,
                });
            }
        }
        
        Ok(())
    }
    
    fn parse_lifetime_specification(&self, spec: &str) -> (Vec<String>, Option<String>, Vec<String>) {
        let mut param_lifetimes = Vec::new();
        let mut return_lifetime = None;
        let mut constraints = Vec::new();
        
        // Split by "where" clause if present
        let parts: Vec<&str> = spec.split("where").collect();
        let main_spec = parts[0].trim();
        
        if parts.len() > 1 {
            constraints.push(parts[1].trim().to_string());
        }
        
        // Parse main specification (params) -> return
        if let Some(arrow_pos) = main_spec.find("->") {
            let params_part = main_spec[..arrow_pos].trim();
            let return_part = main_spec[arrow_pos + 2..].trim();
            
            // Parse parameters
            if params_part.starts_with('(') && params_part.ends_with(')') {
                let params_inner = &params_part[1..params_part.len()-1];
                for param in params_inner.split(',') {
                    param_lifetimes.push(param.trim().to_string());
                }
            }
            
            // Parse return type
            return_lifetime = Some(return_part.to_string());
        } else {
            // No parameters, just return type
            return_lifetime = Some(main_spec.to_string());
        }
        
        (param_lifetimes, return_lifetime, constraints)
    }
    
    fn parse_unsafe_scopes(&mut self, content: &str) -> Result<(), String> {
        // Parse @external_unsafe: namespace::* or @external_unsafe: class::*
        let unsafe_scope_re = Regex::new(r"@external_unsafe:\s*([^\s]+)").unwrap();
        
        for cap in unsafe_scope_re.captures_iter(content) {
            if let Some(scope) = cap.get(1) {
                self.unsafe_scopes.push(scope.as_str().to_string());
            }
        }
        
        // Also parse block syntax: @external_unsafe: { scopes: [...] }
        let unsafe_block_re = Regex::new(r"@external_unsafe:\s*\{[^}]*scopes:\s*\[([^\]]+)\]").unwrap();
        
        if let Some(cap) = unsafe_block_re.captures(content) {
            if let Some(scopes) = cap.get(1) {
                for scope in scopes.as_str().split(',') {
                    let scope = scope.trim().trim_matches('"').to_string();
                    if !scope.is_empty() {
                        self.unsafe_scopes.push(scope);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn parse_whitelist(&mut self, content: &str) -> Result<(), String> {
        let whitelist_re = Regex::new(r"@external_whitelist:\s*\{[^}]*patterns:\s*\[([^\]]+)\]").unwrap();
        
        if let Some(cap) = whitelist_re.captures(content) {
            if let Some(patterns) = cap.get(1) {
                for pattern in patterns.as_str().split(',') {
                    let pattern = pattern.trim().trim_matches('"').to_string();
                    if !pattern.is_empty() {
                        self.whitelist_patterns.push(pattern);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn parse_blacklist(&mut self, content: &str) -> Result<(), String> {
        let blacklist_re = Regex::new(r"@external_blacklist:\s*\{[^}]*patterns:\s*\[([^\]]+)\]").unwrap();
        
        if let Some(cap) = blacklist_re.captures(content) {
            if let Some(patterns) = cap.get(1) {
                for pattern in patterns.as_str().split(',') {
                    let pattern = pattern.trim().trim_matches('"').to_string();
                    if !pattern.is_empty() {
                        self.blacklist_patterns.push(pattern);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn parse_profiles(&mut self, content: &str) -> Result<(), String> {
        let profile_re = Regex::new(r"@external_profile:\s*(\w+)\s*\{([^}]+)\}").unwrap();
        
        for cap in profile_re.captures_iter(content) {
            if let (Some(name), Some(block)) = (cap.get(1), cap.get(2)) {
                let mut profile = ExternalProfile {
                    name: name.as_str().to_string(),
                    safe_patterns: Vec::new(),
                    unsafe_patterns: Vec::new(),
                };
                
                // Parse safe and unsafe patterns in the profile
                let safe_re = Regex::new(r"safe:\s*\[([^\]]+)\]").unwrap();
                let unsafe_re = Regex::new(r"unsafe:\s*\[([^\]]+)\]").unwrap();
                
                if let Some(safe_cap) = safe_re.captures(block.as_str()) {
                    if let Some(patterns) = safe_cap.get(1) {
                        for pattern in patterns.as_str().split(',') {
                            let pattern = pattern.trim().trim_matches('"').to_string();
                            if !pattern.is_empty() {
                                profile.safe_patterns.push(pattern);
                            }
                        }
                    }
                }
                
                if let Some(unsafe_cap) = unsafe_re.captures(block.as_str()) {
                    if let Some(patterns) = unsafe_cap.get(1) {
                        for pattern in patterns.as_str().split(',') {
                            let pattern = pattern.trim().trim_matches('"').to_string();
                            if !pattern.is_empty() {
                                profile.unsafe_patterns.push(pattern);
                            }
                        }
                    }
                }
                
                self.profiles.insert(profile.name.clone(), profile);
            }
        }
        
        Ok(())
    }
    
    fn load_defaults(&mut self) {
        // Load common C standard library functions (unsafe)
        self.add_c_stdlib_defaults();

        // Load default unsafe types - STL containers whose internal structure should not be analyzed
        // These types have internal classes with mutable fields that would trigger false positives
        self.add_stl_unsafe_types();

        // Blacklisted patterns - always unsafe
        self.blacklist_patterns.extend(vec![
            "*::operator new*".to_string(),
            "*::operator delete*".to_string(),
            "*::malloc".to_string(),
            "*::free".to_string(),
            "*::memcpy".to_string(),
            "*::memmove".to_string(),
        ]);
    }

    fn add_stl_unsafe_types(&mut self) {
        // STL containers and their internal classes have mutable fields (e.g., _ReuseOrAllocNode)
        // that would trigger false positives when analyzing @safe classes that use them.
        // Mark these as unsafe_type so their internal structure is not analyzed.
        self.unsafe_types.extend(vec![
            // Hash containers and their internals
            "std::unordered_map*".to_string(),
            "std::unordered_set*".to_string(),
            "std::unordered_multimap*".to_string(),
            "std::unordered_multiset*".to_string(),
            "_Hashtable*".to_string(),
            "_Hash_node*".to_string(),
            "_ReuseOrAllocNode*".to_string(),

            // Other STL containers with complex internals
            "std::map*".to_string(),
            "std::set*".to_string(),
            "std::multimap*".to_string(),
            "std::multiset*".to_string(),
            "std::list*".to_string(),
            "std::forward_list*".to_string(),
            "std::deque*".to_string(),

            // Smart pointers
            "std::shared_ptr*".to_string(),
            "std::weak_ptr*".to_string(),
            "std::unique_ptr*".to_string(),

            // Function wrappers
            "std::function*".to_string(),
            "std::move_only_function*".to_string(),

            // Other STL internals that may have mutable fields
            "_Rb_tree*".to_string(),
            "_List_node*".to_string(),
            "__shared_ptr*".to_string(),
            "__weak_ptr*".to_string(),
        ]);
    }
    
    fn add_c_stdlib_defaults(&mut self) {
        // All C standard library functions are marked [unsafe] because:
        // - They are external code not verified by RustyCpp
        // - Programmer takes responsibility for auditing their usage
        // - This is the correct semantic: unsafe = programmer-audited, safe = tool-verified

        // Common C I/O functions
        for func in &["printf", "fprintf", "snprintf", "puts", "fputs", "fgets",
                      "strcmp", "strncmp", "strlen", "atoi", "atof", "exit"] {
            self.functions.insert(func.to_string(), ExternalFunctionAnnotation {
                name: func.to_string(),
                safety: ExternalSafety::Unsafe,  // All external functions are unsafe
                lifetime_spec: None,
                param_lifetimes: Vec::new(),
                return_lifetime: None,
                lifetime_constraints: Vec::new(),
            });
        }

        // Memory management with lifetimes
        self.functions.insert("malloc".to_string(), ExternalFunctionAnnotation {
            name: "malloc".to_string(),
            safety: ExternalSafety::Unsafe,
            lifetime_spec: Some("(size_t) -> owned void*".to_string()),
            param_lifetimes: vec!["size_t".to_string()],
            return_lifetime: Some("owned void*".to_string()),
            lifetime_constraints: Vec::new(),
        });

        self.functions.insert("free".to_string(), ExternalFunctionAnnotation {
            name: "free".to_string(),
            safety: ExternalSafety::Unsafe,
            lifetime_spec: Some("(void*) -> void".to_string()),
            param_lifetimes: vec!["void*".to_string()],
            return_lifetime: Some("void".to_string()),
            lifetime_constraints: Vec::new(),
        });

        self.functions.insert("strcpy".to_string(), ExternalFunctionAnnotation {
            name: "strcpy".to_string(),
            safety: ExternalSafety::Unsafe,
            lifetime_spec: Some("(char* dest, const char* src) -> char* where dest: 'a, return: 'a".to_string()),
            param_lifetimes: vec!["char* dest".to_string(), "const char* src".to_string()],
            return_lifetime: Some("char*".to_string()),
            lifetime_constraints: vec!["dest: 'a, return: 'a".to_string()],
        });

        // Other memory/string functions
        for func in &["calloc", "realloc", "memcpy", "memmove",
                      "memset", "strcat", "sprintf", "gets"] {
            self.functions.insert(func.to_string(), ExternalFunctionAnnotation {
                name: func.to_string(),
                safety: ExternalSafety::Unsafe,
                lifetime_spec: None,
                param_lifetimes: Vec::new(),
                return_lifetime: None,
                lifetime_constraints: Vec::new(),
            });
        }
    }
    
    /// Check if a type is marked as unsafe_type (internal structure should not be analyzed)
    pub fn is_type_unsafe(&self, type_name: &str) -> bool {
        for pattern in &self.unsafe_types {
            if Self::matches_pattern(type_name, pattern) {
                return true;
            }
        }
        false
    }

    pub fn is_function_safe(&self, func_name: &str) -> Option<bool> {
        // First check if function is in an unsafe scope
        for scope in &self.unsafe_scopes {
            if Self::matches_pattern(func_name, scope) {
                return Some(false);  // Entire scope is unsafe
            }
        }

        // Then check explicit function annotations
        // Try exact match first
        if let Some(annotation) = self.functions.get(func_name) {
            return Some(annotation.safety == ExternalSafety::Safe);
        }

        // Try to match against stored qualified names
        // e.g., if func_name is "swap", check if any "xxx::swap" exists
        for (annotated_name, annotation) in &self.functions {
            // Check if annotated_name ends with "::func_name"
            if annotated_name.ends_with(&format!("::{}", func_name)) {
                return Some(annotation.safety == ExternalSafety::Safe);
            }
            // Also check if func_name is qualified and annotated_name is just the suffix
            if func_name.ends_with(&format!("::{}", annotated_name)) {
                return Some(annotation.safety == ExternalSafety::Safe);
            }
        }
        
        // Then check active profile
        if let Some(profile_name) = &self.active_profile {
            if let Some(profile) = self.profiles.get(profile_name) {
                #[cfg(test)]
                {
                    println!("Checking {} against profile {} with safe patterns: {:?}", 
                        func_name, profile_name, profile.safe_patterns);
                }
                // Check unsafe patterns first (they have higher priority)
                if Self::matches_any_pattern(func_name, &profile.unsafe_patterns) {
                    return Some(false);
                }
                if Self::matches_any_pattern(func_name, &profile.safe_patterns) {
                    return Some(true);
                }
            }
        }
        
        // Check blacklist (higher priority)
        if Self::matches_any_pattern(func_name, &self.blacklist_patterns) {
            return Some(false);
        }
        
        // Check whitelist
        if Self::matches_any_pattern(func_name, &self.whitelist_patterns) {
            return Some(true);
        }
        
        // No annotation found
        None
    }
    
    fn matches_any_pattern(name: &str, patterns: &[String]) -> bool {
        for pattern in patterns {
            #[cfg(test)]
            {
                println!("Checking {} against pattern: '{}'", name, pattern);
            }
            if Self::matches_pattern(name, pattern) {
                return true;
            }
        }
        false
    }
    
    fn matches_pattern(name: &str, pattern: &str) -> bool {
        // Simple glob-like pattern matching
        // * matches any sequence of characters
        // ? matches any single character
        
        if pattern == "*" {
            return true;
        }
        
        // Special case for patterns like "*::functionName"
        if pattern.starts_with("*::") && !pattern[3..].contains('*') {
            // Match any class/namespace prefix
            let suffix = &pattern[3..];
            return name.ends_with(&format!("::{}", suffix)) || 
                   name == suffix;
        }
        
        // Special case for patterns ending with * but no other wildcards
        if pattern.ends_with("*") && pattern.matches('*').count() == 1 {
            let prefix = &pattern[..pattern.len()-1];
            return name.starts_with(prefix);
        }
        
        // General wildcard patterns - use regex
        if pattern.contains('*') || pattern.contains('?') {
            // Convert glob pattern to regex
            // Note: order matters - replace literal chars before wildcards
            let regex_pattern = pattern
                .replace(".", r"\.")
                .replace("+", r"\+")
                .replace("(", r"\(")
                .replace(")", r"\)")
                .replace("[", r"\[")
                .replace("]", r"\]")
                .replace("^", r"\^")
                .replace("$", r"\$")
                .replace("*", ".*")
                .replace("?", ".");
            
            #[cfg(test)]
            {
                println!("Pattern '{}' converted to regex: ^{}$", pattern, regex_pattern);
            }
            
            if let Ok(re) = Regex::new(&format!("^{}$", regex_pattern)) {
                let result = re.is_match(name);
                #[cfg(test)]
                {
                    println!("Matching '{}' against pattern '{}': {}", name, pattern, result);
                }
                return result;
            } else {
                #[cfg(test)]
                {
                    println!("Failed to compile regex for pattern: {}", pattern);
                }
            }
        }
        
        // Handle case where pattern is unqualified but name is qualified
        // e.g., pattern "make_unique" should match name "std::make_unique"
        if !pattern.contains("::") && name.contains("::") {
            if name.ends_with(&format!("::{}", pattern)) {
                return true;
            }
        }

        name == pattern
    }
    
    pub fn set_active_profile(&mut self, profile_name: &str) -> Result<(), String> {
        if self.profiles.contains_key(profile_name) {
            self.active_profile = Some(profile_name.to_string());
            Ok(())
        } else {
            Err(format!("Profile '{}' not found", profile_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_qualified_name_matching() {
        // Test that unqualified names match qualified annotations
        let content = r#"
        // @external: {
        //   std::swap: [safe, (T& a, T& b) -> void]
        //   my_namespace::helper: [unsafe, () -> void]
        // }
        "#;

        let mut annotations = ExternalAnnotations::new();
        annotations.parse_content(content).unwrap();

        // Unqualified name should match qualified annotation
        assert_eq!(annotations.is_function_safe("swap"), Some(true));  // safe
        assert_eq!(annotations.is_function_safe("helper"), Some(false));  // unsafe
        // Qualified name should still work
        assert_eq!(annotations.is_function_safe("std::swap"), Some(true));  // safe
    }

    #[test]
    fn test_safe_vs_unsafe_annotation() {
        let content = r#"
        // @external: {
        //   std::string::length: [safe, (&self) -> size_t]
        //   std::string::c_str: [unsafe, (&self) -> const char*]
        // }
        "#;

        let mut annotations = ExternalAnnotations::new();
        annotations.parse_content(content).unwrap();

        // Safe functions return Some(true)
        assert_eq!(annotations.is_function_safe("std::string::length"), Some(true));
        assert_eq!(annotations.is_function_safe("length"), Some(true));

        // Unsafe functions return Some(false)
        assert_eq!(annotations.is_function_safe("std::string::c_str"), Some(false));
        assert_eq!(annotations.is_function_safe("c_str"), Some(false));
    }

    #[test]
    fn test_parse_safety_block() {
        let content = r#"
        // @external: {
        //   malloc: [unsafe]
        //   printf: [unsafe]
        //   custom_func: [unsafe]
        // }
        "#;

        let mut annotations = ExternalAnnotations::new();
        annotations.parse_content(content).unwrap();

        // All external functions must be marked [unsafe]
        assert_eq!(annotations.is_function_safe("malloc"), Some(false));
        assert_eq!(annotations.is_function_safe("printf"), Some(false));
        assert_eq!(annotations.is_function_safe("custom_func"), Some(false));
    }
    
    #[test]
    fn test_pattern_matching() {
        let mut annotations = ExternalAnnotations::new();
        annotations.whitelist_patterns.push("std::*".to_string());
        annotations.blacklist_patterns.push("*::malloc".to_string());
        
        assert_eq!(annotations.is_function_safe("std::vector::push_back"), Some(true));
        assert_eq!(annotations.is_function_safe("custom::malloc"), Some(false));
        assert_eq!(annotations.is_function_safe("unknown_func"), None);
    }
    
    #[test]
    fn test_profiles() {
        let content = r#"
        // @external_profile: qt {
        //   safe: ["Q*::*", "qt::*"]
        //   unsafe: ["*::connect"]
        // }
        "#;
        
        let mut annotations = ExternalAnnotations::new();
        annotations.parse_content(content).unwrap();
        
        // Debug: check if profile was parsed
        println!("Profiles parsed: {:?}", annotations.profiles.keys().collect::<Vec<_>>());
        
        annotations.set_active_profile("qt").unwrap();
        
        // Debug: check pattern matching
        println!("Checking QWidget::show");
        let result1 = annotations.is_function_safe("QWidget::show");
        println!("Result: {:?}", result1);
        
        println!("Checking QObject::connect");
        let result2 = annotations.is_function_safe("QObject::connect");
        println!("Result: {:?}", result2);
        
        assert_eq!(result1, Some(true));
        assert_eq!(result2, Some(false));
    }
    
    #[test]
    fn test_wildcard_patterns() {
        assert!(ExternalAnnotations::matches_pattern("std::vector::size", "*::size"));
        assert!(ExternalAnnotations::matches_pattern("malloc", "malloc"));
        assert!(ExternalAnnotations::matches_pattern("my_malloc", "*malloc"));
        assert!(ExternalAnnotations::matches_pattern("malloc_wrapper", "malloc*"));
        assert!(!ExternalAnnotations::matches_pattern("free", "malloc"));
    }
    
    #[test]
    fn test_qt_pattern() {
        // Test the specific pattern that's failing
        let pattern = "Q*::*";
        let name = "QWidget::show";
        println!("Testing if '{}' matches pattern '{}'", name, pattern);
        assert!(ExternalAnnotations::matches_pattern(name, pattern),
            "Pattern '{}' should match '{}'", pattern, name);
    }

    #[test]
    fn test_unsafe_type_annotation() {
        let content = r#"
        // @external: {
        //   std::unordered_map: [unsafe_type]
        //   MyCustomContainer: [unsafe_type]
        // }
        "#;

        let mut annotations = ExternalAnnotations::new();
        annotations.parse_content(content).unwrap();

        // Check that the types are marked as unsafe
        assert!(annotations.is_type_unsafe("std::unordered_map"));
        assert!(annotations.is_type_unsafe("MyCustomContainer"));
        // Non-annotated type should not be unsafe (unless it matches default patterns)
        assert!(!annotations.is_type_unsafe("MyOtherClass"));
    }

    #[test]
    fn test_default_stl_unsafe_types() {
        let annotations = ExternalAnnotations::new();

        // STL containers should be marked as unsafe_type by default
        assert!(annotations.is_type_unsafe("std::unordered_map<int, int>"));
        assert!(annotations.is_type_unsafe("std::unordered_set<std::string>"));
        assert!(annotations.is_type_unsafe("_ReuseOrAllocNode"));
        assert!(annotations.is_type_unsafe("std::function<void()>"));

        // Regular user classes should not be unsafe
        assert!(!annotations.is_type_unsafe("MyClass"));
        assert!(!annotations.is_type_unsafe("UserDefinedMap"));
    }

    #[test]
    fn test_split_entries_multiline() {
        // Test that split_entries handles entries on separate lines
        let annotations = ExternalAnnotations::new();
        let block = r#"
            rusty::Option::is_none: [unsafe, (&self) -> bool]
            rusty::Option::is_some: [unsafe, (&self) -> bool]
        "#;

        let entries = annotations.split_entries(block);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("rusty::Option::is_none"));
        assert!(entries[1].contains("rusty::Option::is_some"));
    }

    #[test]
    fn test_split_entries_comma_separated() {
        // Test that split_entries handles comma-separated entries on the same line
        let annotations = ExternalAnnotations::new();
        let block = r#"foo: [unsafe, () -> void], bar: [unsafe, () -> int]"#;

        let entries = annotations.split_entries(block);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("foo"));
        assert!(entries[1].contains("bar"));
    }

    #[test]
    fn test_split_entries_preserves_brackets() {
        // Test that split_entries doesn't split inside brackets
        let annotations = ExternalAnnotations::new();
        let block = r#"func: [unsafe, (int, float) -> void]"#;

        let entries = annotations.split_entries(block);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].contains("(int, float)"));
    }

    #[test]
    fn test_qualified_function_name_parsing() {
        // Test that function names with :: are parsed correctly (not split on first :)
        let content = r#"
        // @external: {
        //   rusty::Option::is_none: [unsafe, (&self) -> bool]
        //   std::vector::push_back: [unsafe, (&mut self, T) -> void]
        // }
        "#;

        let mut annotations = ExternalAnnotations::new();
        annotations.parse_content(content).unwrap();

        // Check that the fully qualified names are stored correctly
        assert!(annotations.functions.contains_key("rusty::Option::is_none"));
        assert!(annotations.functions.contains_key("std::vector::push_back"));
    }
}
