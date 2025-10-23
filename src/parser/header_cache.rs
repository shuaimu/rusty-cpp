use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;
use clang::{Clang, Index};
use crate::debug_println;

use super::annotations::{FunctionSignature, extract_annotations};
use super::safety_annotations::{SafetyMode, parse_entity_safety};
use super::external_annotations::ExternalAnnotations;

/// Cache for storing function signatures from header files
#[derive(Debug)]
pub struct HeaderCache {
    /// Map from function name to its lifetime signature
    signatures: HashMap<String, FunctionSignature>,
    /// Map from function name to its safety annotation from header
    pub safety_annotations: HashMap<String, SafetyMode>,
    /// Paths of headers that have been processed
    processed_headers: Vec<PathBuf>,
    /// Include paths to search for headers
    include_paths: Vec<PathBuf>,
    /// External annotations found in headers
    pub external_annotations: ExternalAnnotations,
}

impl HeaderCache {
    pub fn new() -> Self {
        Self {
            signatures: HashMap::new(),
            safety_annotations: HashMap::new(),
            processed_headers: Vec::new(),
            include_paths: Vec::new(),
            external_annotations: ExternalAnnotations::new(),
        }
    }
    
    /// Set the include paths for header file resolution
    pub fn set_include_paths(&mut self, paths: Vec<PathBuf>) {
        self.include_paths = paths;
    }
    
    /// Get a function signature by name
    pub fn get_signature(&self, func_name: &str) -> Option<&FunctionSignature> {
        self.signatures.get(func_name)
    }
    
    /// Get safety annotation for a function from headers
    pub fn get_safety_annotation(&self, func_name: &str) -> Option<SafetyMode> {
        self.safety_annotations.get(func_name).copied()
    }
    
