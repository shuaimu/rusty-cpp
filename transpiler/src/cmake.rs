use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};

/// Minimal Cargo.toml structure for CMake generation.
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CargoToml {
    pub package: Package,
    #[serde(default)]
    pub lib: Option<LibTarget>,
    #[serde(default, rename = "bin")]
    pub bins: Option<Vec<BinTarget>>,
    #[serde(default)]
    pub dependencies: Option<toml::value::Table>,
    /// Target-qualified dependency tables. Crate mode does not evaluate
    /// Cargo cfg expressions, but reserved runtime identities must still be
    /// visible so validation can fail closed instead of silently bypassing.
    #[serde(default)]
    pub target: Option<toml::value::Table>,
}

/// Information about an external crate dependency.
#[derive(Debug, Clone)]
pub struct CrateDep {
    pub name: String,
    /// Cargo package selected by a renamed dependency (`package = "..."`).
    /// `None` means the dependency key is also the package identity.
    pub package: Option<String>,
    pub version: Option<String>,
    pub path: Option<String>,
    pub is_local: bool,
    pub workspace_inherited: bool,
    pub optional: bool,
    /// Cargo target selector for `[target.<selector>.dependencies]`.
    pub target: Option<String>,
}

/// Extract dependency information from the parsed Cargo.toml.
pub fn extract_dependencies(cargo: &CargoToml) -> Vec<CrateDep> {
    let mut deps = Vec::new();

    if let Some(ref dep_table) = cargo.dependencies {
        extract_dependency_table(dep_table, None, &mut deps);
    }

    if let Some(targets) = &cargo.target {
        for (selector, target_value) in targets {
            let Some(target_table) = target_value.as_table() else {
                continue;
            };
            let Some(dep_table) = target_table
                .get("dependencies")
                .and_then(toml::Value::as_table)
            else {
                continue;
            };
            extract_dependency_table(dep_table, Some(selector), &mut deps);
        }
    }

    deps
}

fn extract_dependency_table(
    dep_table: &toml::value::Table,
    target: Option<&str>,
    deps: &mut Vec<CrateDep>,
) {
    for (name, value) in dep_table {
        match value {
            toml::Value::String(version) => {
                deps.push(CrateDep {
                    name: name.clone(),
                    package: None,
                    version: Some(version.clone()),
                    path: None,
                    is_local: false,
                    workspace_inherited: false,
                    optional: false,
                    target: target.map(str::to_string),
                });
            }
            toml::Value::Table(t) => {
                let package = t
                    .get("package")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let version = t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let path = t
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let workspace_inherited = t
                    .get("workspace")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false);
                let optional = t
                    .get("optional")
                    .and_then(toml::Value::as_bool)
                    .unwrap_or(false);
                let is_local = path.is_some();
                deps.push(CrateDep {
                    name: name.clone(),
                    package,
                    version,
                    path,
                    is_local,
                    workspace_inherited,
                    optional,
                    target: target.map(str::to_string),
                });
            }
            _ => {}
        }
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Package {
    pub name: String,
    #[serde(
        default = "default_version",
        deserialize_with = "deserialize_version_field"
    )]
    pub version: String,
    #[serde(
        default = "default_edition",
        deserialize_with = "deserialize_edition_field"
    )]
    pub edition: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CargoStringField {
    String(String),
    Table(toml::value::Table),
}

fn deserialize_version_field<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<CargoStringField>::deserialize(deserializer)?;
    Ok(match raw {
        Some(CargoStringField::String(value)) if !value.is_empty() => value,
        _ => default_version(),
    })
}

fn deserialize_edition_field<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<CargoStringField>::deserialize(deserializer)?;
    Ok(match raw {
        Some(CargoStringField::String(value)) if !value.is_empty() => value,
        _ => default_edition(),
    })
}

