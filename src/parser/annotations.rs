use clang::Entity;
use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum LifetimeAnnotation {
    // 'a, 'b, etc - just the lifetime name
    Lifetime(String),
    // &'a T - immutable reference with lifetime
    Ref(String),
    // &'a mut T - mutable reference with lifetime  
    MutRef(String),
    // owned - for ownership transfer
    Owned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SafetyAnnotation {
    Safe,    // @safe - enforce borrow checking
    Unsafe,  // @unsafe - skip borrow checking
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub return_lifetime: Option<LifetimeAnnotation>,
    pub param_lifetimes: Vec<Option<LifetimeAnnotation>>,
    pub lifetime_bounds: Vec<LifetimeBound>, // e.g., 'a: 'b
    pub safety: Option<SafetyAnnotation>, // @safe or @unsafe
}

#[derive(Debug, Clone)]
pub struct LifetimeBound {
    pub longer: String,  // 'a in 'a: 'b
    pub shorter: String, // 'b in 'a: 'b
}

pub fn extract_annotations(entity: &Entity) -> Option<FunctionSignature> {
    let name = entity.get_name()?;

    // Try getting comment from LibClang first (doc comments like /// or /** */)
    if let Some(comment) = entity.get_comment() {
        if let Some(sig) = parse_lifetime_annotations(&comment, name.clone()) {
            return Some(sig);
        }
        // Comment exists but no lifetime annotation found, fall through to source reading
    }

    // If no doc comment, read source file for // @lifetime: annotations
    // (similar to how we detect // @unsafe blocks)
    if let Some(sig) = read_lifetime_from_source(entity, &name) {
        return Some(sig);
    }

    None
}

/// Read lifetime annotations from source file (for // comments that LibClang doesn't capture)
fn read_lifetime_from_source(entity: &Entity, name: &str) -> Option<FunctionSignature> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let location = entity.get_location()?;
    let file_location = location.get_file_location();
    let file = file_location.file?;
    let file_path = file.get_path();
    let entity_line = file_location.line as usize;

    // Read the source file
    let file_handle = File::open(&file_path).ok()?;
    let reader = BufReader::new(file_handle);

    // Look for annotations in the lines before the entity
    let mut annotations = String::new();
    let mut current_line = 0;

    for line_result in reader.lines() {
        current_line += 1;
        let line = line_result.ok()?;

        // Check if we're at or past the entity line
        if current_line >= entity_line {
            // Parse accumulated annotations
            if !annotations.is_empty() {
                if let Some(sig) = parse_lifetime_annotations(&annotations, name.to_string()) {
                    return Some(sig);
                }
            }
            return None;
        }

        // Accumulate comment lines before the entity
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            annotations.push_str(&line);
            annotations.push('\n');
        } else if !trimmed.is_empty() && !trimmed.starts_with("/*") {
            // Non-comment, non-empty line - reset accumulation
            annotations.clear();
        }
    }

    None
}

// Parse annotations like:
// @lifetime: 'a -> &'a T
// @lifetime: ('a, 'b) -> &'a T where 'a: 'b
// @lifetime: owned
fn parse_lifetime_annotations(comment: &str, func_name: String) -> Option<FunctionSignature> {
    // Look for @safe or @unsafe annotation first
    let safe_re = Regex::new(r"@safe\b").ok()?;
    let unsafe_re = Regex::new(r"@unsafe\b").ok()?;

    let safety = if safe_re.is_match(comment) {
        Some(SafetyAnnotation::Safe)
    } else if unsafe_re.is_match(comment) {
        Some(SafetyAnnotation::Unsafe)
    } else {
        None
    };

    // Look for @lifetime annotation
    let lifetime_re = Regex::new(r"@lifetime:\s*(.+)").ok()?;

    // If we have either safety or lifetime annotations, create a signature
    if let Some(captures) = lifetime_re.captures(comment) {
        let annotation_str = captures.get(1)?.as_str();

        // Parse the annotation string
        let mut signature = FunctionSignature {
            name: func_name.clone(),
            return_lifetime: None,
            param_lifetimes: Vec::new(),
            lifetime_bounds: Vec::new(),
            safety,
        };

        // Check for where clause
        let parts: Vec<&str> = annotation_str.split("where").collect();
        let main_part = parts[0].trim();

        // Parse lifetime bounds from where clause
        if parts.len() > 1 {
            let bounds_str = parts[1].trim();
            signature.lifetime_bounds = parse_lifetime_bounds(bounds_str);
        }

        // Parse main lifetime specification
        if main_part.contains("->") {
            // Has parameters and return type
            let arrow_parts: Vec<&str> = main_part.split("->").collect();
            if arrow_parts.len() == 2 {
                let params_str = arrow_parts[0].trim();
                let return_str = arrow_parts[1].trim();

                // Parse parameters
                signature.param_lifetimes = parse_param_lifetimes(params_str);

                // Parse return type
                signature.return_lifetime = parse_single_lifetime(return_str);
            }
        } else {
            // Just return type
            signature.return_lifetime = parse_single_lifetime(main_part);
        }

        Some(signature)
    } else if safety.is_some() {
        // Even if no lifetime annotation, return signature if we have safety annotation
        Some(FunctionSignature {
            name: func_name,
            return_lifetime: None,
            param_lifetimes: Vec::new(),
            lifetime_bounds: Vec::new(),
            safety,
        })
    } else {
        None
    }
}

