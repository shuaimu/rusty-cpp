// Type-level lifetime annotations for STL and other types
// This module handles @type_lifetime annotations that describe
// how lifetimes flow through type methods and members

use std::collections::HashMap;
use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeLifetime {
    // &'self - borrows from the object
    SelfRef,
    // &'self mut - mutably borrows from the object  
    SelfMutRef,
    // &'a - reference with explicit lifetime
    Ref(String),
    // &'a mut - mutable reference with explicit lifetime
    MutRef(String),
    // *const - const raw pointer
    ConstPtr,
    // *mut - mutable raw pointer
    MutPtr,
    // owned - ownership transfer
    Owned,
}

#[derive(Debug, Clone)]
pub struct MethodLifetime {
    pub method_name: String,
    pub is_const: bool,
    pub param_lifetimes: Vec<TypeLifetime>,
    pub return_lifetime: TypeLifetime,
}

#[derive(Debug, Clone)]
pub struct TypeLifetimeSpec {
    pub type_name: String,
    pub methods: HashMap<String, Vec<MethodLifetime>>, // method name -> overloads
    pub members: HashMap<String, TypeLifetime>,
    pub typedefs: HashMap<String, TypeLifetime>, // iterator, const_iterator, etc.
}

// Global registry of type lifetime specifications
pub struct TypeLifetimeRegistry {
    specs: HashMap<String, TypeLifetimeSpec>,
}

impl TypeLifetimeRegistry {
    pub fn new() -> Self {
        let mut registry = TypeLifetimeRegistry {
            specs: HashMap::new(),
        };
        registry.load_stl_annotations();
        registry
    }
    
    pub fn get_type_spec(&self, type_name: &str) -> Option<&TypeLifetimeSpec> {
        // Handle template instantiations like vector<int>
        let base_type = extract_base_type(type_name);
        self.specs.get(base_type)
    }
    
    pub fn add_type_spec(&mut self, spec: TypeLifetimeSpec) {
        self.specs.insert(spec.type_name.clone(), spec);
    }
    
