#!/usr/bin/env node

const os = require("node:os");
const path = require("node:path");
const { spawn } = require("node:child_process");

const platform = os.platform();
const arch = os.arch();

// Map Node platform/arch to your platform packages
let pkgName;
if (platform === "darwin" && arch === "arm64") {
  pkgName = "@danielealbano/mcp-for-azure-devops-boards-darwin-arm64";
} else if (platform === "linux" && arch === "x64") {
  pkgName = "@danielealbano/mcp-for-azure-devops-boards-linux-x64";
} else if (platform === "linux" && arch === "arm64") {
  pkgName = "@danielealbano/mcp-for-azure-devops-boards-linux-arm64";
} else if (platform === "win32" && arch === "x64") {
  pkgName = "@danielealbano/mcp-for-azure-devops-boards-win32-x64";
} else {
  console.error(`Unsupported platform/arch: ${platform}/${arch}`);
  process.exit(1);
}

let pkgRoot;
try {
  const pkgJsonPath = require.resolve(`${pkgName}/package.json`);
  pkgRoot = path.dirname(pkgJsonPath);
} catch (err) {
  console.error(
    `Could not resolve platform package "${pkgName}". ` +
      `Did installation fail?`,
    err
  );
  process.exit(1);
}

const exeName = platform === "win32"
  ? "mcp-for-azure-devops-boards.exe"
  : "mcp-for-azure-devops-boards";

const binPath = path.join(pkgRoot, "bin", exeName);

const child = spawn(binPath, process.argv.slice(2), { stdio: "inherit" });

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 0);
  }
});
