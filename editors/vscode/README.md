# RustyCpp VS Code Extension

This extension starts the RustyCpp language server for C and C++ files.

## Local Development

Install dependencies from this directory:

```bash
npm install
```

Then open this directory in VS Code and run the extension host.

By default, the extension compiles `../../tools/rusty-cpp-lsp.cpp` with `c++`
and stores the generated `rusty-cpp-lsp` binary in VS Code's extension storage.
Set `rustyCpp.lsp.serverPath` if you want to use a prebuilt server binary
instead.

Useful settings:

```json
{
  "rustyCpp.lsp.checkerPath": "${workspaceFolder}/target/debug/rusty-cpp-checker",
  "rustyCpp.lsp.includePaths": ["${workspaceFolder}/include"],
  "rustyCpp.lsp.compileCommands": "${workspaceFolder}/build/compile_commands.json"
}
```