    // Load built-in STL annotations
    fn load_stl_annotations(&mut self) {
        // std::vector<T>
        let mut vector_spec = TypeLifetimeSpec {
            type_name: "std::vector".to_string(),
            methods: HashMap::new(),
            members: HashMap::new(),
            typedefs: HashMap::new(),
        };
        
        // Add vector methods
        vector_spec.methods.insert("at".to_string(), vec![
            MethodLifetime {
                method_name: "at".to_string(),
                is_const: false,
                param_lifetimes: vec![TypeLifetime::Owned], // size_t
                return_lifetime: TypeLifetime::SelfMutRef,
            },
            MethodLifetime {
                method_name: "at".to_string(),
                is_const: true,
                param_lifetimes: vec![TypeLifetime::Owned], // size_t
                return_lifetime: TypeLifetime::SelfRef,
            },
        ]);
        
        vector_spec.methods.insert("operator[]".to_string(), vec![
            MethodLifetime {
                method_name: "operator[]".to_string(),
                is_const: false,
                param_lifetimes: vec![TypeLifetime::Owned], // size_t
                return_lifetime: TypeLifetime::SelfMutRef,
            },
            MethodLifetime {
                method_name: "operator[]".to_string(),
                is_const: true,
                param_lifetimes: vec![TypeLifetime::Owned], // size_t
                return_lifetime: TypeLifetime::SelfRef,
            },
        ]);
        
        vector_spec.methods.insert("push_back".to_string(), vec![
            MethodLifetime {
                method_name: "push_back".to_string(),
                is_const: false,
                param_lifetimes: vec![TypeLifetime::Owned], // T
                return_lifetime: TypeLifetime::Owned, // void
            },
        ]);
        
        vector_spec.methods.insert("data".to_string(), vec![
            MethodLifetime {
                method_name: "data".to_string(),
                is_const: false,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::MutPtr,
            },
            MethodLifetime {
                method_name: "data".to_string(),
                is_const: true,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::ConstPtr,
            },
        ]);
        
        vector_spec.methods.insert("begin".to_string(), vec![
            MethodLifetime {
                method_name: "begin".to_string(),
                is_const: false,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::SelfMutRef,
            },
            MethodLifetime {
                method_name: "begin".to_string(),
                is_const: true,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::SelfRef,
            },
        ]);
        
        vector_spec.typedefs.insert("iterator".to_string(), TypeLifetime::SelfMutRef);
        vector_spec.typedefs.insert("const_iterator".to_string(), TypeLifetime::SelfRef);
        
        self.specs.insert("std::vector".to_string(), vector_spec);
        
        // std::unique_ptr<T>
        let mut unique_ptr_spec = TypeLifetimeSpec {
            type_name: "std::unique_ptr".to_string(),
            methods: HashMap::new(),
            members: HashMap::new(),
            typedefs: HashMap::new(),
        };
        
        unique_ptr_spec.methods.insert("get".to_string(), vec![
            MethodLifetime {
                method_name: "get".to_string(),
                is_const: false,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::MutPtr,
            },
            MethodLifetime {
                method_name: "get".to_string(),
                is_const: true,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::ConstPtr,
            },
        ]);
        
        unique_ptr_spec.methods.insert("operator*".to_string(), vec![
            MethodLifetime {
                method_name: "operator*".to_string(),
                is_const: false,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::SelfMutRef,
            },
            MethodLifetime {
                method_name: "operator*".to_string(),
                is_const: true,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::SelfRef,
            },
        ]);
        
        unique_ptr_spec.methods.insert("release".to_string(), vec![
            MethodLifetime {
                method_name: "release".to_string(),
                is_const: false,
                param_lifetimes: vec![],
                return_lifetime: TypeLifetime::Owned,
            },
        ]);
        
        self.specs.insert("std::unique_ptr".to_string(), unique_ptr_spec);
        
        // std::map<K,V>
        let mut map_spec = TypeLifetimeSpec {
            type_name: "std::map".to_string(),
            methods: HashMap::new(),
            members: HashMap::new(),
            typedefs: HashMap::new(),
        };
        
        map_spec.methods.insert("at".to_string(), vec![
            MethodLifetime {
                method_name: "at".to_string(),
                is_const: false,
                param_lifetimes: vec![TypeLifetime::SelfRef], // const K&
                return_lifetime: TypeLifetime::SelfMutRef,
            },
            MethodLifetime {
                method_name: "at".to_string(),
                is_const: true,
                param_lifetimes: vec![TypeLifetime::SelfRef], // const K&
                return_lifetime: TypeLifetime::SelfRef,
            },
        ]);
        
        map_spec.methods.insert("operator[]".to_string(), vec![
            MethodLifetime {
                method_name: "operator[]".to_string(),
                is_const: false,
                param_lifetimes: vec![TypeLifetime::SelfRef], // const K&
                return_lifetime: TypeLifetime::SelfMutRef,
            },
        ]);
        
        map_spec.methods.insert("find".to_string(), vec![
            MethodLifetime {
                method_name: "find".to_string(),
                is_const: false,
                param_lifetimes: vec![TypeLifetime::SelfRef], // const K&
                return_lifetime: TypeLifetime::SelfMutRef,
            },
            MethodLifetime {
                method_name: "find".to_string(),
                is_const: true,
                param_lifetimes: vec![TypeLifetime::SelfRef], // const K&
                return_lifetime: TypeLifetime::SelfRef,
            },
        ]);
        
        self.specs.insert("std::map".to_string(), map_spec);
        
        // Add more STL types as needed...
    }
}

// Parse @type_lifetime annotations from comments
pub fn parse_type_lifetime_annotation(comment: &str) -> Option<TypeLifetimeSpec> {
    let type_re = Regex::new(r"@type_lifetime:\s*([^{]+)\s*\{([^}]+)\}").ok()?;
    
    if let Some(captures) = type_re.captures(comment) {
        let type_name = captures.get(1)?.as_str().trim();
        let body = captures.get(2)?.as_str();
        
        let mut spec = TypeLifetimeSpec {
            type_name: type_name.to_string(),
            methods: HashMap::new(),
            members: HashMap::new(),
            typedefs: HashMap::new(),
        };
        
        // Parse each line in the body
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            
            if line.contains("->") {
                // Method signature
                parse_method_lifetime(line, &mut spec);
            } else if line.contains(":") {
                // Member or typedef
                parse_member_lifetime(line, &mut spec);
            }
        }
        
        Some(spec)
    } else {
        None
    }
}