fn default_version() -> String {
    "0.1.0".to_string()
}
fn default_edition() -> String {
    "2021".to_string()
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct LibTarget {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct BinTarget {
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
}

/// Map a Rust source file path to a C++20 module file path and module name.
/// `src/lib.rs` → (`crate_name.cppm`, `crate_name`)
/// `src/main.rs` → (`main.cppm`, `crate_name`)
/// `src/foo.rs` → (`crate_name.foo.cppm`, `crate_name.foo`)
/// `src/bar/mod.rs` → (`crate_name.bar.cppm`, `crate_name.bar`)
/// `src/bar/baz.rs` → (`crate_name.bar.baz.cppm`, `crate_name.bar.baz`)
pub fn map_rs_to_cppm(rs_path: &Path, crate_name: &str) -> (PathBuf, String) {
    let path_str = rs_path.to_string_lossy().replace('\\', "/");

    // Strip src/ prefix if present
    let relative = if let Some(stripped) = path_str.strip_prefix("src/") {
        stripped
    } else {
        &path_str
    };

    // Strip .rs extension
    let without_ext = relative.strip_suffix(".rs").unwrap_or(relative);

    match without_ext {
        "lib" | "main" => {
            let cppm = format!("{}.cppm", crate_name);
            (PathBuf::from(cppm), crate_name.to_string())
        }
        other => {
            // Replace mod.rs with parent directory name
            let normalized = if other.ends_with("/mod") {
                other.strip_suffix("/mod").unwrap_or(other)
            } else {
                other
            };

            // Convert path separators to dots for module name
            let module_name = format!("{}.{}", crate_name, normalized.replace('/', "."));
            let cppm = format!("{}.cppm", module_name);
            (PathBuf::from(cppm), module_name)
        }
    }
}

/// Generate a CMakeLists.txt from a parsed Cargo.toml.
pub fn generate_cmake(cargo: &CargoToml, source_files: &[PathBuf]) -> String {
    let mut out = String::new();
    let name = &cargo.package.name;
    let version = &cargo.package.version;

    // Header
    out.push_str("# Auto-generated by rusty-cpp-transpiler\n");
    out.push_str("# Do not edit manually.\n\n");
    out.push_str("cmake_minimum_required(VERSION 3.28)\n");
    out.push_str(&format!(
        "project({} VERSION {} LANGUAGES CXX)\n\n",
        name, version
    ));
    out.push_str("set(CMAKE_CXX_STANDARD 23)\n");
    out.push_str("set(CMAKE_CXX_STANDARD_REQUIRED ON)\n\n");

    // Include rusty-cpp headers
    out.push_str("# Include rusty-cpp headers\n");
    out.push_str("# Adjust this path to your rusty-cpp installation\n");
    out.push_str("# include_directories(${RUSTY_CPP_INCLUDE_DIR})\n\n");

    // Map source files to module files
    let module_files: Vec<(PathBuf, String)> = source_files
        .iter()
        .map(|f| map_rs_to_cppm(f, name))
        .collect();

    // Library target
    if cargo.lib.is_some() {
        let lib_name = cargo
            .lib
            .as_ref()
            .and_then(|l| l.name.clone())
            .unwrap_or_else(|| name.replace('-', "_"));

        out.push_str(&format!("add_library({}\n", lib_name));
        for (cppm, _) in &module_files {
            out.push_str(&format!("    {}\n", cppm.display()));
        }
        out.push_str(")\n\n");

        out.push_str(&format!(
            "target_sources({} PUBLIC FILE_SET CXX_MODULES FILES\n",
            lib_name
        ));
        for (cppm, _) in &module_files {
            out.push_str(&format!("    {}\n", cppm.display()));
        }
        out.push_str(")\n\n");
    }

    // Binary targets
    if let Some(bins) = &cargo.bins {
        for bin in bins {
            let bin_path = bin.path.as_deref().unwrap_or("src/main.rs");
            let (cppm, _) = map_rs_to_cppm(Path::new(bin_path), name);

            out.push_str(&format!("add_executable({}\n", bin.name));
            out.push_str(&format!("    {}\n", cppm.display()));
            out.push_str(")\n\n");
        }
    } else {
        // Default: if there's a src/main.rs, create a binary target
        let main_path = PathBuf::from("src/main.rs");
        if source_files.contains(&main_path) {
            let (cppm, _) = map_rs_to_cppm(&main_path, name);
            out.push_str(&format!("add_executable({}\n", name));
            out.push_str(&format!("    {}\n", cppm.display()));
            out.push_str(")\n\n");
        }
    }

    out
}

/// Parse a Cargo.toml file and return the parsed structure.
pub fn parse_cargo_toml(path: &Path) -> Result<CargoToml, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    toml::from_str(&content).map_err(|e| format!("Failed to parse Cargo.toml: {}", e))
}

/// Collect all .rs source files under src/.
pub fn collect_source_files(project_dir: &Path) -> Vec<PathBuf> {
    let src_dir = project_dir.join("src");
    if !src_dir.exists() {
        return vec![];
    }

    let mut files = Vec::new();
    collect_rs_files_recursive(&src_dir, &src_dir, &mut files);
    files.sort();
    files
}

/// One Rust source of a crate: the lexical path that gives a module its
/// identity, and the project-relative file whose bytes are that module's
/// source.
///
/// The two differ only where the crate root remaps a module with
/// `#[path = "..."]`. Identity always keeps the conventional `src/...`
/// spelling, so every downstream module path, C++ module name, and
/// diagnostic reads exactly as it would if the file physically lived there.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CrateSource {
    /// Conventional `src/...` path. Module identity and diagnostics only.
    pub identity: PathBuf,
    /// Project-relative file holding this module's bytes. Reads only.
    pub content: PathBuf,
}

