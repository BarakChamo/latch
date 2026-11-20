#!/usr/bin/env node
// Launches the installed native Latch binary and preserves its output and exit status.

const { spawnSync } = require("node:child_process");
const { dirname, join } = require("node:path");
const { name, version, optionalDependencies } = require("./package.json");

const platformPackage = `${name}-${process.platform}-${process.arch}`;
if (!Object.hasOwn(optionalDependencies ?? {}, platformPackage)) {
  console.error(`latch: unsupported platform ${process.platform}/${process.arch}`);
  process.exit(1);
}

let binary;
try {
  const manifest = require.resolve(`${platformPackage}/package.json`);
  binary = join(dirname(manifest), process.platform === "win32" ? "latch.exe" : "latch");
} catch (error) {
  if (error.code !== "MODULE_NOT_FOUND") throw error;
  console.error(`latch: install the optional package ${platformPackage}@${version} for this platform`);
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`latch: ${result.error.message}`);
  process.exitCode = 1;
} else if (result.signal) {
  process.kill(process.pid, result.signal);
} else {
  process.exitCode = result.status;
}
