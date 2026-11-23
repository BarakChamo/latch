#!/usr/bin/env node
// Launches the installed native Latch binary and preserves its output and exit status.

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const { dirname, join } = require("node:path");
const { name, version, optionalDependencies } = require("./package.json");

const platform = `${process.platform}-${process.arch}`;
const executable = process.platform === "win32" ? "latch.exe" : "latch";
let binary = join(__dirname, "bin", platform, executable);

if (optionalDependencies) {
  const platformPackage = `${name}-${platform}`;
  if (Object.hasOwn(optionalDependencies, platformPackage)) {
    try {
      const manifest = require.resolve(`${platformPackage}/package.json`);
      binary = join(dirname(manifest), executable);
    } catch (error) {
      if (error.code !== "MODULE_NOT_FOUND") throw error;
      console.error(`latch: install the optional package ${platformPackage}@${version} for this platform`);
      process.exit(1);
    }
  }
}

if (!existsSync(binary)) {
  console.error(`latch: no binary for ${process.platform}/${process.arch}`);
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