    /// Parse a header file and extract all annotated function signatures
    pub fn parse_header(&mut self, header_path: &Path) -> Result<(), String> {
        debug_println!("DEBUG HEADER: Parsing header file: {}", header_path.display());
        
        // Skip if already processed
        if self.processed_headers.iter().any(|p| p == header_path) {
            debug_println!("DEBUG HEADER: Already processed, skipping");
            return Ok(());
        }
        
        // First, try to parse any external annotations from the header file
        if let Ok(content) = fs::read_to_string(header_path) {
            // Parse external annotations from the file content
            // These might be in comments or in the file directly
            if let Err(e) = self.external_annotations.parse_content(&content) {
                debug_println!("DEBUG HEADER: Failed to parse external annotations: {}", e);
            } else {
                debug_println!("DEBUG HEADER: Parsed external annotations from header");
            }
        }
        
        // Initialize Clang
        let clang = Clang::new()
            .map_err(|e| format!("Failed to initialize Clang: {:?}", e))?;
        let index = Index::new(&clang, false, false);
        
        // Build arguments with include paths
        let mut args = vec![
            "-std=c++17".to_string(), 
            "-xc++".to_string(),
            "-fparse-all-comments".to_string(),  // Essential for getting comments from headers
        ];
        for include_path in &self.include_paths {
            args.push(format!("-I{}", include_path.display()));
        }
        
        // Parse the header file
        let tu = index
            .parser(header_path)
            .arguments(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .parse()
            .map_err(|e| format!("Failed to parse header {}: {:?}", header_path.display(), e))?;
        
        // Extract function signatures with annotations
        let root = tu.get_entity();
        self.visit_entity_for_signatures(&root);
        
        debug_println!("DEBUG HEADER: Found {} safety annotations in header", self.safety_annotations.len());
        for (name, mode) in &self.safety_annotations {
            debug_println!("DEBUG HEADER:   - {} : {:?}", name, mode);
        }
        
        self.processed_headers.push(header_path.to_path_buf());
        Ok(())
    }
    
    /// Parse headers from a C++ source file's includes
    pub fn parse_includes_from_source(&mut self, cpp_file: &Path) -> Result<(), String> {
        let content = fs::read_to_string(cpp_file)
            .map_err(|e| format!("Failed to read {}: {}", cpp_file.display(), e))?;
        
        let (quoted_includes, angle_includes) = extract_includes(&content);
        
        // Process quoted includes (search relative to source file first)
        for include_path in quoted_includes {
            if let Some(resolved) = self.resolve_include(&include_path, cpp_file, true) {
                self.parse_header(&resolved)?;
            }
        }
        
        // Process angle bracket includes (search include paths only)
        for include_path in angle_includes {
            if let Some(resolved) = self.resolve_include(&include_path, cpp_file, false) {
                self.parse_header(&resolved)?;
            }
        }
        
        Ok(())
    }
    
    /// Resolve an include path using standard C++ include resolution rules
    fn resolve_include(&self, include_path: &str, source_file: &Path, search_source_dir: bool) -> Option<PathBuf> {
        // For quoted includes, first try relative to the source file
        if search_source_dir {
            if let Some(parent) = source_file.parent() {
                let local_path = parent.join(include_path);
                if local_path.exists() {
                    return Some(local_path);
                }
            }
        }
        
        // Search in include paths
        for include_dir in &self.include_paths {
            let full_path = include_dir.join(include_path);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        
        // Try as absolute or relative to current directory
        let path = PathBuf::from(include_path);
        if path.exists() {
            return Some(path);
        }
        
        None
    }
    
    fn visit_entity_for_signatures(&mut self, entity: &clang::Entity) {
        self.visit_entity_with_namespace(entity, None);
    }
    
    fn visit_entity_with_namespace(&mut self, entity: &clang::Entity, namespace_safety: Option<SafetyMode>) {
        use clang::EntityKind;
        
        // Check if this is a namespace with safety annotation
        let mut current_namespace_safety = namespace_safety;
        if entity.get_kind() == EntityKind::Namespace {
            if let Some(safety) = parse_entity_safety(entity) {
                current_namespace_safety = Some(safety);
                if let Some(name) = entity.get_name() {
                    debug_println!("DEBUG SAFETY: Found namespace '{}' with {:?} annotation", name, safety);
                }
            }
        }
        
        match entity.get_kind() {
            EntityKind::FunctionDecl | EntityKind::Method | EntityKind::Constructor => {
                // Extract lifetime annotations
                if let Some(sig) = extract_annotations(entity) {
                    self.signatures.insert(sig.name.clone(), sig);
                }
                
                // Extract safety annotations from the entity itself
                let mut safety = parse_entity_safety(entity);
                
                // If no explicit safety annotation, inherit from namespace
                if safety.is_none() {
                    safety = current_namespace_safety;
                }
                
                // if let Some(name) = entity.get_name() {
                //     debug_println!("DEBUG HEADER: Processing function '{}'", name);
                //     if let Some(comment) = entity.get_comment() {
                //         debug_println!("DEBUG HEADER:   Comment: {}", comment);
                //     }
                // }
                
                if let Some(safety_mode) = safety {
                    // Use qualified name for methods and constructors to avoid collisions
                    let name = if entity.get_kind() == EntityKind::Method || entity.get_kind() == EntityKind::Constructor {
                        crate::parser::ast_visitor::get_qualified_name(entity)
                    } else {
                        entity.get_name().unwrap_or_else(|| "anonymous".to_string())
                    };

                    self.safety_annotations.insert(name.clone(), safety_mode);
                    debug_println!("DEBUG SAFETY: Found function '{}' with {:?} annotation in header", name, safety_mode);
                }
            }
            _ => {}
        }
        
        // Recursively visit children, passing down namespace safety
        for child in entity.get_children() {
            self.visit_entity_with_namespace(&child, current_namespace_safety);
        }
    }
    
    /// Check if any signatures are cached
    pub fn has_signatures(&self) -> bool {
        !self.signatures.is_empty()
    }
}

/// Extract include paths from C++ source, separating quoted and angle bracket includes
fn extract_includes(content: &str) -> (Vec<String>, Vec<String>) {
    let mut quoted_includes = Vec::new();
    let mut angle_includes = Vec::new();
    
    // Match quoted includes: #include "file.h"
    let quoted_re = Regex::new(r#"#include\s*"([^"]+)""#).unwrap();
    for cap in quoted_re.captures_iter(content) {
        if let Some(path) = cap.get(1) {
            quoted_includes.push(path.as_str().to_string());
        }
    }
    
    // Match angle bracket includes: #include <file.h>
    let angle_re = Regex::new(r#"#include\s*<([^>]+)>"#).unwrap();
    for cap in angle_re.captures_iter(content) {
        if let Some(path) = cap.get(1) {
            angle_includes.push(path.as_str().to_string());
        }
    }
    
    (quoted_includes, angle_includes)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_includes() {
        let content = r#"
#include "user.h"
#include "data.h"
#include <iostream>
#include <vector>
#include "utils/helper.h"
        "#;
        
        let (quoted, angle) = extract_includes(content);
        assert_eq!(quoted.len(), 3);
        assert_eq!(quoted[0], "user.h");
        assert_eq!(quoted[1], "data.h");
        assert_eq!(quoted[2], "utils/helper.h");
        
        assert_eq!(angle.len(), 2);
        assert_eq!(angle[0], "iostream");
        assert_eq!(angle[1], "vector");
    }
}