use crate::debug_println;
use clang::{Clang, Index};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::annotations::{FunctionSignature, extract_annotations, parse_lifetime_annotations};
use super::external_annotations::ExternalAnnotations;
use super::safety_annotations::{SafetyMode, parse_entity_safety};

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

/// Strip template parameters from a name (e.g., "Option<T>" -> "Option")
fn strip_template_params(name: &str) -> String {
    // println!("DEBUG: Stripping template parameters from '{}'", name);
    if let Some(pos) = name.find('<') {
        let stripped_name = name[..pos].to_string();
        // println!("DEBUG: Name after stripping: '{}'", stripped_name);
        return stripped_name;
    } else {
        name.to_string()
    }
}

fn qualified_name_from_scope_stack(scope_stack: &[(String, i32)]) -> Option<String> {
    let parts: Vec<&str> = scope_stack
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| !name.is_empty())
        .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("::"))
    }
}

fn is_namespace_line(line: &str) -> bool {
    (line.starts_with("namespace ") || line.contains(" namespace ")) && line.contains('{')
}

fn extract_namespace_name(line: &str) -> Option<String> {
    let namespace_pos = line.find("namespace")?;
    let after_namespace = line[namespace_pos + "namespace".len()..].trim_start();
    let name: String = after_namespace
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if name.is_empty() { None } else { Some(name) }
}

fn is_class_line(line: &str) -> bool {
    (line.starts_with("class ")
        || line.starts_with("struct ")
        || line.contains(" class ")
        || line.contains(" struct "))
        && line.contains('{')
}

fn extract_class_name_for_lifetime_scan(line: &str) -> Option<String> {
    let keyword = if let Some(pos) = line.find("class ") {
        (pos, "class")
    } else if let Some(pos) = line.find("struct ") {
        (pos, "struct")
    } else {
        return None;
    };

    let after_keyword = line[keyword.0 + keyword.1.len()..].trim_start();
    let raw_name: String = after_keyword
        .chars()
        .take_while(|c| {
            c.is_alphanumeric()
                || *c == '_'
                || *c == ':'
                || *c == '<'
                || *c == '>'
                || *c == '&'
                || *c == ','
                || *c == ' '
                || *c == '*'
        })
        .collect();
    let first_name = raw_name.split_whitespace().next()?;
    let simple_name = first_name.rsplit("::").next().unwrap_or(first_name);

    Some(strip_template_params(simple_name))
}

