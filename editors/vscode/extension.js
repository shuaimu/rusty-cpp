const fs = require("fs");
const path = require("path");
const cp = require("child_process");
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;
let outputChannel;
let statusBarItem;
let lifecycle = Promise.resolve();

function activate(context) {
  outputChannel = vscode.window.createOutputChannel("RustyCpp", { log: true });
  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 10);
  statusBarItem.command = "rustyCpp.showOutput";
  statusBarItem.tooltip = "Show RustyCpp language server output";
  statusBarItem.show();

  const restart = vscode.commands.registerCommand("rustyCpp.restartLanguageServer", async () => {
    await queueRestart(context);
  });
  const showOutput = vscode.commands.registerCommand("rustyCpp.showOutput", () => {
    outputChannel.show(true);
  });
  const configurationChanged = vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration("rustyCpp.lsp")) {
      queueRestart(context);
    }
  });

  context.subscriptions.push(
    outputChannel,
    statusBarItem,
    restart,
    showOutput,
    configurationChanged
  );

  return queueRestart(context);
}

async function deactivate() {
  await lifecycle.catch(() => {});
  await stopClient();
}

function queueRestart(context) {
  lifecycle = lifecycle
    .catch(() => {})
    .then(async () => {
      await stopClient();
      await startClient(context);
    })
    .catch(async (error) => {
      setStatus("error", error.message);
      outputChannel.appendLine(`failed to start: ${error.stack ?? error.message}`);
      outputChannel.show(true);
      const selection = await vscode.window.showErrorMessage(
        `RustyCpp LSP failed to start: ${error.message}`,
        "Open Output",
        "Open Settings"
      );
      if (selection === "Open Output") {
        outputChannel.show(true);
      } else if (selection === "Open Settings") {
        await vscode.commands.executeCommand(
          "workbench.action.openSettings",
          "rustyCpp.lsp"
        );
      }
    });
  return lifecycle;
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
  setStatus("starting");
  const configurationResource =
    vscode.window.activeTextEditor?.document.uri ??
    vscode.workspace.workspaceFolders?.[0]?.uri;
  const windowConfig = vscode.workspace.getConfiguration("rustyCpp.lsp");
  const resourceConfig = vscode.workspace.getConfiguration(
    "rustyCpp.lsp",
    configurationResource
  );
  const repoRoot = path.resolve(context.extensionPath, "..", "..");
  const serverPath = await resolveServerPath(context, windowConfig);
  const initializationOptions = buildInitializationOptions(resourceConfig, repoRoot);

  outputChannel.appendLine("");
  outputChannel.appendLine(`starting rusty-cpp-lsp: ${serverPath}`);
  outputChannel.appendLine(`checker path: ${initializationOptions.checkerPath || "<PATH lookup>"}`);
  outputChannel.appendLine(`include paths: ${initializationOptions.includePaths.join(", ") || "<none>"}`);
  if (initializationOptions.compileCommands) {
    outputChannel.appendLine(`compile commands: ${initializationOptions.compileCommands}`);
  }

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
    },
    middleware: {
      handleDiagnostics(uri, diagnostics, next) {
        outputChannel.appendLine(
          `diagnostics: ${uri.fsPath || uri.toString()} (${diagnostics.length})`
        );
        return next(uri, diagnostics);
      }
    },
    outputChannel,
    traceOutputChannel: outputChannel
  };

  const nextClient = new LanguageClient("rustyCpp", "RustyCpp", serverOptions, clientOptions);
  await nextClient.start();
  client = nextClient;
  setStatus("ready");
}

async function resolveServerPath(context, config) {
  const configuredPath = resolveConfiguredPath(config.get("serverPath", ""));
  if (configuredPath) {
    requireFile(configuredPath, "language server");
    return configuredPath;
  }

  const repoRoot = path.resolve(context.extensionPath, "..", "..");
  const sourcePath = [
    path.join(context.extensionPath, "server", "rusty-cpp-lsp.cpp"),
    path.join(repoRoot, "tools", "rusty-cpp-lsp.cpp")
  ].find(fs.existsSync);
  if (!sourcePath) {
    throw new Error(
      "The bundled LSP source is missing. Set rustyCpp.lsp.serverPath to a built rusty-cpp-lsp binary."
    );
  }

  await fs.promises.mkdir(context.globalStorageUri.fsPath, { recursive: true });
  const outputPath = path.join(context.globalStorageUri.fsPath, process.platform === "win32" ? "rusty-cpp-lsp.exe" : "rusty-cpp-lsp");

  if (needsRebuild(sourcePath, outputPath)) {
    const compilerPath = resolveConfiguredPath(config.get("compilerPath", "c++"));
    buildServer(compilerPath, sourcePath, outputPath);
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
  const compilerArgs = [
    "-std=c++23",
    "-Wall",
    "-Wextra",
    "-pedantic",
    sourcePath,
    "-o",
    outputPath
  ];
  if (process.platform !== "win32") {
    compilerArgs.unshift("-pthread");
  }

  try {
    cp.execFileSync(
      compilerPath,
      compilerArgs,
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

function buildInitializationOptions(config, repoRoot) {
  const checkerPath = resolveCheckerPath(config, repoRoot);
  const compileCommands = resolveConfiguredPath(config.get("compileCommands", ""));
  const includePaths = config.get("includePaths", []).map(resolveConfiguredPath).filter(Boolean);
  const defines = config.get("defines", []).filter(Boolean);

  if (compileCommands) {
    requireFile(compileCommands, "compile commands file");
  }

  return {
    checkerPath,
    includePaths,
    defines,
    compileCommands
  };
}

function resolveCheckerPath(config, repoRoot) {
  const configuredPath = resolveConfiguredPath(config.get("checkerPath", ""));
  if (configuredPath) {
    requireFile(configuredPath, "RustyCpp checker");
    return configuredPath;
  }

  const binaryName = process.platform === "win32"
    ? "rusty-cpp-checker.exe"
    : "rusty-cpp-checker";
  const workspaceRoots = (vscode.workspace.workspaceFolders ?? [])
    .map((folder) => folder.uri.fsPath);
  const searchRoots = [...new Set([...workspaceRoots, repoRoot])];

  for (const root of searchRoots) {
    for (const profile of ["debug", "release"]) {
      const candidate = path.join(root, "target", profile, binaryName);
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  }

  return "";
}

function resolveConfiguredPath(value) {
  if (!value) {
    return "";
  }

  const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? "";
  if (!workspaceFolder && value.includes("${workspaceFolder}")) {
    return "";
  }
  const expanded = value
    .replaceAll("${workspaceFolder}", workspaceFolder)
    .replace(/^~(?=$|\/|\\)/, process.env.HOME ?? "~");
  if (path.isAbsolute(expanded) || !workspaceFolder || !expanded.includes(path.sep)) {
    return expanded;
  }
  return path.resolve(workspaceFolder, expanded);
}

function requireFile(filePath, description) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`${description} not found at ${filePath}`);
  }
}

function setStatus(state, detail = "") {
  if (!statusBarItem) {
    return;
  }
  if (state === "starting") {
    statusBarItem.text = "$(sync~spin) RustyCpp";
    statusBarItem.tooltip = "RustyCpp language server is starting";
  } else if (state === "ready") {
    statusBarItem.text = "$(check) RustyCpp";
    statusBarItem.tooltip = "RustyCpp language server is running";
  } else {
    statusBarItem.text = "$(error) RustyCpp";
    statusBarItem.tooltip = `RustyCpp language server failed: ${detail}`;
  }
}

module.exports = {
  activate,
  deactivate
};
