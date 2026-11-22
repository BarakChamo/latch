// Drives real OpenCode and Pi sessions against a scripted localhost model.

import assert from "node:assert/strict";
import { execFile, execFileSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { parseArgs, promisify } from "node:util";

const exec = promisify(execFile);
const { positionals: [archive, ...selected], values: { marketplace } } = parseArgs({
  options: { marketplace: { type: "boolean" } },
  allowPositionals: true,
});
const harnesses = selected.length ? selected : ["opencode", "pi"];
assert(archive && harnesses.every((host) => ["opencode", "pi"].includes(host)),
  "Use npm run test:extensions -- <archive.tar.gz> [opencode|pi ...] [--marketplace]");
const scratch = mkdtempSync(join(tmpdir(), "latch extension smoke "));
const editMarker = "LATCH_FIXTURE_EDIT_FAILURE";
const secondEditMarker = "LATCH_FIXTURE_SECOND_FAILURE";
const turnMarker = "LATCH_FIXTURE_TURN_FAILURE";

try {
  execFileSync("tar", ["-xzf", resolve(archive), "-C", scratch]);
  const [bundle] = readdirSync(scratch);
  assert(bundle, "Archive must contain a release directory");
  for (const host of harnesses) await verify(host, join(scratch, bundle));
} finally {
  rmSync(scratch, { recursive: true, force: true });
}

async function verify(host, bundle) {
  const root = join(scratch, host);
  const project = join(root, "project");
  const profile = join(root, "pi");
  for (const dir of [project, profile]) mkdirSync(dir, { recursive: true });
  const guard = join(root, "guard");
  mkdirSync(guard);
  writeFileSync(join(guard, "latch"), "#!/bin/sh\nexit 97\n", { mode: 0o755 });
  const env = {
    PATH: [guard, process.env.PATH].join(delimiter),
    HOME: root,
    XDG_CONFIG_HOME: join(root, "config"),
    XDG_DATA_HOME: join(root, "data"),
    XDG_CACHE_HOME: join(root, "cache"),
    XDG_STATE_HOME: join(root, "state"),
    PI_CODING_AGENT_DIR: profile,
    PI_OFFLINE: "1",
    OPENCODE_DISABLE_AUTOUPDATE: "1",
    OPENCODE_DISABLE_MODELS_FETCH: "1",
    OPENCODE_DISABLE_LSP_DOWNLOAD: "1",
    OPENCODE_EXPERIMENTAL_DISABLE_FILEWATCHER: "1",
  };
  const opencodeVersion = host === "opencode"
    ? execFileSync(host, ["--version"], { cwd: project, env, encoding: "utf8" }).trim()
    : "";
  const isOpenCodeV2 = /^(?:opencode v?)?2\./.test(opencodeVersion);
  const fixture = join(project, "tracked.txt");
  const counter = join(project, "checks.log");
  writeFileSync(fixture, "ready\n");
  writeFileSync(join(project, ".gitignore"), "checks.log\n");
  writeJson(join(project, "latch.json"), {
    hooks: {
      after_edit: {
        commands: [
          ["sh", "-c", `echo edit >> checks.log; printf '%s\\n' '${editMarker}' >&2; exit 1`],
          ["sh", "-c", `echo second-edit >> checks.log; printf '%s\\n' '${secondEditMarker}' >&2; exit 2`],
        ],
      },
      turn_end: {
        commands: [["sh", "-c", `echo turn >> checks.log; printf '%s\\n' '${turnMarker}' >&2; exit 1`]],
      },
    },
  });
  for (const args of [["init", "-q"], ["add", "."], ["-c", "user.name=Latch Smoke",
    "-c", "user.email=latch@example.test", "-c", "commit.gpgsign=false", "commit", "-qm", "Initial fixture"]]) {
    execFileSync("git", args, { cwd: project, env });
  }

  const requests = [];
  const server = createServer(async (req, res) => {
    try {
      let body = "";
      for await (const chunk of req) body += chunk;
      const model = JSON.parse(body);
      const latest = model.messages.findLast((message) => message.role === "user"
        && /FIXTURE_(NOOP|CHANGE|READ|TOOL|CONTINUE)/.test(JSON.stringify(message.content)));
      const marker = JSON.stringify(latest?.content)?.match(/FIXTURE_(NOOP|CHANGE|READ|TOOL|CONTINUE)/)?.[1];
      const previous = requests.findLast((request) => request.marker === marker);
      requests.push({ marker, model });
      const tools = model.tools?.map((tool) => tool.function.name) ?? [];
      const runTool = tools.find((name) => name.endsWith("run_turn_end"));
      if (marker === "TOOL" && !previous && runTool) {
        return reply(res, toolCall(runTool, {}));
      }
      if ((marker === "CHANGE" || marker === "CONTINUE") && isOpenCodeV2 && !previous) {
        return reply(res, toolCall("patch", {
          patchText: [
            "*** Begin Patch", "*** Update File: tracked.txt", "@@",
            ...(marker === "CHANGE" ? ["-already dirty", "+new edit to dirty file"]
              : ["-new edit to dirty file", "+continued edit to dirty file"]), "*** End Patch",
          ].join("\n"),
        }));
      }
      if ((marker === "NOOP" || marker === "CHANGE" || marker === "CONTINUE") && !previous) {
        const pathField = host === "pi" || isOpenCodeV2 ? "path" : "filePath";
        const content = {
          NOOP: "already dirty\n",
          CHANGE: "new edit to dirty file\n",
          CONTINUE: "continued edit to dirty file\n",
        }[marker];
        return reply(res, toolCall("write", { [pathField]: fixture, content }));
      }
      reply(res, { content: "Fixture reply." });
    } catch (error) {
      res.writeHead(500).end(String(error));
    }
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  try {
    const baseURL = `http://127.0.0.1:${server.address().port}/v1`;
    if (host === "pi") {
      writeJson(join(profile, "models.json"), { providers: { fixture: {
        baseUrl: baseURL, api: "openai-completions", apiKey: "fixture-key", models: [{ id: "local" }],
      } } });
    } else if (isOpenCodeV2) {
      const model = {
        name: "Fixture",
        capabilities: { tools: true, input: ["text"], output: ["text"] },
        limit: { context: 32768, output: 8192 },
      };
      writeJson(join(project, "opencode.json"), {
        $schema: "https://opencode.ai/config.json",
        providers: { fixture: {
          package: "@opencode/ai/providers/openai-compatible",
          name: "Fixture",
          settings: { baseURL, apiKey: "fixture-key" },
          models: { local: model, "gpt-latch": model },
        } },
      });
    } else {
      writeJson(join(project, "opencode.json"), {
        provider: { fixture: {
          npm: "@ai-sdk/openai-compatible", name: "Fixture", options: { baseURL, apiKey: "fixture-key" },
          models: { local: { name: "Fixture" } },
        } },
        small_model: "fixture/local",
        permission: "allow",
        formatter: false,
        agent: { title: { disable: true } },
        $schema: "https://opencode.ai/config.json",
      });
    }
    if (host === "opencode") {
      execFileSync("git", ["add", "opencode.json"], { cwd: project, env });
      execFileSync("git", ["-c", "user.name=Latch Smoke", "-c", "user.email=latch@example.test",
        "-c", "commit.gpgsign=false", "commit", "-qm", "Add model configuration"], { cwd: project, env });
    }
    await install(host, isOpenCodeV2, bundle, project, env);
    const piSession = randomUUID();
    let opencodeSession;
    const turn = async (marker) => {
      // V2 exposes patch in place of edit/write for GPT-family models.
      const opencodePatch = isOpenCodeV2 && ["FIXTURE_CHANGE", "FIXTURE_CONTINUE"].includes(marker);
      const args = host === "pi"
        ? ["--print", "--mode", "json", "--no-approve", "--model", "fixture/local", "--session-id", piSession, marker]
        : [...(isOpenCodeV2 ? ["--standalone"] : []),
          ...(opencodePatch ? ["--print-logs"] : []),
          "run", "--format", "json", "--model", opencodePatch ? "fixture/gpt-latch" : "fixture/local",
          ...(isOpenCodeV2 ? ["--auto", "--title", "Latch smoke"] : []),
          ...(opencodeSession ? ["--session", opencodeSession] : []), marker];
      const pending = exec(host, args, { cwd: project, env, timeout: 60_000 });
      pending.child.stdin.end();
      const result = await pending;
      if (host === "opencode" && !opencodeSession) {
        const events = result.stdout.split("\n").filter((line) => line.startsWith("{")).map(JSON.parse);
        opencodeSession = events.map((event) => event.sessionID ?? event.data?.sessionID).find(Boolean);
        assert(opencodeSession, `OpenCode did not report its session ID: ${result.stdout} ${result.stderr}`);
      }
      return result;
    };
    const forTurn = (marker) => requests.filter((request) => request.marker === marker);

    writeFileSync(fixture, "already dirty\n");
    await turn("FIXTURE_NOOP");
    assert.equal(forTurn("NOOP").length, 2, `${host} did not complete the native no-op edit`);
    assert(!existsSync(counter), `${host} ran checks for an already-dirty no-op`);
    const changed = await turn("FIXTURE_CHANGE");
    assert.equal(readFileSync(fixture, "utf8"), "new edit to dirty file\n", changed.stdout);
    assert.equal(readFileSync(counter, "utf8"), "edit\nsecond-edit\nturn\n");
    assert.equal(forTurn("CHANGE").length, 2, `${host} continued the turn after the failed check`);
    const modelContext = JSON.stringify(forTurn("CHANGE")[1].model.messages);
    for (const marker of [editMarker, secondEditMarker]) {
      assert(modelContext.includes(marker), `${host} dropped edit diagnostic ${marker}`);
    }
    const surfaced = changed.stdout + changed.stderr;
    assert(surfaced.includes(turnMarker), `${host} did not report turn-end failure: ${surfaced}`);

    await turn("FIXTURE_READ");
    assert.equal(forTurn("READ").length, 1);
    assert(JSON.stringify(forTurn("READ")[0].model.messages).includes(turnMarker), `${host} dropped the prior turn diagnostic`);
    assert.equal(readFileSync(counter, "utf8"), "edit\nsecond-edit\nturn\n");

    await turn("FIXTURE_TOOL");
    const calls = forTurn("TOOL");
    assert.equal(calls.length, 2, `${host} did not expose the explicit Latch tool`);
    assert(calls[0].model.tools.some((tool) => tool.function.name.endsWith("search_output")),
      `${host} did not expose the saved-output search tool`);
    const toolResult = calls[1].model.messages.findLast((message) => message.role === "tool");
    assert(JSON.stringify(toolResult).includes(turnMarker), `${host} dropped the explicit tool failure`);
    assert.equal(readFileSync(counter, "utf8"), "edit\nsecond-edit\nturn\nturn\n");
    if (host === "pi" || isOpenCodeV2) {
      const configPath = join(project, "latch.json");
      const config = JSON.parse(readFileSync(configPath, "utf8"));
      writeJson(configPath, { ...config, turn_end_feedback: "continue_once" });
      await turn("FIXTURE_CONTINUE");
      const continuation = forTurn("CONTINUE");
      assert.equal(continuation.length, 3, `${host} should make one additional model request`);
      assert(JSON.stringify(continuation[2].model.messages).includes("Review the Latch failure below"));
      assert.equal(readFileSync(counter, "utf8"), "edit\nsecond-edit\nturn\nturn\nedit\nsecond-edit\nturn\n");
    }
    console.log(`${host}: ${marketplace ? "Git package" : "archive"} install, no-op, same-turn feedback, turn-end failure, next prompt, and MCP passed`);
  } finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
}

async function install(host, isOpenCodeV2, bundle, cwd, env) {
  const options = { cwd, env, timeout: 60_000 };
  if (!marketplace) {
    await exec(join(bundle, "install"), [host], options);
  } else if (host === "pi") {
    const { PI_OFFLINE, ...onlineEnv } = env;
    await exec("pi", ["install", "git:github.com/BarakChamo/latch"], { ...options, env: onlineEnv });
  } else if (isOpenCodeV2) {
    await exec("opencode", ["plugin", "add", "github:BarakChamo/latch#main::path:plugins"], options);
  } else {
    const config = join(env.XDG_CONFIG_HOME, "opencode");
    mkdirSync(join(config, "plugins"), { recursive: true });
    await exec("git", ["clone", "--depth", "1",
      "https://github.com/BarakChamo/latch.git", join(config, "latch")], options);
    symlinkSync("../latch/plugins/opencode/index.js", join(config, "plugins", "latch.js"));
  }
}

function writeJson(path, value) {
  writeFileSync(path, JSON.stringify(value));
}

function toolCall(name, args) {
  return { tool_calls: [{
    index: 0, id: `call_${randomUUID()}`, type: "function",
    function: { name, arguments: JSON.stringify(args) },
  }] };
}

function reply(response, delta) {
  response.writeHead(200, { "content-type": "text/event-stream" });
  const envelope = { id: "fixture", object: "chat.completion.chunk", created: 0, model: "local" };
  const emit = (choices) => response.write(`data: ${JSON.stringify({ ...envelope, choices })}\n\n`);
  emit([{ index: 0, delta: { role: "assistant", ...delta }, finish_reason: null }]);
  emit([{ index: 0, delta: {}, finish_reason: delta.tool_calls ? "tool_calls" : "stop" }]);
  response.end("data: [DONE]\n\n");
}