fn extract_annotated_function_name(line: &str) -> Option<String> {
    if !line.contains('(') {
        return None;
    }

    let before_paren = line.split('(').next()?.trim_end();
    if before_paren.is_empty() {
        return None;
    }

    if let Some(operator_pos) = before_paren.rfind("operator") {
        return Some(before_paren[operator_pos..].trim().to_string());
    }

    let token = before_paren.split_whitespace().last()?;
    let simple_name = token.rsplit("::").next().unwrap_or(token);
    let name = strip_template_params(simple_name)
        .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '~')
        .to_string();

    if name.is_empty() { None } else { Some(name) }
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

    fn insert_signature(&mut self, qualified_name: String, sig: FunctionSignature) {
        if sig.return_lifetime.is_none() {
            if let Some(existing) = self.signatures.get(&qualified_name) {
                if existing.return_lifetime.is_some() {
                    debug_println!(
                        "DEBUG HEADER: Preserving existing lifetime signature for '{}'",
                        qualified_name
                    );
                    return;
                }
            }
        }

        self.signatures.insert(qualified_name, sig);
    }

    /// Parse a header file and extract all annotated function signatures
    pub fn parse_header(&mut self, header_path: &Path) -> Result<(), String> {
        debug_println!(
            "DEBUG HEADER: Parsing header file: {}",
            header_path.display()
        );

        // Skip if already processed
        if self.processed_headers.iter().any(|p| p == header_path) {
            debug_println!("DEBUG HEADER: Already processed, skipping");
            return Ok(());
        }

        // Parse safety annotations directly from the header file (before libclang parsing)
        // This ensures we get regular C++ comments (// and /* */) not just Doxygen comments
        // Store temporarily - we'll qualify the names after LibClang parsing
        let mut unqualified_annotations = HashMap::new();
        if let Ok(header_safety_context) =
            super::safety_annotations::parse_safety_annotations(header_path)
        {
            // Store unqualified annotations temporarily
            for (func_sig, safety_mode) in &header_safety_context.function_overrides {
                debug_println!(
                    "DEBUG HEADER: Found unqualified annotation for '{}': {:?}",
                    func_sig.name,
                    safety_mode
                );
                unqualified_annotations.insert(func_sig.name.clone(), *safety_mode);
            }
            debug_println!(
                "DEBUG HEADER: Parsed {} unqualified safety annotations from header file",
                header_safety_context.function_overrides.len()
            );
        }

        // Also parse external annotations from the header file
        if let Ok(content) = fs::read_to_string(header_path) {
            // Parse external annotations from the file content
            // These might be in comments or in the file directly
            if let Err(e) = self.external_annotations.parse_content(&content) {
                debug_println!("DEBUG HEADER: Failed to parse external annotations: {}", e);
            } else {
                debug_println!("DEBUG HEADER: Parsed external annotations from header");
            }

            self.parse_lifetime_annotations_from_text(&content);
        }

        // Initialize Clang
        let clang = Clang::new().map_err(|e| format!("Failed to initialize Clang: {:?}", e))?;
        let index = Index::new(&clang, false, false);

        // Build arguments with include paths
        let mut args = vec![
            "-std=c++17".to_string(),
            "-xc++".to_string(),
            "-fparse-all-comments".to_string(), // Essential for getting comments from headers
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

        // Now qualify the unqualified annotations using the qualified names from LibClang
        // Build a map from simple method names to their qualified names
        let mut simple_to_qualified: HashMap<String, Vec<String>> = HashMap::new();
        for qualified_name in self.safety_annotations.keys() {
            // Extract the simple name (last component after ::)
            if let Some(simple_name) = qualified_name.split("::").last() {
                simple_to_qualified
                    .entry(simple_name.to_string())
                    .or_insert_with(Vec::new)
                    .push(qualified_name.clone());
            }
        }

        // Now qualify the unqualified annotations
        debug_println!(
            "DEBUG HEADER: Qualifying {} unqualified annotations",
            unqualified_annotations.len()
        );
        for (simple_name, safety_mode) in &unqualified_annotations {
            debug_println!(
                "DEBUG HEADER: Processing unqualified '{}': {:?}",
                simple_name,
                safety_mode
            );
            // Check if this simple name has qualified versions from LibClang
            if let Some(qualified_names) = simple_to_qualified.get(simple_name) {
                // This is a method - add annotation for all qualified versions
                for qualified in qualified_names {
                    debug_println!(
                        "DEBUG HEADER: Qualifying '{}' -> '{}': {:?}",
                        simple_name,
                        qualified,
                        safety_mode
                    );
                    // Update the annotation (LibClang may have found it too, but comment annotation takes precedence)
                    self.safety_annotations
                        .insert(qualified.clone(), *safety_mode);
                }
            } else {
                // Not a method (no qualified name found), just a plain function
                // Keep the simple name
                debug_println!(
                    "DEBUG HEADER: Adding plain function annotation for '{}': {:?}",
                    simple_name,
                    safety_mode
                );
                self.safety_annotations
                    .insert(simple_name.clone(), *safety_mode);
            }
        }

        debug_println!(
            "DEBUG HEADER: Found {} safety annotations in header (after qualification)",
            self.safety_annotations.len()
        );
        for (name, mode) in &self.safety_annotations {
            debug_println!("DEBUG HEADER:   - {} : {:?}", name, mode);
        }

        // Mark as processed BEFORE parsing includes to avoid infinite recursion
        self.processed_headers.push(header_path.to_path_buf());

        // Recursively parse includes from this header
        if let Ok(content) = fs::read_to_string(header_path) {
            let (quoted_includes, angle_includes) = extract_includes(&content);

            // Process quoted includes (search relative to header file first)
            for include_path in quoted_includes {
                if let Some(resolved) = self.resolve_include(&include_path, header_path, true) {
                    // Recursively parse the included header
                    let _ = self.parse_header(&resolved);
                }
            }

            // Process angle bracket includes (search include paths only)
            for include_path in angle_includes {
                if let Some(resolved) = self.resolve_include(&include_path, header_path, false) {
                    // Recursively parse the included header
                    let _ = self.parse_header(&resolved);
                }
            }
        }

        Ok(())
    }

    fn parse_lifetime_annotations_from_text(&mut self, content: &str) {
        let mut pending_lifetime: Option<String> = None;
        let mut scope_stack: Vec<(String, i32)> = Vec::new();
        let mut current_depth = 0i32;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("//") && trimmed.contains("@lifetime:") {
                pending_lifetime = Some(trimmed.to_string());
                continue;
            }

            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
                continue;
            }

            let opens = trimmed.matches('{').count() as i32;
            let closes = trimmed.matches('}').count() as i32;

            if let Some(comment) = pending_lifetime.take() {
                if let Some(func_name) = extract_annotated_function_name(trimmed) {
                    let qualified_name = match qualified_name_from_scope_stack(&scope_stack) {
                        Some(prefix) => format!("{}::{}", prefix, func_name),
                        None => func_name.clone(),
                    };

                    if let Some(mut sig) = parse_lifetime_annotations(&comment, func_name.clone()) {
                        sig.name = qualified_name.clone();
                        debug_println!(
                            "DEBUG HEADER: Text lifetime annotation '{}' -> {:?}",
                            qualified_name,
                            sig
                        );
                        self.insert_signature(qualified_name, sig);
                    }
                } else if !trimmed.ends_with(';') && !trimmed.contains('{') {
                    pending_lifetime = Some(comment);
                }
            }

            if is_namespace_line(trimmed) {
                if let Some(ns_name) = extract_namespace_name(trimmed) {
                    if !ns_name.is_empty() {
                        scope_stack.push((ns_name, current_depth + opens));
                    }
                }
            } else if is_class_line(trimmed) {
                if let Some(class_name) = extract_class_name_for_lifetime_scan(trimmed) {
                    scope_stack.push((class_name, current_depth + opens));
                }
            }

            current_depth += opens - closes;
            if current_depth < 0 {
                current_depth = 0;
            }

            while let Some((_, push_depth)) = scope_stack.last() {
                if *push_depth > current_depth {
                    scope_stack.pop();
                } else {
                    break;
                }
            }
        }
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

        // Process C++23 named-module imports. Each `import X.Y.Z;` is resolved
        // to the project source file that declares `export module X.Y.Z;`,
        // then parsed for safety annotations (text pre-pass only — LibClang
        // can't parse module units without the full module-graph setup).
        for module_name in extract_module_imports(&content) {
            if let Some(resolved) = self.resolve_module_import(&module_name) {
                self.parse_module_source_for_annotations(&resolved)?;
            } else {
                debug_println!(
                    "DEBUG HEADER: Could not resolve module import '{}' to a source file",
                    module_name
                );
            }
        }

        Ok(())
    }

    /// Locate the project source file that declares `export module <name>;`.
    /// Searches include paths (and their subdirectories) for files whose last
    /// path component matches the module name's last segment, then verifies
    /// the file's `export module ...;` declaration matches the full name.
    fn resolve_module_import(&self, module_name: &str) -> Option<PathBuf> {
        // The convention `rrr.foo` maps roughly to `<root>/.../foo.{cpp,cppm,...}`.
        let last_segment = module_name.split('.').last()?;
        let extensions = ["cpp", "cppm", "cc", "cxx", "ixx"];

        for include_dir in &self.include_paths {
            for ext in &extensions {
                let filename = format!("{}.{}", last_segment, ext);
                if let Some(found) = find_file_recursive(include_dir, &filename, 5) {
                    if module_declaration_matches(&found, module_name) {
                        return Some(found);
                    }
                }
            }
        }

        None
    }

    /// Lightweight annotation-only parse for a C++23 module source unit.
    /// Skips LibClang (which would fail without the full module graph) and
    /// runs only the text-based safety-annotation pre-pass, qualifying the
    /// resulting names against namespace context.
    fn parse_module_source_for_annotations(&mut self, module_path: &Path) -> Result<(), String> {
        if self.processed_headers.iter().any(|p| p == module_path) {
            return Ok(());
        }
        self.processed_headers.push(module_path.to_path_buf());

        debug_println!(
            "DEBUG HEADER: Parsing module source for annotations: {}",
            module_path.display()
        );

        // Derive the namespace prefix from the module's `export module X.Y.Z;`
        // declaration: by strong project convention (and what every rrr file
        // actually does) the symbols a module exports live in `namespace X {
        // ... }` where X is the module name's first segment. The text-pre-pass
        // safety_annotations parser does its own brace-counting for namespace
        // context, but that counter drifts on complex source (string-literal
        // braces, macros, etc.) and frequently fails to qualify symbols. We
        // patch that here: any unqualified name from the text parse gets the
        // module's top-level namespace prepended so lookups like
        // `rrr::Log_debug` resolve.
        let module_namespace = fs::read_to_string(module_path).ok().and_then(|content| {
            let re = Regex::new(r"(?m)^\s*export\s+module\s+([a-zA-Z_][a-zA-Z_0-9]*)\b").unwrap();
            re.captures(&content)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
        });

        if let Ok(ctx) = super::safety_annotations::parse_safety_annotations(module_path) {
            for (func_sig, safety_mode) in &ctx.function_overrides {
                let name = if !func_sig.name.contains("::") {
                    if let Some(ns) = module_namespace.as_deref() {
                        format!("{}::{}", ns, func_sig.name)
                    } else {
                        func_sig.name.clone()
                    }
                } else {
                    func_sig.name.clone()
                };
                debug_println!(
                    "DEBUG HEADER: Module annotation '{}': {:?}",
                    name,
                    safety_mode
                );
                self.safety_annotations.insert(name, *safety_mode);
            }
        }

        // Also parse external annotations from the module source.
        if let Ok(content) = fs::read_to_string(module_path) {
            let _ = self.external_annotations.parse_content(&content);

            // Recursively chase imports.
            for nested in extract_module_imports(&content) {
                if let Some(resolved) = self.resolve_module_import(&nested) {
                    self.parse_module_source_for_annotations(&resolved)?;
                }
            }
        }

        Ok(())
    }

    /// Resolve an include path using standard C++ include resolution rules
    fn resolve_include(
        &self,
        include_path: &str,
        source_file: &Path,
        search_source_dir: bool,
    ) -> Option<PathBuf> {
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
        self.visit_entity_with_context(entity, None, None);
    }

    /// Visit entities tracking both namespace and class-level safety annotations.
    /// Annotation hierarchy: function > class > namespace
    fn visit_entity_with_context(
        &mut self,
        entity: &clang::Entity,
        namespace_safety: Option<SafetyMode>,
        class_safety: Option<SafetyMode>,
    ) {
        use clang::EntityKind;

        // Track current context
        let mut current_namespace_safety = namespace_safety;
        let mut current_class_safety = class_safety;

        // Check if this is a namespace with safety annotation
        if entity.get_kind() == EntityKind::Namespace {
            if let Some(safety) = parse_entity_safety(entity) {
                current_namespace_safety = Some(safety);
                if let Some(name) = entity.get_name() {
                    debug_println!(
                        "DEBUG SAFETY: Found namespace '{}' with {:?} annotation",
                        name,
                        safety
                    );
                }
            } else {
                // IMPORTANT: Reset namespace safety when entering a namespace without annotation
                // This prevents safety from leaking from one namespace to another
                // (e.g., user's @safe namespace shouldn't apply to std::)
                current_namespace_safety = None;
                debug_println!(
                    "DEBUG SAFETY: Entering namespace {:?} without annotation - resetting namespace safety",
                    entity.get_name()
                );
            }
        }

        // Check if this is a class/struct with safety annotation
        if entity.get_kind() == EntityKind::ClassDecl || entity.get_kind() == EntityKind::StructDecl
        {
            if let Some(safety) = parse_entity_safety(entity) {
                current_class_safety = Some(safety);
                if let Some(name) = entity.get_name() {
                    debug_println!(
                        "DEBUG SAFETY: Found class '{}' with {:?} annotation in header",
                        name,
                        safety
                    );
                }
            } else if current_namespace_safety.is_some() {
                // If class has no explicit annotation, DON'T inherit from namespace
                // Classes without annotations are undeclared
                current_class_safety = None;
            }
        }

        match entity.get_kind() {
            EntityKind::FunctionDecl
            | EntityKind::Method
            | EntityKind::Constructor
            | EntityKind::FunctionTemplate => {
                // Extract lifetime annotations
                debug_println!("ALVIN DEBUG ANALYSIS: EXTRACTING ANNOTATIONS");
                if let Some(mut sig) = extract_annotations(entity) {
                    // Always use qualified name for all functions to avoid namespace collisions
                    // This ensures functions like ns1::helper and ns2::helper are distinguished
                    let qualified_name = crate::parser::ast_visitor::get_qualified_name(entity);
                    // Update the signature name to use qualified name
                    sig.name = qualified_name.clone();
                    debug_println!(
                        "ALVIN DEBUG ANALYSIS: Found function '{}' with signature {:?} in header",
                        sig.name,
                        sig
                    );
                    self.insert_signature(qualified_name, sig);
                } else {
                    debug_println!(
                        "ALVIN DEBUG ANALYSIS: No annotations found for function '{}'",
                        entity.get_name().unwrap_or("<unnamed>".to_string())
                    );
                }

                // Extract safety annotations from the entity itself
                let mut safety = parse_entity_safety(entity);

                // If no explicit safety annotation, inherit from class first, then namespace
                // Hierarchy: function > class > namespace
                if safety.is_none() {
                    if current_class_safety.is_some() {
                        safety = current_class_safety;
                        debug_println!("DEBUG SAFETY: Method inheriting {:?} from class", safety);
                    } else {
                        safety = current_namespace_safety;
                        if safety.is_some() {
                            debug_println!(
                                "DEBUG SAFETY: Function inheriting {:?} from namespace",
                                safety
                            );
                        }
                    }
                }

                if let Some(safety_mode) = safety {
                    // Always use qualified name for all functions to avoid namespace collisions
                    // This ensures functions like ns1::helper and ns2::helper are distinguished
                    let raw_name = crate::parser::ast_visitor::get_qualified_name(entity);

                    // For template constructors, the name may include template params like "Option<T>"
                    // Strip template params so lookups match (call sites use "Option", not "Option<T>")
                    let name = strip_template_params(&raw_name);

                    self.safety_annotations.insert(name.clone(), safety_mode);
                    debug_println!(
                        "DEBUG SAFETY: Found function '{}' with {:?} annotation in header",
                        name,
                        safety_mode
                    );
                }
            }
            _ => {}
        }

        // Recursively visit children, passing down context
        // For class children, pass current_class_safety
        // For namespace children (not inside a class), pass None for class_safety
        let child_class_safety = if entity.get_kind() == EntityKind::ClassDecl
            || entity.get_kind() == EntityKind::StructDecl
        {
            current_class_safety
        } else {
            class_safety // Keep parent's class safety for nested entities
        };

        for child in entity.get_children() {
            self.visit_entity_with_context(&child, current_namespace_safety, child_class_safety);
        }
    }

    /// Check if any signatures are cached
    pub fn has_signatures(&self) -> bool {
        !self.signatures.is_empty()
    }
}