fn parse_param_lifetimes(params_str: &str) -> Vec<Option<LifetimeAnnotation>> {
    let mut result = Vec::new();
    
    // Remove parentheses if present
    let cleaned = params_str.trim_start_matches('(').trim_end_matches(')');
    
    // Split by comma
    for param in cleaned.split(',') {
        result.push(parse_single_lifetime(param.trim()));
    }
    
    result
}

fn parse_single_lifetime(lifetime_str: &str) -> Option<LifetimeAnnotation> {
    let trimmed = lifetime_str.trim();
    
    if trimmed == "owned" {
        Some(LifetimeAnnotation::Owned)
    } else if trimmed.starts_with("&'") && trimmed.contains("mut") {
        // &'a mut T
        let lifetime_name = extract_lifetime_name(trimmed);
        lifetime_name.map(|name| LifetimeAnnotation::MutRef(name))
    } else if trimmed.starts_with("&'") {
        // &'a T
        let lifetime_name = extract_lifetime_name(trimmed);
        lifetime_name.map(|name| LifetimeAnnotation::Ref(name))
    } else if trimmed.starts_with('\'') {
        // Just 'a
        Some(LifetimeAnnotation::Lifetime(trimmed.to_string()))
    } else {
        None
    }
}

fn extract_lifetime_name(s: &str) -> Option<String> {
    let re = Regex::new(r"'([a-z][a-z0-9]*)").ok()?;
    re.captures(s)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn parse_lifetime_bounds(bounds_str: &str) -> Vec<LifetimeBound> {
    let mut bounds = Vec::new();
    
    // Parse patterns like 'a: 'b
    let bound_re = Regex::new(r"'([a-z][a-z0-9]*)\s*:\s*'([a-z][a-z0-9]*)").unwrap();
    
    for cap in bound_re.captures_iter(bounds_str) {
        if let (Some(longer), Some(shorter)) = (cap.get(1), cap.get(2)) {
            bounds.push(LifetimeBound {
                longer: longer.as_str().to_string(),
                shorter: shorter.as_str().to_string(),
            });
        }
    }
    
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_lifetime() {
        let comment = "// @lifetime: &'a";
        let sig = parse_lifetime_annotations(comment, "test".to_string()).unwrap();
        
        assert_eq!(sig.name, "test");
        assert_eq!(sig.return_lifetime, Some(LifetimeAnnotation::Ref("a".to_string())));
        assert!(sig.param_lifetimes.is_empty());
    }
    
    #[test]
    fn test_parse_owned() {
        let comment = "// @lifetime: owned";
        let sig = parse_lifetime_annotations(comment, "test".to_string()).unwrap();
        
        assert_eq!(sig.return_lifetime, Some(LifetimeAnnotation::Owned));
    }
    
    #[test]
    fn test_parse_with_params() {
        let comment = "// @lifetime: (&'a, &'b) -> &'a where 'a: 'b";
        let sig = parse_lifetime_annotations(comment, "test".to_string()).unwrap();
        
        assert_eq!(sig.param_lifetimes.len(), 2);
        assert_eq!(sig.param_lifetimes[0], Some(LifetimeAnnotation::Ref("a".to_string())));
        assert_eq!(sig.param_lifetimes[1], Some(LifetimeAnnotation::Ref("b".to_string())));
        assert_eq!(sig.return_lifetime, Some(LifetimeAnnotation::Ref("a".to_string())));
        assert_eq!(sig.lifetime_bounds.len(), 1);
        assert_eq!(sig.lifetime_bounds[0].longer, "a");
        assert_eq!(sig.lifetime_bounds[0].shorter, "b");
    }
    
    #[test]
    fn test_parse_mut_ref() {
        let comment = "// @lifetime: &'a mut";
        let sig = parse_lifetime_annotations(comment, "test".to_string()).unwrap();
        
        assert_eq!(sig.return_lifetime, Some(LifetimeAnnotation::MutRef("a".to_string())));
    }
}