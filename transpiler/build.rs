use std::process::Command;

/// Embed the transpiler's git revision so the parity-matrix module cache can fold
/// it into its env hash — a transpiler change must invalidate stale cached BMIs /
/// objects (the committed source identity; uncommitted edits are additionally
/// covered at runtime by the binary's mtime). Best-effort: outside a git checkout
/// the values fall back to "unknown".
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

    // Re-run when HEAD or the ref it points at moves, so the embedded hash tracks
    // the current commit. Paths are relative to the workspace's .git (one level up
    // from this crate). If they don't exist the directives are simply inert.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
}
