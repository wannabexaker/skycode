#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const { spawn } = require("child_process");

const exeName = process.platform === "win32" ? "sky.exe" : "sky";
const binPath = path.join(__dirname, "..", "npm", "bin", exeName);

if (!fs.existsSync(binPath)) {
  console.error("skycode: binary not found.");
  console.error("Run: npm rebuild skycode");
  process.exit(1);
}

const child = spawn(binPath, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: false
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code == null ? 1 : code);
});
