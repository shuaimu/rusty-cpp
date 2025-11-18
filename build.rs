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
            // Common Linux LLVM paths
            let llvm_paths = vec![
                "/usr/lib/llvm-14/lib",
                "/usr/lib/llvm-13/lib",
                "/usr/lib/llvm-12/lib",
                "/usr/lib/x86_64-linux-gnu",
                "/usr/local/lib",
            ];

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