impl CrateSource {
    /// A source whose bytes live at its own conventional path.
    fn conventional(path: PathBuf) -> Self {
        Self {
            identity: path.clone(),
            content: path,
        }
    }
}

/// Lift the results of a plain `src/` walk, where every file's bytes live at
/// its own conventional path.
pub fn crate_sources_from_walk(paths: Vec<PathBuf>) -> Vec<CrateSource> {
    paths.into_iter().map(CrateSource::conventional).collect()
}

/// The modules a crate root attaches from outside `src/` with `#[path]`.
pub fn remapped_crate_root_sources(project_dir: &Path) -> Result<Vec<CrateSource>, String> {
    remapped_crate_root_modules(project_dir)
}

/// Every Rust source of a crate, including modules whose crate root remaps
/// them out of `src/` with `#[path = "..."]`.
///
/// A plain directory walk can only find files that physically live under
/// `src/`, so a crate keeping a module's canonical bytes elsewhere is
/// invisible to it. rustc does not work that way: it starts at the crate
/// root and follows each `mod` declaration, honoring `#[path]`. Discovery
/// follows the same rule, or a remapped module is silently dropped from the
/// crate rather than reported.
pub fn collect_crate_sources(project_dir: &Path) -> Result<Vec<CrateSource>, String> {
    let mut sources = crate_sources_from_walk(collect_source_files(project_dir));
    sources.extend(remapped_crate_root_modules(project_dir)?);
    sources.sort();
    for pair in sources.windows(2) {
        if pair[0].identity == pair[1].identity {
            return Err(format!(
                "Rust module {} is claimed by two sources: {} and {}",
                pair[0].identity.display(),
                pair[0].content.display(),
                pair[1].content.display()
            ));
        }
    }
    Ok(sources)
}

/// The conventional library or binary crate root, when the crate has one.
fn conventional_crate_root(project_dir: &Path) -> Option<PathBuf> {
    ["src/lib.rs", "src/main.rs"]
        .into_iter()
        .map(PathBuf::from)
        .find(|relative| project_dir.join(relative).is_file())
}

