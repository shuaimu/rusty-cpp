use serde::Deserialize;
use std::collections::HashSet;
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
                    return TargetKind::Lib;
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

    fn module_collision_suffix(&self) -> &'static str {
        match self {
            TargetKind::Lib => "lib",
            TargetKind::Bin => "bin",
            TargetKind::Test => "test",
            TargetKind::Example => "example",
            TargetKind::Bench => "bench",
            TargetKind::Other(_) => "target",
        }
    }

    fn sort_rank(&self) -> u8 {
        match self {
            TargetKind::Lib => 0,
            TargetKind::Bin => 1,
            TargetKind::Test => 2,
            TargetKind::Example => 3,
            TargetKind::Bench => 4,
            TargetKind::Other(_) => 5,
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
    manifest_path: PathBuf,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    src_path: String,
}

#[derive(Debug, Clone)]
struct RawTarget {
    name: String,
    kind: TargetKind,
    src_path: PathBuf,
}

fn canonicalized_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn select_target_package<'a>(
    metadata: &'a CargoMetadata,
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<&'a Package, String> {
    if let Some(filter) = package_filter {
        return metadata
            .packages
            .iter()
            .find(|p| p.name == filter)
            .ok_or_else(|| format!("Package '{}' not found in metadata", filter));
    }

    let requested_manifest = canonicalized_path(manifest_path);
    if let Some(pkg) = metadata
        .packages
        .iter()
        .find(|p| canonicalized_path(&p.manifest_path) == requested_manifest)
    {
        return Ok(pkg);
    }

    metadata
        .packages
        .first()
        .ok_or_else(|| "No packages found in cargo metadata".to_string())
}

fn normalize_module_base(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    if out.is_empty() {
        out.push_str("target");
    }

    if out
        .as_bytes()
        .first()
        .is_some_and(|first| first.is_ascii_digit())
    {
        out.insert(0, '_');
    }

    out
}

fn assign_module_names(mut raw_targets: Vec<RawTarget>) -> Vec<CrateTarget> {
    // Keep target processing deterministic so module naming and downstream artifact
    // generation are stable across reruns and environments.
    raw_targets.sort_by(|a, b| {
        a.kind
            .sort_rank()
            .cmp(&b.kind.sort_rank())
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.src_path.cmp(&b.src_path))
    });

    let mut used_module_names: HashSet<String> = HashSet::new();
    let mut targets = Vec::with_capacity(raw_targets.len());

    for raw in raw_targets {
        let base = normalize_module_base(&raw.name);
        let mut module_name = base.clone();

        if used_module_names.contains(&module_name) {
            module_name = format!("{}_{}", base, raw.kind.module_collision_suffix());
        }

        if used_module_names.contains(&module_name) {
            let stem = module_name.clone();
            let mut index = 2u32;
            loop {
                let candidate = format!("{}_{}", stem, index);
                if !used_module_names.contains(&candidate) {
                    module_name = candidate;
                    break;
                }
                index += 1;
            }
        }

        used_module_names.insert(module_name.clone());
        targets.push(CrateTarget {
            name: raw.name,
            kind: raw.kind,
            src_path: raw.src_path,
            module_name,
        });
    }

    targets
}

