const fs = require("fs");
const path = require("path");
const cp = require("child_process");
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const restart = vscode.commands.registerCommand("rustyCpp.restartLanguageServer", async () => {
    await stopClient();
    await startClient(context);
  });
  context.subscriptions.push(restart);

  startClient(context).catch((error) => {
    vscode.window.showErrorMessage(`RustyCpp LSP failed to start: ${error.message}`);
  });
}

async function deactivate() {
  await stopClient();
}

async function stopClient() {
  if (!client) {
    return;
  }
  const running = client;
  client = undefined;
  await running.stop();
}

async function startClient(context) {
  const config = vscode.workspace.getConfiguration("rustyCpp.lsp");
  const serverPath = await resolveServerPath(context, config);
  const initializationOptions = buildInitializationOptions(config);

  const serverOptions = {
    run: {
      command: serverPath,
      transport: TransportKind.stdio
    },
    debug: {
      command: serverPath,
      transport: TransportKind.stdio
    }
  };

  const clientOptions = {
    documentSelector: [
      { scheme: "file", language: "cpp" },
      { scheme: "file", language: "c" },
      { scheme: "file", pattern: "**/*.cpp" },
      { scheme: "file", pattern: "**/*.c" },
      { scheme: "file", pattern: "**/*.h" },
      { scheme: "file", pattern: "**/*.hpp" }
    ],
    initializationOptions,
    synchronize: {
      configurationSection: "rustyCpp.lsp"
    }
  };

  client = new LanguageClient("rustyCpp", "RustyCpp", serverOptions, clientOptions);
  context.subscriptions.push(client);
  await client.start();
}

async function resolveServerPath(context, config) {
  const configuredPath = expandWorkspaceVariables(config.get("serverPath", ""));
  if (configuredPath) {
    return configuredPath;
  }

  const repoRoot = path.resolve(context.extensionPath, "..", "..");
  const sourcePath = path.join(repoRoot, "tools", "rusty-cpp-lsp.cpp");
  if (!fs.existsSync(sourcePath)) {
    throw new Error("Set rustyCpp.lsp.serverPath to a built rusty-cpp-lsp binary.");
  }

  await fs.promises.mkdir(context.globalStorageUri.fsPath, { recursive: true });
  const outputPath = path.join(context.globalStorageUri.fsPath, process.platform === "win32" ? "rusty-cpp-lsp.exe" : "rusty-cpp-lsp");

  if (needsRebuild(sourcePath, outputPath)) {
    buildServer(config.get("compilerPath", "c++"), sourcePath, outputPath);
  }

  return outputPath;
}

function needsRebuild(sourcePath, outputPath) {
  if (!fs.existsSync(outputPath)) {
    return true;
  }
  return fs.statSync(sourcePath).mtimeMs > fs.statSync(outputPath).mtimeMs;
}

function buildServer(compilerPath, sourcePath, outputPath) {
  try {
    cp.execFileSync(
      compilerPath,
      ["-std=c++23", "-Wall", "-Wextra", "-pedantic", sourcePath, "-o", outputPath],
      { stdio: "pipe" }
    );
  } catch (error) {
    const stderr = error.stderr ? error.stderr.toString() : error.message;
    throw new Error(`failed to build rusty-cpp-lsp with ${compilerPath}: ${stderr}`);
  }
  if (process.platform !== "win32") {
    fs.chmodSync(outputPath, 0o755);
  }
}

function buildInitializationOptions(config) {
  const checkerPath = expandWorkspaceVariables(config.get("checkerPath", ""));
  const compileCommands = expandWorkspaceVariables(config.get("compileCommands", ""));
  const includePaths = config.get("includePaths", []).map(expandWorkspaceVariables).filter(Boolean);
  const defines = config.get("defines", []).filter(Boolean);

  return {
    checkerPath,
    includePaths,
    defines,
    compileCommands
  };
}

function expandWorkspaceVariables(value) {
  if (!value) {
    return "";
  }

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? "";
  if (!workspaceFolder && value.includes("${workspaceFolder}")) {
    return "";
  }
  return value.replaceAll("${workspaceFolder}", workspaceFolder);
}

module.exports = {
  activate,
  deactivate
};