/// Out-of-line `#[path = "..."] mod name;` declarations at the top level of
/// the crate root.
///
/// rustc resolves such a path against the directory holding the declaring
/// file, so a crate root at `src/lib.rs` resolves against `src/`. Only that
/// one position is followed. A `#[path]` deeper in the module tree, or on an
/// inline module, is left for the module-graph validators to reject:
/// accepting a form discovery does not follow would drop sources silently.
fn remapped_crate_root_modules(project_dir: &Path) -> Result<Vec<CrateSource>, String> {
    let Some(root_relative) = conventional_crate_root(project_dir) else {
        return Ok(Vec::new());
    };
    let root = project_dir.join(&root_relative);
    let source = std::fs::read_to_string(&root)
        .map_err(|error| format!("Error reading {}: {error}", root.display()))?;
    // A crate root that never spells `path` cannot carry the attribute in any
    // form, so skip parsing it. The test is deliberately looser than `#[path`:
    // attributes may contain whitespace, and a pre-filter that missed
    // `#[ path = "..."]` would drop that module silently -- the exact failure
    // this resolution exists to prevent.
    if !source.contains("path") {
        return Ok(Vec::new());
    }
    let file = match syn::parse_file(&source) {
        Ok(file) => file,
        // This walker also runs over dependency crates whose syntax this
        // transpiler never has to understand, and which it otherwise only
        // scans textually. Refuse to guess only when the text shows an
        // attribute we would have had to resolve.
        Err(_) if !source.contains("#[path") => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "could not parse crate root {} while resolving `#[path]` modules: {error}",
                root.display()
            ));
        }
    };
    let root_dir = root_relative
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut remapped = Vec::new();
    for item in &file.items {
        let syn::Item::Mod(item_mod) = item else {
            continue;
        };
        if item_mod.content.is_some() {
            continue;
        }
        let Some(value) = module_path_attribute(&item_mod.attrs)
            .map_err(|error| format!("{} declares `mod {};`: {error}", root.display(), item_mod.ident))?
        else {
            continue;
        };
        let name = item_mod.ident.to_string();
        let content = resolve_module_path(project_dir, &root_dir, &value)
            .map_err(|error| format!("`#[path]` on `mod {name};` in {}: {error}", root.display()))?;
        remapped.push(CrateSource {
            identity: PathBuf::from("src").join(format!("{name}.rs")),
            content,
        });
    }
    Ok(remapped)
}

/// The single `#[path = "..."]` value on a module declaration, when present.
fn module_path_attribute(attrs: &[syn::Attribute]) -> Result<Option<String>, String> {
    let mut found: Option<String> = None;
    for attr in attrs {
        if !attr.path().is_ident("path") {
            continue;
        }
        let syn::Meta::NameValue(name_value) = &attr.meta else {
            return Err("`#[path]` requires a `#[path = \"...\"]` string value".to_string());
        };
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(literal),
            ..
        }) = &name_value.value
        else {
            return Err("`#[path]` requires a `#[path = \"...\"]` string value".to_string());
        };
        if found.is_some() {
            return Err("`#[path]` is declared more than once".to_string());
        }
        found = Some(literal.value());
    }
    Ok(found)
}

/// Resolve a `#[path]` value against the declaring file's directory and
/// return it project-relative.
fn resolve_module_path(
    project_dir: &Path,
    declaring_dir: &Path,
    value: &str,
) -> Result<PathBuf, String> {
    if value.is_empty() || value.contains('\\') {
        return Err(format!(
            "value must be a non-empty forward-slash relative path; found {value:?}"
        ));
    }
    if !value.ends_with(".rs") {
        return Err(format!(
            "value must name a Rust source file; found {value:?}"
        ));
    }
    let relative = Path::new(value);
    if relative.is_absolute() {
        return Err(format!("value must be relative; found {value:?}"));
    }
    let target = project_dir.join(declaring_dir).join(relative);
    let canonical_target = std::fs::canonicalize(&target)
        .map_err(|error| format!("cannot resolve {}: {error}", target.display()))?;
    if !canonical_target.is_file() {
        return Err(format!("{} is not a file", target.display()));
    }
    let canonical_project = std::fs::canonicalize(project_dir).map_err(|error| {
        format!(
            "cannot resolve crate directory {}: {error}",
            project_dir.display()
        )
    })?;
    canonical_target
        .strip_prefix(&canonical_project)
        .map(Path::to_path_buf)
        .map_err(|_| {
            format!(
                "{} escapes the crate at {}",
                target.display(),
                project_dir.display()
            )
        })
}

