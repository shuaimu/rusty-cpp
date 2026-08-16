use std::process::Command;
use std::{env, path::PathBuf};

/// Embed the transpiler's git revision so the parity-matrix module cache can fold
/// it into its env hash — a transpiler change must invalidate stale cached BMIs /
/// objects. The same revision and dirty state are exposed by `--build-info`.
/// Best-effort: outside a git checkout the values fall back to "unknown".
fn main() {
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let hash = git(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    println!("cargo:rustc-env=RUSTY_CPP_GIT_HASH={hash}");
    println!("cargo:rustc-env=RUSTY_CPP_GIT_DIRTY={dirty}");

    // Ask Git for its real administrative paths: a submodule or linked worktree
    // does not keep HEAD in the workspace's `../.git/HEAD`.
    for git_path in ["HEAD", "refs/heads", "packed-refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", git_path]).map(PathBuf::from) {
            let absolute = if path.is_absolute() {
                path
            } else if let Ok(current_dir) = env::current_dir() {
                current_dir.join(path)
            } else {
                continue;
            };
            let resolved = absolute.canonicalize().unwrap_or(absolute);
            if resolved.exists() {
                println!("cargo:rerun-if-changed={}", resolved.display());
            }
        }
    }

    // Recompute the dirty bit whenever transpiler source changes.
    println!("cargo:rerun-if-changed=src");
}
