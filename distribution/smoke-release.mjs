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
  verifyChecksum(platformTarball);
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

  const initialized = run("config", "init");
  assert.equal(initialized.status, 0, initialized.stderr);
  const configPath = join(project, "latch.json");
  const starter = readFileSync(configPath, "utf8");
  assert.equal(JSON.parse(starter).defaults.on_failure, "collect");
  const repeated = run("config", "init");
  assert.equal(repeated.status, 1, repeated.stderr);
  assert.equal(readFileSync(configPath, "utf8"), starter);

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

  verifyChecksum(archive);
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

function verifyChecksum(path) {
  const checksum = readFileSync(`${path}.sha256`, "utf8").split(/\s+/)[0];
  assert.equal(createHash("sha256").update(readFileSync(path)).digest("hex"), checksum, path);
}

function verifySharedFiles(directory) {
  assert.deepEqual(
    readFileSync(join(directory, "latch.schema.json")),
    readFileSync(join(root, "latch.schema.json")),
  );
  for (const catalog of [".agents/plugins", ".claude-plugin", ".github/plugin"]) {
    assert(existsSync(join(directory, catalog, "marketplace.json")), `Missing marketplace ${catalog}`);
  }
  for (const entry of ["extension-hook.mjs", "opencode/index.js", "opencode/v2.js", "pi/index.ts", "kimi/kimi.plugin.json"]) {
    assert(existsSync(join(directory, "plugins", entry)), `Missing plugin file ${entry}`);
  }
}

function verifyBundledPlugins(extracted, project) {
  for (const host of ["opencode", "pi", "kimi"]) {
    assert(existsSync(join(extracted, "plugins", host, "bin", "latch")), `Missing ${host} bundled executable`);
  }
  const fixtureBin = join(scratch, "harness tools");
  mkdirSync(fixtureBin);
  const env = { ...process.env, PATH: fixtureBin };
  // The installer uses system utilities, while the harness entry points need no CLI on PATH.
  for (const host of ["codex", "copilot", "openhands", "grok"]) {
    writeFileSync(join(fixtureBin, host), "#!/bin/sh\nexit 0\n", { mode: 0o755 });
    execFileSync(join(extracted, "install"), [host], {
      cwd: project,
      env: { ...env, PATH: `${fixtureBin}:/usr/bin:/bin`, OPENHANDS_PERSISTENCE_DIR: join(scratch, "openhands home") },
    });
  }

  const openhandsHooks = readJson(join(project, ".openhands/hooks.json"));
  assert.match(openhandsHooks.stop[0].hooks[0].command, /\/bin\/latch' hook openhands/);
  const kimi = join(extracted, "plugins", "kimi");
  const kimiManifest = readJson(join(kimi, "kimi.plugin.json"));
  assert.equal(kimiManifest.hooks.find(({ event }) => event === "Stop").command, "./bin/latch hook kimi");
  assert.equal(execFileSync(join(kimi, kimiManifest.mcpServers.latch.command), ["--version"], {
    cwd: kimi, env, encoding: "utf8",
  }).trim(), `latch ${version}`);

  const smokeMarker = "Bundled check ran";
  writeFileSync(join(project, "latch.json"), JSON.stringify({
    defaults: { on_success: "output" },
    hooks: {
      after_edit: { commands: [[process.execPath, "-e", `console.log('${smokeMarker}')`]] },
      turn_end: { when: "always", commands: [[process.execPath, "-e", `console.log('${smokeMarker}')`]] },
    },
  }));
  const stateHome = join(scratch, "harness state");
  const runCommandHook = (command, cwd, input) => execFileSync("/bin/sh", ["-c", command], {
    cwd, env: { ...env, HOME: stateHome, OPENHANDS_PERSISTENCE_DIR: stateHome, KIMI_CODE_HOME: stateHome },
    input: JSON.stringify(input), encoding: "utf8",
  });
  const openHandsEvent = { session_id: "open-hands-smoke", working_dir: project };
  runCommandHook(openhandsHooks.post_tool_use[0].hooks[0].command, project, {
    ...openHandsEvent, event_type: "PostToolUse", tool_name: "file_editor", tool_input: { command: "create" },
  });
  const openHandsReport = runCommandHook(openhandsHooks.user_prompt_submit[0].hooks[0].command, project, {
    ...openHandsEvent, event_type: "UserPromptSubmit",
  });
  assert(JSON.parse(openHandsReport).additionalContext.includes(smokeMarker));
  const kimiEvent = { session_id: "kimi-smoke", cwd: project };
  const runKimiHook = (event) => runCommandHook(kimiManifest.hooks.find((hook) => hook.event === event).command,
    kimi, { ...kimiEvent, hook_event_name: event });
  assert.equal(runKimiHook("UserPromptSubmit"), "");
  assert.equal(runKimiHook("Stop"), "");
  assert(runKimiHook("UserPromptSubmit").includes(smokeMarker));

  const grokHooks = readJson(join(project, ".grok/hooks/latch.json")).hooks;
  const grokReport = runCommandHook(grokHooks.PostToolUse[0].hooks[0].command, project, {
    hook_event_name: "PostToolUse", sessionId: "grok-smoke", session_id: "grok-smoke",
    cwd: project, toolName: "write", tool_name: "write",
  });
  assert(JSON.parse(grokReport).hookSpecificOutput.additionalContext.includes(smokeMarker));

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
