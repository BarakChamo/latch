// Checks bundled install, edit feedback, and MCP through real harness sessions.

import assert from "node:assert/strict";
import { execFileSync, spawn, execFile } from "node:child_process";
import {
  chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync,
  readdirSync, rmSync, writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { parseArgs, promisify } from "node:util";

const exec = promisify(execFile);
const harnesses = ["codex", "claude", "copilot"];
const { positionals, values: { harness, help, marketplace } } = parseArgs({
  allowPositionals: true,
  options: {
    harness: { type: "string", multiple: true },
    help: { type: "boolean" },
    marketplace: { type: "boolean" },
  },
});
const selected = harness ?? harnesses;
const usage = "Usage: npm run test:live -- <local-release.tar.gz> [--harness codex|claude|copilot] [--marketplace]";
if (help) {
  console.log(usage);
  process.exit(0);
}
assert(positionals.length === 1 && selected.every((name) => harnesses.includes(name)), usage);
const [archive] = positionals;

const prompt = "Use your native edit tool to replace ready in demo.txt with changed. Leave the edit in place. Then call the Latch run_turn_end MCP tool in this project once and report its output. Do not run shell commands yourself.";
const firstFailure = "LATCH_LIVE_FIRST_FAILURE";
const secondFailure = "LATCH_LIVE_SECOND_FAILURE";
const toolOutput = "LATCH_LIVE_TOOL_RAN";
const omittedOutput = "LATCH_LIVE_RETRIEVAL_MARKER";
const scratch = mkdtempSync(join(tmpdir(), "latch live smoke "));
const environment = { ...process.env };
const marketplaceName = marketplace ? "latch-tools" : "latch-local";

try {
  execFileSync("tar", ["-xzf", resolve(archive), "-C", scratch]);
  const roots = readdirSync(scratch);
  assert.equal(roots.length, 1, "Expected one directory in the release archive");
  const bundle = join(scratch, roots[0]);
  assert(existsSync(join(bundle, "install")), "Expected a macOS or Linux archive with the bundled installer");
  const guard = join(scratch, "guard");
  mkdirSync(guard);
  writeFileSync(join(guard, "latch"), "#!/bin/sh\nexit 97\n", { mode: 0o755 });
  environment.PATH = [guard, environment.PATH ?? ""].join(delimiter);
  environment.XDG_STATE_HOME = join(scratch, "state");

  const names = [...new Set(selected)];
  const results = await Promise.allSettled(names.map(async (name) => {
    const fixture = prepareFixture(name, bundle);
    const events = await { codex: runCodex, claude: runClaude, copilot: runCopilot }[name](fixture);
    assert.equal(readFileSync(join(fixture.project, "demo.txt"), "utf8"), "changed\n", `${name} did not edit the fixture`);
    const log = join(fixture.project, "checks.log");
    assert(existsSync(log), `${name} did not run Latch`);
    const checkOrder = readFileSync(log, "utf8");
    assert.equal(checkOrder, "edit-first\nedit-second\ntool\n", `${name} must run each check exactly once, in order. Observed ${JSON.stringify(checkOrder)}`);
    assert(events.includes("Latch checks failed."), `${name} did not surface the edit failure`);
    for (const diagnostic of [firstFailure, secondFailure]) {
      assert(events.includes(diagnostic), `${name} dropped collected diagnostic ${diagnostic}`);
    }
    const outputId = events.match(/Saved log: [^\r\n]*?(r-[A-Za-z0-9]{24})\.log/)?.[1];
    assert(outputId, `${name} did not surface the saved output ID`);
    assert(!events.includes(omittedOutput), `${name} included raw output in its preview`);
    const original = execFileSync(join(bundle, "bin", "latch"), [
      "output", outputId, "--grep", omittedOutput,
    ], { cwd: fixture.project, env: fixture.env, encoding: "utf8" });
    assert.equal(original.trim(), omittedOutput, `${name} could not retrieve the omitted line`);
    assert(events.includes(toolOutput), `${name} did not surface the MCP result`);
    console.log(`${name}: bundled install, edit feedback and MCP passed`);
  }));
  for (const [index, result] of results.entries()) {
    if (result.status === "rejected") {
      // Avoid printing vendor event streams or environment values in failure logs.
      console.error(`${names[index]}: ${result.reason.message.split("\n")[0]}`);
      process.exitCode = 1;
    }
  }
} finally {
  rmSync(scratch, { recursive: true });
}

function prepareFixture(name, bundle) {
  const root = join(scratch, name);
  const project = join(root, "project");
  const profile = join(root, "profile");
  mkdirSync(project, { recursive: true });
  mkdirSync(profile, { mode: 0o700 });
  writeFileSync(join(project, "demo.txt"), "ready\n");
  writeFileSync(join(project, ".gitignore"), "checks.log\n");
  writeFileSync(join(project, "latch.json"), JSON.stringify({
    hooks: {
      after_edit: {
        failure_output: name === "codex" ? "tail" : "compact",
        tail_lines: 3,
        commands: [
          ["sh", "-c", `echo edit-first >> checks.log; echo ${omittedOutput}; yes 'passing test' | head -500; echo 'error: ${firstFailure}'; exit 1`],
          ["sh", "-c", `echo edit-second >> checks.log; echo ${secondFailure} >&2; git diff --exit-code HEAD -- demo.txt`],
        ],
      },
      // This entry runs via the tool only because the edit does not match its filter.
      turn_end: {
        paths: ["no-file-matches-this"],
        on_success: "output",
        commands: [["sh", "-c", `echo tool >> checks.log; echo ${toolOutput}`]],
      },
    },
  }));
  for (const args of [
    ["init", "-q"],
    ["add", "."],
    ["-c", "user.name=Latch Smoke", "-c", "user.email=latch@example.test",
      "-c", "commit.gpgsign=false", "commit", "-qm", "Create test project"],
  ]) {
    execFileSync("git", args, { cwd: project, timeout: 15_000 });
  }
  return { name, bundle, project, profile, env: { ...environment } };
}

async function install(fixture, env) {
  const options = { cwd: fixture.project, env, timeout: 45_000 };
  if (!marketplace) {
    await exec(join(fixture.bundle, "install"), [fixture.name], options);
    return;
  }

  const host = fixture.name;
  const marketplaceSource = {
    codex: ["BarakChamo/latch", "--ref", "codex/marketplace"],
    claude: ["BarakChamo/latch#codex/marketplace"],
    copilot: ["https://github.com/BarakChamo/latch.git#codex/marketplace"],
  };
  await exec(host, ["plugin", "marketplace", "add", ...marketplaceSource[host]], options);
  await exec(host, ["plugin", host === "codex" ? "add" : "install", "latch@latch-tools"], options);
}

async function runClaude(fixture) {
  const env = { ...fixture.env, CLAUDE_CONFIG_DIR: fixture.profile };
  if (marketplace) {
    const auth = join(environment.CLAUDE_CONFIG_DIR || join(homedir(), ".claude"), ".credentials.json");
    if (existsSync(auth)) {
      const destination = join(fixture.profile, ".credentials.json");
      copyFileSync(auth, destination);
      chmodSync(destination, 0o600);
    }
  }
  await install(fixture, env);
  const cache = join(fixture.profile, "plugins", "cache", marketplace ? "latch-tools" : "latch-marketplace", "latch");
  const versions = readdirSync(cache);
  assert.equal(versions.length, 1, "Expected one Claude Code plugin version");
  const activation = marketplace ? [] : ["--plugin-dir", join(cache, versions[0])];
  const { stdout } = await exec("claude", [
    "--print", "--verbose", "--output-format", "stream-json", "--include-hook-events",
    "--no-session-persistence", "--setting-sources", marketplace ? "user,project" : "project",
    "--permission-mode", "acceptEdits", "--tools", "Read,Edit,Write",
    "--allowedTools", "mcp__plugin_latch_latch__run_turn_end",
    ...activation, "--", prompt,
  ], { cwd: fixture.project, env: marketplace ? env : fixture.env, timeout: 90_000, maxBuffer: 8_000_000 });
  return stdout;
}

async function runCopilot(fixture) {
  const env = { ...fixture.env, COPILOT_HOME: fixture.profile, COPILOT_CACHE_HOME: join(fixture.profile, "cache") };
  await install(fixture, env);
  const { stdout } = await exec("copilot", [
    "--no-auto-update", "--no-color", "--no-custom-instructions", "--disable-builtin-mcps",
    "--output-format", "json", "--allow-tool=write", "--allow-tool=latch(run_turn_end)",
    "-C", fixture.project, "-p", prompt,
  ], { cwd: fixture.project, env, timeout: 90_000, maxBuffer: 8_000_000 });
  return stdout;
}

async function runCodex(fixture) {
  const callerHome = environment.CODEX_HOME || join(homedir(), ".codex");
  const auth = join(callerHome, "auth.json");
  if (existsSync(auth)) {
    const copy = join(fixture.profile, "auth.json");
    copyFileSync(auth, copy);
    chmodSync(copy, 0o600);
  }
  const env = { ...fixture.env, CODEX_HOME: fixture.profile };
  await install(fixture, env);
  const project = execFileSync("pwd", ["-P"], { cwd: fixture.project, encoding: "utf8" }).trim();
  const { hooks } = await withCodex(fixture, env, async ({ call }) => {
    const { data } = await call("hooks/list", { cwds: [project] });
    return data[0];
  });
  assert(hooks.length, "Codex did not discover bundled hooks");
  const config = join(fixture.profile, "config.toml");
  const trusted = hooks.map(({ key, currentHash }) =>
    `[hooks.state.${JSON.stringify(key)}]\ntrusted_hash = ${JSON.stringify(currentHash)}\n`);
  writeFileSync(config, [
    readFileSync(config, "utf8"),
    `[projects.${JSON.stringify(project)}]\ntrust_level = "trusted"`,
    ...trusted,
    `[plugins."latch@${marketplaceName}".mcp_servers.latch]\ndefault_tools_approval_mode = "approve"`,
  ].join("\n\n"));
  return withCodex(fixture, env, async ({ call, receive, send }) => {
    const { thread } = await call("thread/start", {
      cwd: project, sandbox: "workspace-write", approvalPolicy: "on-request",
    });
    const events = [];
    await call("turn/start", {
      threadId: thread.id,
      input: [{
        type: "text", text_elements: [],
        text: `${prompt}\nUse this exact edit:\n*** Begin Patch\n*** Update File: demo.txt\n@@\n-ready\n+changed\n*** End Patch`,
      }],
    }, events);
    for (;;) {
      const message = await receive();
      if (message.method && message.id !== undefined) {
        // Only approve the fixture's Latch tool. Never approve shell permissions.
        const latchRequest = JSON.stringify(message.params ?? {}).includes("run_turn_end");
        if (latchRequest && message.method === "mcpServer/elicitation/request") {
          send({ id: message.id, result: { action: "accept", content: {} } });
        } else if (latchRequest && message.method === "item/tool/requestUserInput") {
          const answers = Object.fromEntries(message.params.questions.map(({ id }) => [id, { answers: ["Allow"] }]));
          send({ id: message.id, result: { answers } });
        } else {
          send({ id: message.id, result: { decision: "decline" } });
        }
      }
      events.push(message);
      if (message.method === "turn/completed") {
        assert.equal(message.params.turn.status, "completed", "Codex turn did not complete");
        return JSON.stringify(events);
      }
    }
  });
}

async function withCodex(fixture, env, action) {
  const child = spawn("codex", ["app-server"], {
    cwd: fixture.project, env, stdio: ["pipe", "pipe", "ignore"], timeout: 90_000,
    killSignal: "SIGKILL",
  });
  const exited = new Promise((done) => child.once("close", done));
  let spawnError;
  child.on("error", (error) => { spawnError = error; });
  child.stdin.on("error", (error) => { spawnError = error; });
  const lines = createInterface({ input: child.stdout })[Symbol.asyncIterator]();
  let id = 0;
  const send = (message) => child.stdin.write(JSON.stringify(message) + "\n");
  const receive = async () => {
    const line = await lines.next();
    if (line.done) throw spawnError ?? new Error("Codex app-server closed its output");
    return JSON.parse(line.value);
  };
  const call = async (method, params, events) => {
    const request = ++id;
    send({ id: request, method, params });
    for (;;) {
      const response = await receive();
      if (response.id === request && !response.method) {
        assert(response.result, `Codex rejected ${method}`);
        return response.result;
      }
      events?.push(response);
    }
  };
  try {
    await call("initialize", {
      clientInfo: { name: "latch-live-smoke", version: "1.0.0" },
      capabilities: { experimentalApi: true },
    });
    send({ method: "initialized" });
    return await action({ call, receive, send });
  } finally {
    child.kill("SIGKILL");
    await exited;
  }
}
