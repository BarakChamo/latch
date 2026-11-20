// Exercises the npm launcher, standalone archive, and bundled plugin entry points.

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const { name, version } = JSON.parse(readFileSync(join(root, "distribution/npm/package.json"), "utf8"));
const [target, destination = "dist"] = process.argv.slice(2);
const archives = resolve(destination);
const npm = process.env.npm_execpath;
assert(target && npm, "Use npm run test:release -- <target> [archive-directory].");

const windows = process.platform === "win32";
const packageStem = name.replace(/^@/, "").replace("/", "-");
const bundle = `latch-${version}-${target}`;
const archive = join(archives, `${bundle}.tar.gz`);
const platformTarball = join(archives, `${packageStem}-${process.platform}-${process.arch}-${version}.tgz`);
const scratch = mkdtempSync(join(tmpdir(), "latch release test "));

try {
  const wrapperName = `${packageStem}-${version}.tgz`;
  let wrapper = join(archives, wrapperName);
  if (!existsSync(wrapper)) {
    execFileSync(process.execPath, [npm, "run", "package:release", "--", "npm", scratch], {
      cwd: root,
      stdio: "inherit",
    });
    wrapper = join(scratch, wrapperName);
  }
  const prefix = join(scratch, "install");
  execFileSync(process.execPath, [npm, "install", "--global", "--offline", "--ignore-scripts", "--prefix",
    prefix, wrapper, platformTarball], { stdio: "inherit" });
  const modules = execFileSync(process.execPath, [npm, "root", "--global", "--prefix", prefix], {
    encoding: "utf8",
  }).trim();
  verifySharedFiles(join(modules, name));

  const bin = windows ? prefix : join(prefix, "bin");
  const env = { ...process.env, PATH: `${bin}${delimiter}${process.env.PATH ?? ""}` };
  const project = join(scratch, "project");
  mkdirSync(project);
  const run = (...args) => spawnSync(
    windows ? "cmd.exe" : join(bin, "latch"),
    windows ? ["/d", "/c", "latch", ...args] : args,
    { cwd: project, env, encoding: "utf8" },
  );
  const installed = run("--version");
  assert.equal(installed.status, 0, installed.stderr);
  assert.equal(installed.stdout.trim(), `latch ${version}`);

  writeFileSync(join(project, "latch.json"), JSON.stringify({
    hooks: { after_edit: { commands: [["node", "-e", "console.log('release smoke check')"]] } },
  }));
  const success = run("run", "after_edit");
  assert.equal(success.status, 0, success.stderr);
  assert.equal(success.stdout, "");
  writeFileSync(join(project, "latch.json"), JSON.stringify({
    commands: [["node", "-e", "console.error('release smoke failure'); process.exit(17)"]],
  }));
  const failure = run();
  assert.equal(failure.status, 1, failure.stderr);
  assert.match(failure.stderr, /release smoke failure/);

  const checksum = readFileSync(`${archive}.sha256`, "utf8").split(/\s+/)[0];
  assert.equal(createHash("sha256").update(readFileSync(archive)).digest("hex"), checksum);
  execFileSync("tar", ["-xzf", archive, "-C", scratch], { stdio: "inherit" });
  const extracted = join(scratch, bundle);
  const binary = join(extracted, "bin", windows ? "latch.exe" : "latch");
  assert.equal(execFileSync(binary, ["--version"], { encoding: "utf8" }).trim(), `latch ${version}`);
  verifySharedFiles(extracted);
  if (!windows) verifyBundledPlugins(extracted, project);
  console.log(`Verified the release packages for ${target}.`);
} finally {
  rmSync(scratch, { recursive: true });
}

function verifySharedFiles(directory) {
  assert.deepEqual(
    readFileSync(join(directory, "latch.schema.json")),
    readFileSync(join(root, "latch.schema.json")),
  );
  for (const catalog of [".agents/plugins", ".claude-plugin", ".github/plugin"]) {
    assert(existsSync(join(directory, catalog, "marketplace.json")), `Missing marketplace ${catalog}`);
  }
  for (const entry of ["extension-hook.mjs", "opencode/index.js", "opencode/v2.js", "pi/index.ts"]) {
    assert(existsSync(join(directory, "plugins", entry)), `Missing plugin file ${entry}`);
  }
}

function verifyBundledPlugins(extracted, project) {
  for (const host of ["opencode", "pi"]) {
    assert(existsSync(join(extracted, "plugins", host, "bin", "latch")), `Missing ${host} bundled executable`);
  }
  const fixtureBin = join(scratch, "harness tools");
  mkdirSync(fixtureBin);
  const env = { ...process.env, PATH: fixtureBin };
  // The installer uses system utilities, while the harness entry points need no CLI on PATH.
  for (const host of ["codex", "copilot"]) {
    writeFileSync(join(fixtureBin, host), "#!/bin/sh\nexit 0\n", { mode: 0o755 });
    execFileSync(join(extracted, "install"), [host], {
      cwd: project,
      env: { ...env, PATH: `${fixtureBin}:/usr/bin:/bin` },
    });
  }

  const smokeMarker = "Bundled check ran";
  writeFileSync(join(project, "latch.json"), JSON.stringify({
    defaults: { on_success: "output" },
    hooks: { after_edit: { commands: [[process.execPath, "-e", `console.log('${smokeMarker}')`]] } },
  }));
  for (const [harness, tool] of [["codex", "apply_patch"], ["claude-code", "Edit"], ["copilot", "edit"]]) {
    const plugin = join(extracted, "plugins", harness);
    const isCopilot = harness === "copilot";
    const hooks = readJson(join(plugin, isCopilot ? "hooks.json" : "hooks/hooks.json")).hooks;
    const command = isCopilot ? hooks.postToolUse[0].bash : hooks.PostToolUse[0].hooks[0].command;
    const input = isCopilot
      ? { sessionId: "smoke", cwd: project, toolName: tool }
      : { hook_event_name: "PostToolUse", session_id: "smoke", turn_id: "turn", cwd: project, tool_name: tool };
    const output = execFileSync("/bin/sh", ["-c", command], {
      cwd: project,
      env: { ...env, PLUGIN_ROOT: plugin, CLAUDE_PLUGIN_ROOT: plugin },
      input: JSON.stringify(input),
      encoding: "utf8",
    });
    assert(output.includes(smokeMarker), `${harness} did not run the bundled hook: ${output}`);

    const mcp = readJson(join(plugin, ".mcp.json")).mcpServers.latch;
    const expand = (value) => value.replace(/\$\{(?:CLAUDE_)?PLUGIN_ROOT\}/g, () => plugin);
    const response = execFileSync(expand(mcp.command), mcp.args, {
      cwd: project,
      env,
      input: JSON.stringify({
        jsonrpc: "2.0", id: 1, method: "initialize",
        params: {
          protocolVersion: "2025-03-26", capabilities: {},
          clientInfo: { name: "release-smoke", version: "1" },
        },
      }) + '\n{"jsonrpc":"2.0","method":"notifications/initialized"}\n',
      encoding: "utf8",
      timeout: 10_000,
    });
    assert(JSON.parse(response).result.capabilities.tools, `${harness} MCP startup failed`);
  }
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}