/// Discover crate targets by running `cargo metadata`.
/// Returns the package name and a list of discovered targets.
pub fn discover_targets(
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<(String, Vec<CrateTarget>), String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));

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

    // Select target package. Without an explicit package filter, prefer the package
    // whose Cargo.toml matches the requested manifest path instead of metadata order.
    let pkg = select_target_package(&metadata, manifest_path, package_filter)?;

    let mut raw_targets = Vec::new();
    let mut skipped = Vec::new();

    for target in &pkg.targets {
        let kind = TargetKind::from_cargo(&target.kind);

        if kind.is_test_capable() {
            raw_targets.push(RawTarget {
                name: target.name.clone(),
                kind,
                src_path: PathBuf::from(&target.src_path),
            });
        } else {
            skipped.push((target.name.clone(), kind));
        }
    }

    let targets = assign_module_names(raw_targets);

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
        assert_eq!(
            TargetKind::from_cargo(&["lib".to_string()]),
            TargetKind::Lib
        );
        assert_eq!(
            TargetKind::from_cargo(&["bin".to_string()]),
            TargetKind::Bin
        );
        assert_eq!(
            TargetKind::from_cargo(&["test".to_string()]),
            TargetKind::Test
        );
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

    #[test]
    fn test_normalize_module_base() {
        assert_eq!(normalize_module_base("cli-tool"), "cli_tool");
        assert_eq!(normalize_module_base("cfg.if"), "cfg_if");
        assert_eq!(normalize_module_base("123name"), "_123name");
    }

    #[test]
    fn test_assign_module_names_handles_normalized_collisions_deterministically() {
        let targets = assign_module_names(vec![
            RawTarget {
                name: "cli-tool".to_string(),
                kind: TargetKind::Bin,
                src_path: PathBuf::from("src/main.rs"),
            },
            RawTarget {
                name: "cli_tool".to_string(),
                kind: TargetKind::Test,
                src_path: PathBuf::from("tests/cli_tool.rs"),
            },
        ]);

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "cli-tool");
        assert_eq!(targets[0].module_name, "cli_tool");
        assert_eq!(targets[1].name, "cli_tool");
        assert_eq!(targets[1].module_name, "cli_tool_test");
    }

    #[test]
    fn test_assign_module_names_prefers_lib_base_name_when_colliding() {
        let targets = assign_module_names(vec![
            RawTarget {
                name: "demo-lib".to_string(),
                kind: TargetKind::Lib,
                src_path: PathBuf::from("src/lib.rs"),
            },
            RawTarget {
                name: "demo_lib".to_string(),
                kind: TargetKind::Test,
                src_path: PathBuf::from("tests/demo_lib.rs"),
            },
            RawTarget {
                name: "demo_lib".to_string(),
                kind: TargetKind::Bin,
                src_path: PathBuf::from("src/main.rs"),
            },
        ]);

        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].module_name, "demo_lib");
        assert_eq!(targets[1].module_name, "demo_lib_bin");
        assert_eq!(targets[2].module_name, "demo_lib_test");
    }

    #[test]
    fn test_select_target_package_prefers_manifest_owner_when_filter_missing() {
        let fixture = tempfile::tempdir().unwrap();
        let root_manifest = fixture.path().join("Cargo.toml");
        let xtask_manifest = fixture.path().join("xtask").join("Cargo.toml");
        std::fs::create_dir_all(xtask_manifest.parent().unwrap()).unwrap();
        std::fs::write(&root_manifest, "[package]\nname = \"root_pkg\"\nversion = \"0.1.0\"\n")
            .unwrap();
        std::fs::write(
            &xtask_manifest,
            "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let metadata = CargoMetadata {
            packages: vec![
                Package {
                    name: "xtask".to_string(),
                    version: "0.0.0".to_string(),
                    targets: vec![],
                    manifest_path: xtask_manifest,
                },
                Package {
                    name: "root_pkg".to_string(),
                    version: "0.1.0".to_string(),
                    targets: vec![],
                    manifest_path: root_manifest.clone(),
                },
            ],
        };

        let selected = select_target_package(&metadata, &root_manifest, None).unwrap();
        assert_eq!(selected.name, "root_pkg");
    }

    #[test]
    fn test_select_target_package_respects_explicit_filter() {
        let fixture = tempfile::tempdir().unwrap();
        let root_manifest = fixture.path().join("Cargo.toml");
        let member_manifest = fixture.path().join("xtask").join("Cargo.toml");
        std::fs::create_dir_all(member_manifest.parent().unwrap()).unwrap();
        std::fs::write(
            &root_manifest,
            "[package]\nname = \"root_pkg\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            &member_manifest,
            "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let metadata = CargoMetadata {
            packages: vec![
                Package {
                    name: "root_pkg".to_string(),
                    version: "0.1.0".to_string(),
                    targets: vec![],
                    manifest_path: root_manifest.clone(),
                },
                Package {
                    name: "xtask".to_string(),
                    version: "0.0.0".to_string(),
                    targets: vec![],
                    manifest_path: member_manifest,
                },
            ],
        };

        let selected = select_target_package(&metadata, &root_manifest, Some("xtask")).unwrap();
        assert_eq!(selected.name, "xtask");
    }

    #[test]
    fn test_discover_targets_prefers_manifest_owner_package_when_workspace_member_precedes_it() {
        let fixture = tempfile::tempdir().unwrap();
        let root_manifest = fixture.path().join("Cargo.toml");
        let root_src = fixture.path().join("src");
        let xtask_src = fixture.path().join("xtask").join("src");
        std::fs::create_dir_all(&root_src).unwrap();
        std::fs::create_dir_all(&xtask_src).unwrap();

        std::fs::write(
            &root_manifest,
            "[package]\nname = \"manifest_owned_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\nmembers = [\"xtask\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        std::fs::write(root_src.join("lib.rs"), "pub fn value() -> i32 { 7 }\n").unwrap();
        std::fs::write(
            fixture.path().join("xtask").join("Cargo.toml"),
            "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(xtask_src.join("main.rs"), "fn main() {}\n").unwrap();

        let (pkg_name, targets) = discover_targets(&root_manifest, None).unwrap();
        assert_eq!(pkg_name, "manifest_owned_fixture");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "manifest_owned_fixture");
        assert_eq!(targets[0].kind, TargetKind::Lib);
    }
}
