// External annotations parser - handles safety and lifetime annotations
// for third-party functions that can't be modified

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::fs;
use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum ExternalSafety {
    Safe,
    Unsafe,
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
    
    pub fn parse_content(&mut self, content: &str) -> Result<(), String> {
        // Parse unified @external blocks (primary syntax)
        self.parse_unified_blocks(content)?;
        
        // Parse @external_function blocks (detailed syntax)
        self.parse_external_function_blocks(content)?;
        
        // Parse @external_unsafe for classes/namespaces
        self.parse_unsafe_scopes(content)?;
        
        // Parse @external_whitelist
        self.parse_whitelist(content)?;
        
        // Parse @external_blacklist
        self.parse_blacklist(content)?;
        
        // Parse @external_profile blocks
        self.parse_profiles(content)?;
        
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
        for line in block.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            
            // Parse entries like: function_name: [safety, lifetime_spec]
            if let Some(colon_pos) = line.find(':') {
                let func_name = line[..colon_pos].trim().to_string();
                let spec_str = line[colon_pos + 1..].trim();
                
                // Parse [safety, lifetime] array
                if spec_str.starts_with('[') && spec_str.ends_with(']') {
                    let inner = &spec_str[1..spec_str.len()-1];
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    
                    if parts.len() >= 1 {
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
            }
        }
        
        Ok(())
    }
    
    fn parse_external_function_blocks(&mut self, content: &str) -> Result<(), String> {
        // Parse @external_function: name { safety: ..., lifetime: ..., where: ... }
        let func_re = Regex::new(r"@external_function:\s*(\w+)\s*\{([^}]+)\}").unwrap();
        
        for cap in func_re.captures_iter(content) {
            if let (Some(name), Some(block)) = (cap.get(1), cap.get(2)) {
                let func_name = name.as_str().to_string();
                let block_content = block.as_str();
                
                // Parse safety field
                let safety = if block_content.contains("safety: unsafe") {
                    ExternalSafety::Unsafe
                } else if block_content.contains("safety: safe") {
                    ExternalSafety::Safe
                } else {
                    ExternalSafety::Safe // Default to safe
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
        // Load common C standard library functions
        self.add_c_stdlib_defaults();
        
        // Load common patterns
        self.whitelist_patterns.extend(vec![
            "std::*".to_string(),
            "rusty::*".to_string(),
            "*::size".to_string(),
            "*::length".to_string(),
            "*::empty".to_string(),
        ]);
        
        self.blacklist_patterns.extend(vec![
            "*::operator new*".to_string(),
            "*::operator delete*".to_string(),
            "*::malloc".to_string(),
            "*::free".to_string(),
            "*::memcpy".to_string(),
            "*::memmove".to_string(),
        ]);
    }
    
    fn add_c_stdlib_defaults(&mut self) {
        // Safe C functions
        for func in &["printf", "fprintf", "snprintf", "puts", "fputs", "fgets", 
                      "strcmp", "strncmp", "strlen", "atoi", "atof", "exit"] {
            self.functions.insert(func.to_string(), ExternalFunctionAnnotation {
                name: func.to_string(),
                safety: ExternalSafety::Safe,
                lifetime_spec: None,
                param_lifetimes: Vec::new(),
                return_lifetime: None,
                lifetime_constraints: Vec::new(),
            });
        }
        
        // Unsafe C functions with lifetimes
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
        
        // Add other unsafe functions with simple defaults
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
    
    pub fn is_function_safe(&self, func_name: &str) -> Option<bool> {
        // First check if function is in an unsafe scope
        for scope in &self.unsafe_scopes {
            if Self::matches_pattern(func_name, scope) {
                return Some(false);  // Entire scope is unsafe
            }
        }
        
        // Then check explicit function annotations
        if let Some(annotation) = self.functions.get(func_name) {
            return Some(annotation.safety == ExternalSafety::Safe);
        }
        
        // Then check active profile
        if let Some(profile_name) = &self.active_profile {
            if let Some(profile) = self.profiles.get(profile_name) {
                if Self::matches_any_pattern(func_name, &profile.safe_patterns) {
                    return Some(true);
                }
                if Self::matches_any_pattern(func_name, &profile.unsafe_patterns) {
                    return Some(false);
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
        
        if pattern.starts_with("*::") {
            // Match any class/namespace prefix
            let suffix = &pattern[3..];
            return name.ends_with(&format!("::{}", suffix)) || 
                   name == suffix;
        }
        
        if pattern.ends_with("*") {
            let prefix = &pattern[..pattern.len()-1];
            return name.starts_with(prefix);
        }
        
        if pattern.contains('*') {
            // Convert glob pattern to regex
            let regex_pattern = pattern
                .replace(".", r"\.")
                .replace("*", ".*")
                .replace("?", ".");
            
            if let Ok(re) = Regex::new(&format!("^{}$", regex_pattern)) {
                return re.is_match(name);
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
    fn test_parse_safety_block() {
        let content = r#"
        // @external_safety: {
        //   malloc: unsafe
        //   printf: safe
        //   custom_func: safe
        // }
        "#;
        
        let mut annotations = ExternalAnnotations::new();
        annotations.parse_content(content).unwrap();
        
        assert_eq!(annotations.is_function_safe("malloc"), Some(false));
        assert_eq!(annotations.is_function_safe("printf"), Some(true));
        assert_eq!(annotations.is_function_safe("custom_func"), Some(true));
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
        annotations.set_active_profile("qt").unwrap();
        
        assert_eq!(annotations.is_function_safe("QWidget::show"), Some(true));
        assert_eq!(annotations.is_function_safe("QObject::connect"), Some(false));
    }
    
    #[test]
    fn test_wildcard_patterns() {
        assert!(ExternalAnnotations::matches_pattern("std::vector::size", "*::size"));
        assert!(ExternalAnnotations::matches_pattern("malloc", "malloc"));
        assert!(ExternalAnnotations::matches_pattern("my_malloc", "*malloc"));
        assert!(ExternalAnnotations::matches_pattern("malloc_wrapper", "malloc*"));
        assert!(!ExternalAnnotations::matches_pattern("free", "malloc"));
    }
}