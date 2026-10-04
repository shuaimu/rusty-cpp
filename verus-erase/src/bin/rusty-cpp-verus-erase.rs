// rusty-cpp-verus-erase: Verus's `verus!` erasure as a helper process for
// `rusty-cpp-transpiler --verus-exec`. See `verus_erase::protocol` for why it
// is a separate process and for the stdin/stdout format.
//
//   rusty-cpp-verus-erase             serve one request from stdin
//   rusty-cpp-verus-erase --version   print the vendored Verus revision

use std::io::{Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => {}
        [flag] if flag == "--version" => {
            println!("{}", verus_erase::protocol::version_line());
            return ExitCode::SUCCESS;
        }
        _ => {
            eprintln!("usage: rusty-cpp-verus-erase [--version]  (request on stdin)");
            return ExitCode::from(2);
        }
    }
    let mut request = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut request) {
        eprintln!("rusty-cpp-verus-erase: could not read stdin: {error}");
        return ExitCode::from(2);
    }
    match verus_erase::protocol::serve(&request) {
        Ok(response) => {
            let mut stdout = std::io::stdout().lock();
            if let Err(error) = stdout.write_all(&response).and_then(|()| stdout.flush()) {
                eprintln!("rusty-cpp-verus-erase: could not write stdout: {error}");
                return ExitCode::from(2);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("rusty-cpp-verus-erase: protocol error: {error}");
            ExitCode::from(2)
        }
    }
}
