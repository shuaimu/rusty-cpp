use crate::codegen::CodeGen;

/// Transpile Rust source code to C++ code.
pub fn transpile(rust_source: &str) -> Result<String, String> {
    let file: syn::File =
        syn::parse_str(rust_source).map_err(|e| format!("Parse error: {}", e))?;

    let mut codegen = CodeGen::new();
    codegen.emit_file(&file);
    Ok(codegen.into_output())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_basic() {
        let result = transpile("fn main() { let x = 42; }");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("void main()"));
        assert!(output.contains("const auto x = 42;"));
    }

    #[test]
    fn test_transpile_error() {
        let result = transpile("fn {{{ invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_transpile_multiple_items() {
        let result = transpile(
            r#"
            struct Point { x: f64, y: f64 }
            const PI: f64 = 3.14159;
            fn distance(a: &Point, b: &Point) -> f64 {
                0.0
            }
        "#,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("struct Point"));
        assert!(output.contains("constexpr double PI"));
        assert!(output.contains("double distance"));
    }

    #[test]
    fn test_transpile_complete_program() {
        let result = transpile(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }

            fn main() {
                let result = add(1, 2);
            }
        "#,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("int32_t add(int32_t a, int32_t b)"));
        assert!(output.contains("return a + b;"));
        assert!(output.contains("void main()"));
        assert!(output.contains("add(1, 2)"));
    }
}
