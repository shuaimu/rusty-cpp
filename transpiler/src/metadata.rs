use serde::Deserialize;
use std::path::{Path, PathBuf};

/// A discovered crate target from `cargo metadata`.
#[derive(Debug, Clone)]
pub struct CrateTarget {
    pub name: String,
    pub kind: TargetKind,
    pub src_path: PathBuf,
    /// C++20 module name derived from target name
    pub module_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TargetKind {
    Lib,
    Bin,
    Test,
    Example,
    Bench,
    Other(String),
}

impl TargetKind {
    fn from_cargo(kinds: &[String]) -> Self {
        for k in kinds {
            match k.as_str() {
                "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro" => {
                    return TargetKind::Lib
                }
                "bin" => return TargetKind::Bin,
                "test" => return TargetKind::Test,
                "example" => return TargetKind::Example,
                "bench" => return TargetKind::Bench,
                _ => {}
            }
        }
        TargetKind::Other(kinds.join(","))
    }

    pub fn is_test_capable(&self) -> bool {
        matches!(self, TargetKind::Lib | TargetKind::Bin | TargetKind::Test)
    }

    pub fn cargo_expand_flag(&self) -> Option<&'static str> {
        match self {
            TargetKind::Lib => Some("--lib"),
            TargetKind::Bin => Some("--bin"),
            TargetKind::Test => Some("--test"),
            TargetKind::Example => Some("--example"),
            _ => None,
        }
    }
}

/// Raw cargo metadata JSON structures (subset).
#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    #[allow(dead_code)]
    version: String,
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    src_path: String,
}

/// Discover crate targets by running `cargo metadata`.
/// Returns the package name and a list of discovered targets.
pub fn discover_targets(
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<(String, Vec<CrateTarget>), String> {
    let project_dir = manifest_path
        .parent()
        .unwrap_or(Path::new("."));

    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest_path)
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo metadata: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed:\n{}", stderr));
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse cargo metadata: {}", e))?;

    // Find the target package
    let pkg = if let Some(filter) = package_filter {
        metadata
            .packages
            .iter()
            .find(|p| p.name == filter)
            .ok_or_else(|| format!("Package '{}' not found in metadata", filter))?
    } else {
        metadata
            .packages
            .first()
            .ok_or_else(|| "No packages found in cargo metadata".to_string())?
    };

    let mut targets = Vec::new();
    let mut skipped = Vec::new();

    for target in &pkg.targets {
        let kind = TargetKind::from_cargo(&target.kind);
        let module_name = format!("{}", target.name.replace('-', "_"));

        if kind.is_test_capable() {
            targets.push(CrateTarget {
                name: target.name.clone(),
                kind,
                src_path: PathBuf::from(&target.src_path),
                module_name,
            });
        } else {
            skipped.push((target.name.clone(), kind));
        }
    }

    // Report skipped targets
    for (name, kind) in &skipped {
        eprintln!(
            "  Skipping target '{}' ({:?}): not test-capable",
            name, kind
        );
    }

    Ok((pkg.name.clone(), targets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_kind_from_cargo() {
        assert_eq!(TargetKind::from_cargo(&["lib".to_string()]), TargetKind::Lib);
        assert_eq!(TargetKind::from_cargo(&["bin".to_string()]), TargetKind::Bin);
        assert_eq!(TargetKind::from_cargo(&["test".to_string()]), TargetKind::Test);
        assert_eq!(
            TargetKind::from_cargo(&["example".to_string()]),
            TargetKind::Example
        );
    }

    #[test]
    fn test_target_kind_test_capable() {
        assert!(TargetKind::Lib.is_test_capable());
        assert!(TargetKind::Bin.is_test_capable());
        assert!(TargetKind::Test.is_test_capable());
        assert!(!TargetKind::Example.is_test_capable());
        assert!(!TargetKind::Bench.is_test_capable());
    }

    #[test]
    fn test_cargo_expand_flag() {
        assert_eq!(TargetKind::Lib.cargo_expand_flag(), Some("--lib"));
        assert_eq!(TargetKind::Bin.cargo_expand_flag(), Some("--bin"));
        assert_eq!(TargetKind::Test.cargo_expand_flag(), Some("--test"));
    }

    #[test]
    fn test_module_name_from_target() {
        let target = CrateTarget {
            name: "my-crate".to_string(),
            kind: TargetKind::Lib,
            src_path: PathBuf::from("src/lib.rs"),
            module_name: "my_crate".to_string(),
        };
        assert_eq!(target.module_name, "my_crate");
    }
}
