# RustyCpp for VS Code

This extension provides live RustyCpp diagnostics and safety-annotation quick
fixes for C and C++ files.

## Requirements

- A built `rusty-cpp-checker`, configured with
  `rustyCpp.lsp.checkerPath` or available on `PATH`.
- A C++23 compiler. The extension uses `c++` by default to compile its small
  bundled language server.

When developing inside the RustyCpp repository, the extension automatically
looks for `target/debug/rusty-cpp-checker` and
`target/release/rusty-cpp-checker`.

## Run From Source

Build the checker and install the extension dependencies:

```bash
cd ../..
cargo build
cd editors/vscode
npm install
```

Open `editors/vscode` in VS Code and press `F5`. The `Run RustyCpp Extension`
launch configuration opens the repository in an Extension Development Host.
Open a C or C++ file there and check:

- The status bar shows `RustyCpp` with a check icon.
- RustyCpp findings appear as editor squiggles and in the Problems panel.
- `Quick Fix...` on a function offers `Mark function as @safe` and
  `Mark function as @unsafe` when the function has no safety annotation.
- `RustyCpp: Show Language Server Output` shows resolved server, checker, and
  include paths.

Changing a `rustyCpp.lsp` setting automatically restarts the language server.
`RustyCpp: Restart Language Server` is also available from the Command Palette.

## Package

Create an installable VSIX:

```bash
npm run package
```

The packaging step synchronizes `../../tools/rusty-cpp-lsp.cpp` into the
extension. The installed extension compiles that source into VS Code's extension
storage on first use.

Useful settings:

```json
{
  "rustyCpp.lsp.checkerPath": "${workspaceFolder}/target/debug/rusty-cpp-checker",
  "rustyCpp.lsp.includePaths": ["${workspaceFolder}/include"],
  "rustyCpp.lsp.compileCommands": "${workspaceFolder}/build/compile_commands.json"
}
```

Paths may be absolute or relative to the first workspace folder. The
`${workspaceFolder}` and `~` prefixes are supported.
