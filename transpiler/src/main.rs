use clap::Parser;
use std::path::PathBuf;
use std::process;

mod codegen;
mod transpile;
mod types;

#[derive(Parser)]
#[command(name = "rusty-cpp-transpiler")]
#[command(about = "Transpile Rust source code to C++ using rusty-cpp types")]
struct Cli {
    /// Input Rust source file (.rs)
    input: PathBuf,

    /// Output C++ module file (.cppm)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// C++20 module name (e.g., "my_crate" or "my_crate.submodule")
    #[arg(short, long)]
    module_name: Option<String>,

    /// Expand macros before transpilation (requires cargo-expand)
    #[arg(long)]
    expand: bool,
}

fn main() {
    let cli = Cli::parse();

    let input_path = &cli.input;
    if !input_path.exists() {
        eprintln!("Error: input file '{}' not found", input_path.display());
        process::exit(1);
    }

    let output_path = cli.output.unwrap_or_else(|| {
        let mut p = input_path.clone();
        p.set_extension("cppm");
        p
    });

    let source = match std::fs::read_to_string(input_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", input_path.display(), e);
            process::exit(1);
        }
    };

    let cpp_output = match transpile::transpile(&source, cli.module_name.as_deref()) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Transpilation error: {}", e);
            process::exit(1);
        }
    };

    match std::fs::write(&output_path, &cpp_output) {
        Ok(()) => {
            println!(
                "Transpiled {} -> {}",
                input_path.display(),
                output_path.display()
            );
        }
        Err(e) => {
            eprintln!("Error writing '{}': {}", output_path.display(), e);
            process::exit(1);
        }
    }
}