fn collect_rs_files_recursive(base: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files_recursive(base, &path, files);
            } else if path.extension().is_some_and(|e| e == "rs") {
                // Make relative to project root (include "src/" prefix)
                if let Ok(relative) = path.strip_prefix(base.parent().unwrap_or(base)) {
                    files.push(relative.to_path_buf());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_lib_rs() {
        let (cppm, module) = map_rs_to_cppm(Path::new("src/lib.rs"), "my_crate");
        assert_eq!(cppm, PathBuf::from("my_crate.cppm"));
        assert_eq!(module, "my_crate");
    }

    #[test]
    fn test_map_main_rs() {
        let (cppm, module) = map_rs_to_cppm(Path::new("src/main.rs"), "my_crate");
        assert_eq!(cppm, PathBuf::from("my_crate.cppm"));
        assert_eq!(module, "my_crate");
    }

    #[test]
    fn test_map_submodule() {
        let (cppm, module) = map_rs_to_cppm(Path::new("src/foo.rs"), "my_crate");
        assert_eq!(cppm, PathBuf::from("my_crate.foo.cppm"));
        assert_eq!(module, "my_crate.foo");
    }

    #[test]
    fn test_map_nested_module() {
        let (cppm, module) = map_rs_to_cppm(Path::new("src/bar/baz.rs"), "my_crate");
        assert_eq!(cppm, PathBuf::from("my_crate.bar.baz.cppm"));
        assert_eq!(module, "my_crate.bar.baz");
    }

    #[test]
    fn test_map_mod_rs() {
        let (cppm, module) = map_rs_to_cppm(Path::new("src/bar/mod.rs"), "my_crate");
        assert_eq!(cppm, PathBuf::from("my_crate.bar.cppm"));
        assert_eq!(module, "my_crate.bar");
    }

    #[test]
    fn test_parse_cargo_toml() {
        let toml_str = r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [[bin]]
            name = "hello"
            path = "src/main.rs"
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        assert_eq!(cargo.package.name, "hello");
        assert_eq!(cargo.package.version, "1.0.0");
        assert!(cargo.bins.is_some());
        assert_eq!(cargo.bins.as_ref().unwrap()[0].name, "hello");
    }

    #[test]
    fn test_parse_cargo_toml_workspace_inherited_package_fields() {
        let toml_str = r#"
            [package]
            name = "hello"
            version.workspace = true
            edition.workspace = true
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        assert_eq!(cargo.package.name, "hello");
        assert_eq!(cargo.package.version, "0.1.0");
        assert_eq!(cargo.package.edition, "2021");
    }

    #[test]
    fn test_generate_cmake_binary() {
        let cargo = CargoToml {
            package: Package {
                name: "hello".to_string(),
                version: "1.0.0".to_string(),
                edition: "2021".to_string(),
            },
            lib: None,
            bins: Some(vec![BinTarget {
                name: "hello".to_string(),
                path: Some("src/main.rs".to_string()),
            }]),
            dependencies: None,
            target: None,
        };
        let sources = vec![PathBuf::from("src/main.rs")];
        let cmake = generate_cmake(&cargo, &sources);
        assert!(cmake.contains("project(hello VERSION 1.0.0"));
        assert!(cmake.contains("add_executable(hello"));
        assert!(cmake.contains("hello.cppm"));
    }

    #[test]
    fn test_generate_cmake_library() {
        let cargo = CargoToml {
            package: Package {
                name: "my-lib".to_string(),
                version: "0.2.0".to_string(),
                edition: "2021".to_string(),
            },
            lib: Some(LibTarget {
                name: Some("my_lib".to_string()),
                path: None,
            }),
            bins: None,
            dependencies: None,
            target: None,
        };
        let sources = vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/utils.rs")];
        let cmake = generate_cmake(&cargo, &sources);
        assert!(cmake.contains("project(my-lib VERSION 0.2.0"));
        assert!(cmake.contains("add_library(my_lib"));
        assert!(cmake.contains("my-lib.cppm"));
        assert!(cmake.contains("my-lib.utils.cppm"));
    }

    #[test]
    fn test_extract_dependencies_string_version() {
        let toml_str = r#"
            [package]
            name = "test"
            version = "0.1.0"
            [dependencies]
            serde = "1.0"
            rand = "0.8"
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        let deps = extract_dependencies(&cargo);
        assert_eq!(deps.len(), 2);
        assert!(
            deps.iter()
                .any(|d| d.name == "serde" && d.version.as_deref() == Some("1.0") && !d.is_local)
        );
        assert!(deps.iter().any(|d| d.name == "rand" && !d.is_local));
    }

    #[test]
    fn test_extract_dependencies_table_form() {
        let toml_str = r#"
            [package]
            name = "test"
            version = "0.1.0"
            [dependencies]
            serde = { version = "1.0", features = ["derive"] }
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        let deps = extract_dependencies(&cargo);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "serde");
        assert_eq!(deps[0].version.as_deref(), Some("1.0"));
        assert!(!deps[0].is_local);
    }

    #[test]
    fn test_extract_dependencies_path() {
        let toml_str = r#"
            [package]
            name = "test"
            version = "0.1.0"
            [dependencies]
            my_lib = { path = "../my_lib" }
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        let deps = extract_dependencies(&cargo);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "my_lib");
        assert_eq!(deps[0].package, None);
        assert_eq!(deps[0].path.as_deref(), Some("../my_lib"));
        assert!(deps[0].is_local);
    }

    #[test]
    fn test_extract_dependencies_preserves_renamed_package_identity() {
        let toml_str = r#"
            [package]
            name = "test"
            version = "0.1.0"
            [dependencies]
            runtime = { package = "rusty", path = "../rusty" }
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        let deps = extract_dependencies(&cargo);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "runtime");
        assert_eq!(deps[0].package.as_deref(), Some("rusty"));
        assert_eq!(deps[0].path.as_deref(), Some("../rusty"));
        assert!(deps[0].is_local);
    }

    #[test]
    fn test_extract_dependencies_preserves_workspace_and_target_context() {
        let toml_str = r#"
            [package]
            name = "test"
            version = "0.1.0"

            [dependencies]
            inherited = { workspace = true }

            [target.'cfg(unix)'.dependencies]
            runtime = { package = "rusty", path = "../rusty" }
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        let deps = extract_dependencies(&cargo);
        assert_eq!(deps.len(), 2);
        let inherited = deps
            .iter()
            .find(|dependency| dependency.name == "inherited")
            .unwrap();
        assert!(inherited.workspace_inherited);
        assert!(!inherited.optional);
        assert_eq!(inherited.target, None);
        let runtime = deps
            .iter()
            .find(|dependency| dependency.name == "runtime")
            .unwrap();
        assert_eq!(runtime.package.as_deref(), Some("rusty"));
        assert_eq!(runtime.target.as_deref(), Some("cfg(unix)"));
        assert!(!runtime.workspace_inherited);
    }

    #[test]
    fn test_extract_no_dependencies() {
        let toml_str = r#"
            [package]
            name = "test"
            version = "0.1.0"
        "#;
        let cargo: CargoToml = toml::from_str(toml_str).unwrap();
        let deps = extract_dependencies(&cargo);
        assert!(deps.is_empty());
    }

    /// Build a crate directory from `(relative path, contents)` pairs.
    fn crate_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (relative, contents) in files {
            let path = dir.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, contents).unwrap();
        }
        dir
    }

    #[test]
    fn crate_sources_resolve_a_crate_root_path_remap_to_conventional_identity() {
        let dir = crate_dir(&[
            (
                "src/lib.rs",
                "#[path = \"../base/remapped.rs\"]\npub mod remapped;\npub mod local;\n",
            ),
            ("src/local.rs", "pub fn local() {}\n"),
            ("base/remapped.rs", "pub fn remapped() {}\n"),
        ]);
        let sources = collect_crate_sources(dir.path()).unwrap();
        assert_eq!(
            sources,
            vec![
                CrateSource {
                    identity: PathBuf::from("src/lib.rs"),
                    content: PathBuf::from("src/lib.rs"),
                },
                CrateSource {
                    identity: PathBuf::from("src/local.rs"),
                    content: PathBuf::from("src/local.rs"),
                },
                // Identity stays conventional so `map_rs_to_cppm` and every
                // module-path validator keep the spelling they would have had
                // if the file physically lived under src/.
                CrateSource {
                    identity: PathBuf::from("src/remapped.rs"),
                    content: PathBuf::from("base/remapped.rs"),
                },
            ]
        );
        assert_eq!(
            map_rs_to_cppm(&sources[2].identity, "demo"),
            (PathBuf::from("demo.remapped.cppm"), "demo.remapped".to_string())
        );
    }

    #[test]
    fn crate_sources_reject_a_remap_that_collides_with_a_real_source() {
        let dir = crate_dir(&[
            ("src/lib.rs", "#[path = \"../base/api.rs\"]\npub mod api;\n"),
            ("src/api.rs", "pub fn shadowed() {}\n"),
            ("base/api.rs", "pub fn remapped() {}\n"),
        ]);
        let error = collect_crate_sources(dir.path()).unwrap_err();
        assert!(error.contains("claimed by two sources"), "{error}");
    }

    #[test]
    fn crate_sources_reject_a_remap_that_escapes_the_crate_or_is_missing() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("api.rs"), "pub fn outside() {}\n").unwrap();
        let escaping = crate_dir(&[(
            "src/lib.rs",
            &format!(
                "#[path = {:?}]\npub mod api;\n",
                // A relative climb out of the crate, spelled the way a crate
                // root would have to spell it.
                std::path::Path::new("..")
                    .join("..")
                    .join(outside.path().file_name().unwrap())
                    .join("api.rs")
                    .to_string_lossy()
            ),
        )]);
        // Either resolution fails or the target is outside the crate; both are
        // hard errors rather than a silently skipped module.
        assert!(collect_crate_sources(escaping.path()).is_err());

        let missing = crate_dir(&[("src/lib.rs", "#[path = \"../base/gone.rs\"]\npub mod gone;\n")]);
        let error = collect_crate_sources(missing.path()).unwrap_err();
        assert!(error.contains("cannot resolve"), "{error}");
    }

    #[test]
    fn crate_sources_ignore_path_on_inline_modules_and_crates_without_one() {
        // `#[path]` on an inline module redirects that module's *children*,
        // not its own bytes. Discovery must not invent a source for it; the
        // module-graph validators own rejecting the form.
        let inline = crate_dir(&[(
            "src/lib.rs",
            "#[path = \"elsewhere\"]\npub mod outer { pub fn inline() {} }\n",
        )]);
        assert_eq!(
            collect_crate_sources(inline.path()).unwrap(),
            vec![CrateSource {
                identity: PathBuf::from("src/lib.rs"),
                content: PathBuf::from("src/lib.rs"),
            }]
        );

        let plain = crate_dir(&[
            ("src/lib.rs", "pub mod api;\n"),
            ("src/api.rs", "pub fn api() {}\n"),
        ]);
        let sources = collect_crate_sources(plain.path()).unwrap();
        assert!(
            sources
                .iter()
                .all(|source| source.identity == source.content),
            "a crate with no remap must keep identity and content equal: {sources:?}"
        );
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn crate_sources_resolve_a_whitespace_spelled_path_attribute() {
        // The pre-parse filter must not key on the exact `#[path` spelling:
        // rustc accepts whitespace inside the attribute, and a module missed
        // here would vanish from the crate with no diagnostic at all.
        let dir = crate_dir(&[
            ("src/lib.rs", "#[ path = \"../base/api.rs\" ]\npub mod api;\n"),
            ("base/api.rs", "pub fn api() {}\n"),
        ]);
        let sources = collect_crate_sources(dir.path()).unwrap();
        assert!(
            sources.contains(&CrateSource {
                identity: PathBuf::from("src/api.rs"),
                content: PathBuf::from("base/api.rs"),
            }),
            "{sources:?}"
        );
    }
}