/// Extract include paths from C++ source, separating quoted and angle bracket includes
/// Walk `root` up to `max_depth` levels, returning the first file whose final
/// path component equals `filename`. Cheap depth-limited search used to map
/// `import X.foo;` → `<somewhere>/foo.cpp` without indexing the whole tree.
fn find_file_recursive(root: &Path, filename: &str, max_depth: usize) -> Option<PathBuf> {
    if !root.is_dir() {
        return None;
    }
    let direct = root.join(filename);
    if direct.is_file() {
        return Some(direct);
    }
    if max_depth == 0 {
        return None;
    }
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        // Skip hidden/build directories — common noise that would otherwise
        // dominate the search (.git, build/, target/).
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') || name == "build" || name == "target" {
                continue;
            }
        }
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, filename, max_depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

/// Return true if the file's first ~60 lines contain
/// `export module <module_name>;` (whitespace-tolerant, comment-tolerant).
fn module_declaration_matches(file: &Path, module_name: &str) -> bool {
    use std::io::{BufRead, BufReader};
    let Ok(handle) = fs::File::open(file) else {
        return false;
    };
    let reader = BufReader::new(handle);
    let target = format!("export module {};", module_name);
    for (i, line) in reader.lines().enumerate() {
        if i >= 60 {
            break;
        }
        let Ok(line) = line else { return false };
        if line.contains(&target) {
            return true;
        }
    }
    false
}

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