fn parse_method_lifetime(line: &str, spec: &mut TypeLifetimeSpec) {
    // Parse patterns like: at(size_t) const -> &'self
    let method_re = Regex::new(r"(\w+)\((.*?)\)(\s+const)?\s*->\s*(.+)").unwrap();
    
    if let Some(captures) = method_re.captures(line) {
        let method_name = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let params = captures.get(2).map(|m| m.as_str()).unwrap_or("");
        let is_const = captures.get(3).is_some();
        let return_type = captures.get(4).map(|m| m.as_str()).unwrap_or("");
        
        let method_lifetime = MethodLifetime {
            method_name: method_name.to_string(),
            is_const,
            param_lifetimes: parse_param_types(params),
            return_lifetime: parse_type_lifetime(return_type),
        };
        
        spec.methods.entry(method_name.to_string())
            .or_insert_with(Vec::new)
            .push(method_lifetime);
    }
}

fn parse_member_lifetime(line: &str, spec: &mut TypeLifetimeSpec) {
    // Parse patterns like: iterator: &'self mut
    if let Some(colon_pos) = line.find(':') {
        let name = line[..colon_pos].trim();
        let lifetime = line[colon_pos + 1..].trim();
        
        // Check if it's a known typedef
        if name == "iterator" || name == "const_iterator" || 
           name == "reference" || name == "const_reference" ||
           name == "pointer" || name == "const_pointer" {
            spec.typedefs.insert(name.to_string(), parse_type_lifetime(lifetime));
        } else {
            spec.members.insert(name.to_string(), parse_type_lifetime(lifetime));
        }
    }
}

fn parse_param_types(params: &str) -> Vec<TypeLifetime> {
    if params.is_empty() {
        return vec![];
    }
    
    params.split(',')
        .map(|p| parse_type_lifetime(p.trim()))
        .collect()
}

fn parse_type_lifetime(s: &str) -> TypeLifetime {
    let s = s.trim();
    
    match s {
        "&'self" => TypeLifetime::SelfRef,
        "&'self mut" => TypeLifetime::SelfMutRef,
        "*const" => TypeLifetime::ConstPtr,
        "*mut" => TypeLifetime::MutPtr,
        "owned" => TypeLifetime::Owned,
        _ => {
            if s.starts_with("&'") && s.contains("mut") {
                // Extract lifetime name
                if let Some(name) = extract_lifetime_name(s) {
                    TypeLifetime::MutRef(name)
                } else {
                    TypeLifetime::Owned
                }
            } else if s.starts_with("&'") {
                if let Some(name) = extract_lifetime_name(s) {
                    TypeLifetime::Ref(name)
                } else {
                    TypeLifetime::Owned
                }
            } else {
                TypeLifetime::Owned
            }
        }
    }
}

fn extract_lifetime_name(s: &str) -> Option<String> {
    let re = Regex::new(r"'([a-z][a-z0-9]*)").ok()?;
    re.captures(s)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_base_type(type_name: &str) -> &str {
    // Extract base type from template instantiation
    // e.g., "std::vector<int>" -> "std::vector"
    if let Some(angle_pos) = type_name.find('<') {
        &type_name[..angle_pos]
    } else {
        type_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_type_lifetime() {
        let comment = r#"
        @type_lifetime: std::vector<T> {
            at(size_t) -> &'self mut
            at(size_t) const -> &'self
            data() -> *mut
            iterator: &'self mut
        }
        "#;
        
        let spec = parse_type_lifetime_annotation(comment).unwrap();
        assert_eq!(spec.type_name, "std::vector<T>");
        assert_eq!(spec.methods["at"].len(), 2);
        assert_eq!(spec.typedefs["iterator"], TypeLifetime::SelfMutRef);
    }
    
    #[test]
    fn test_registry() {
        let registry = TypeLifetimeRegistry::new();
        let vector_spec = registry.get_type_spec("std::vector<int>").unwrap();
        assert_eq!(vector_spec.type_name, "std::vector");
    }
}