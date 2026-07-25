const fs = require("fs");
const path = require("path");

const extensionRoot = path.resolve(__dirname, "..");
const repositoryRoot = path.resolve(extensionRoot, "..", "..");
const sourcePath = path.join(repositoryRoot, "tools", "rusty-cpp-lsp.cpp");
const outputDirectory = path.join(extensionRoot, "server");
const outputPath = path.join(outputDirectory, "rusty-cpp-lsp.cpp");
const licenseSourcePath = path.join(repositoryRoot, "LICENSE");
const licenseOutputPath = path.join(extensionRoot, "LICENSE");

if (!fs.existsSync(sourcePath)) {
  throw new Error(`RustyCpp LSP source not found at ${sourcePath}`);
}
if (!fs.existsSync(licenseSourcePath)) {
  throw new Error(`RustyCpp license not found at ${licenseSourcePath}`);
}

fs.mkdirSync(outputDirectory, { recursive: true });
fs.copyFileSync(sourcePath, outputPath);
fs.copyFileSync(licenseSourcePath, licenseOutputPath);
console.log(`Synchronized ${path.relative(extensionRoot, outputPath)}`);
console.log(`Synchronized ${path.relative(extensionRoot, licenseOutputPath)}`);
