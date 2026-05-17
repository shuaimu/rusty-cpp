use std::env;
use std::path::Path;

fn main() {
    // Get the target OS
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    // Platform-specific configuration
    match target_os.as_str() {
        "macos" => {
            // Find LLVM libraries
            let llvm_paths = vec![
                "/opt/homebrew/Cellar/llvm/19.1.7/lib",
                "/opt/homebrew/lib",
                "/usr/local/lib",
            ];

            for path in &llvm_paths {
                if Path::new(path).exists() {
                    println!("cargo:rustc-link-search=native={}", path);

                    // Use @rpath for macOS to make binaries relocatable
                    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
                    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path);
                    break;
                }
            }

            // Link against libclang dynamically with fallback paths
            println!("cargo:rustc-link-lib=dylib=clang");
        }
        "linux" => {
            // Prefer LIBCLANG_PATH when set (standard clang-sys convention).
            // The hardcoded fallback list trails behind so out-of-tree LLVM
            // installs (Homebrew/Linuxbrew, custom builds at $HOME/...) win
            // over a stale /usr/lib/llvm-N that ships an older libclang
            // missing symbols (e.g. clang_CXXMethod_isDeleted needs LLVM 16+).
            let mut llvm_paths: Vec<String> = Vec::new();
            if let Ok(libclang_path) = env::var("LIBCLANG_PATH") {
                llvm_paths.push(libclang_path);
            }
            llvm_paths.extend([
                "/home/linuxbrew/.linuxbrew/opt/llvm/lib".to_string(),
                "/usr/lib/llvm-22/lib".to_string(),
                "/usr/lib/llvm-21/lib".to_string(),
                "/usr/lib/llvm-20/lib".to_string(),
                "/usr/lib/llvm-19/lib".to_string(),
                "/usr/lib/llvm-18/lib".to_string(),
                "/usr/lib/llvm-17/lib".to_string(),
                "/usr/lib/llvm-16/lib".to_string(),
                "/usr/lib/llvm-14/lib".to_string(),
                "/usr/lib/llvm-13/lib".to_string(),
                "/usr/lib/llvm-12/lib".to_string(),
                "/usr/lib/x86_64-linux-gnu".to_string(),
                "/usr/local/lib".to_string(),
            ]);

            for path in &llvm_paths {
                if Path::new(path).exists() {
                    println!("cargo:rustc-link-search=native={}", path);
                    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path);
                    break;
                }
            }
        }
        "windows" => {
            // Windows typically uses PATH for DLL resolution
            // Add common LLVM installation paths
            if let Ok(llvm_path) = env::var("LLVM_PATH") {
                println!("cargo:rustc-link-search=native={}/lib", llvm_path);
            }
        }
        _ => {}
    }

    // Rerun if environment changes
    println!("cargo:rerun-if-env-changed=LLVM_PATH");
    println!("cargo:rerun-if-env-changed=LIBCLANG_PATH");
}