/// Extract C++23 module imports of the form `import X.Y.Z;` from a source file.
/// Returns the module names. Skips `import std;` (no project source to chase),
/// `import std.compat;`, header-unit imports (`import <foo>;`,
/// `import "foo";`), and partition imports (`import :part;`).
fn extract_module_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    // `import` must be at the start of a logical line (allow leading whitespace).
    // The dotted module name uses [a-zA-Z_][a-zA-Z_0-9]* segments joined by '.'.
    // Reject header-unit forms (next non-space char after `import` is `<` or `"`)
    // and partition forms (`import :name;`).
    let re = Regex::new(r"(?m)^\s*import\s+([a-zA-Z_][a-zA-Z_0-9.]*)\s*;").unwrap();
    for cap in re.captures_iter(content) {
        if let Some(name) = cap.get(1) {
            let n = name.as_str();
            // Skip standard library module — analyzed independently as a BMI
            // via `-fmodule-file=std=...`, not as project source.
            if n == "std" || n.starts_with("std.") {
                continue;
            }
            imports.push(n.to_string());
        }
    }
    imports
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

    #[test]
    fn test_strip_template_params_simple() {
        // Simple template class name
        assert_eq!(strip_template_params("Option<T>"), "Option");
        assert_eq!(strip_template_params("Vector<int>"), "Vector");
        assert_eq!(strip_template_params("Map<K, V>"), "Map");
    }

    #[test]
    fn test_strip_template_params_nested() {
        // Nested template parameters
        assert_eq!(strip_template_params("Option<Vector<int>>"), "Option");
        assert_eq!(strip_template_params("Map<string, Vector<int>>"), "Map");
    }

    #[test]
    fn test_strip_template_params_qualified() {
        // Qualified names with templates
        assert_eq!(strip_template_params("rusty::Option<T>"), "rusty::Option");
        assert_eq!(strip_template_params("std::vector<int>"), "std::vector");
        assert_eq!(
            strip_template_params("ns::inner::Class<T, U>"),
            "ns::inner::Class"
        );
    }

    #[test]
    fn test_strip_template_params_no_template() {
        // Names without template parameters should be unchanged
        assert_eq!(strip_template_params("Option"), "Option");
        assert_eq!(strip_template_params("rusty::Option"), "rusty::Option");
        assert_eq!(strip_template_params("some_function"), "some_function");
    }

    #[test]
    fn test_strip_template_params_constructor() {
        // Constructor names like "Option<T>::Option<T>" -> "Option::Option"
        // Note: This tests the function itself, not the full qualified name handling
        assert_eq!(strip_template_params("Option<T>::Option"), "Option");
    }
}
