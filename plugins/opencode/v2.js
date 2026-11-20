// Keeps OpenCode v2 feedback at native tool and session boundaries without queuing agent work.

import { latchExecutable, runHook } from "../extension-hook.mjs";

const edits = new Set(["edit", "write", "patch"]);

/** @type {import("@opencode/plugin").Plugin.Plugin["setup"]} */
export async function setup(ctx) {
  const directory = ctx.location.directory;
  const executable = latchExecutable(import.meta.url);
  const controller = new AbortController();
  /** @type {Map<string, {turn: import("../extension-hook.mjs").Turn, report: Promise<string>}>} */
  const active = new Map();
  /** @type {Map<string, Promise<string>>} */
  const settling = new Map();

  await ctx.mcp.transform((editor) => {
    if (!editor.get("latch")) {
      editor.set("latch", { type: "local", command: [executable, "mcp"], cwd: directory, codemode: false });
    }
  });

  await ctx.session.hook("prompt", async ({ sessionID, messageID }) => {
    if (active.has(sessionID)) return;
    const session = await ctx.session.get({ sessionID });
    // A prompt can be submitted while the previous execution is awaiting its checks.
    await settling.get(sessionID);
    if (session.parentID || active.has(sessionID)) return;

    const turn = { cwd: directory, session_id: sessionID, turn_id: messageID };
    const report = runHook(executable, "opencode", { ...turn, hook_event_name: "UserPromptSubmit" });
    active.set(sessionID, { turn, report });
    await report;
  });

  await ctx.session.hook("context", async ({ sessionID, messages }) => {
    const current = active.get(sessionID);
    if (!current) return;
    const report = await current.report;
    if (!report) return;

    // Request-only context leaves the persisted prompt and its attachments intact.
    const index = messages.findIndex((message) => message.id === current.turn.turn_id && message.role === "user");
    const message = messages[index];
    if (message?.role === "user") {
      messages[index] = { ...message, content: [...message.content, { type: "text", text: report }] };
    }
  });

  await ctx.tool.hook("execute.before", async ({ tool, sessionID, id }) => {
    const current = active.get(sessionID);
    if (!current || !edits.has(tool)) return;
    await runHook(executable, "opencode", {
      ...current.turn, hook_event_name: "PreToolUse", tool_name: tool, tool_use_id: id,
    });
  });

  await ctx.tool.hook("execute.after", async (event) => {
    const current = active.get(event.sessionID);
    if (!current || event.status !== "completed" || !edits.has(event.tool)) return;
    const report = await runHook(executable, "opencode", {
      ...current.turn, hook_event_name: "PostToolUse", tool_name: event.tool, tool_use_id: event.id,
    });
    if (!report) return;
    const { content, output } = event.result;
    const original = typeof content === "string"
      ? [{ type: /** @type {const} */ ("text"), text: content }]
      : content ?? [{ type: /** @type {const} */ ("text"), text: typeof output === "string" ? output : JSON.stringify(output) ?? "" }];
    event.result = { ...event.result, content: [...original, { type: "text", text: report }] };
  });

  await ctx.session.hook("experimental.idle", async ({ sessionID }) => {
    const current = active.get(sessionID);
    if (!current) return;
    active.delete(sessionID);
    // This awaited boundary finishes validation before the run exits. It must never admit another input.
    const check = runHook(executable, "opencode", {
      ...current.turn, hook_event_name: "Stop", stop_hook_active: false,
    });
    settling.set(sessionID, check);
    try {
      const report = await check;
      if (report) console.error(report);
    } finally {
      settling.delete(sessionID);
    }
  });

  /** @param {string} sessionID */
  async function discard(sessionID) {
    const current = active.get(sessionID);
    if (!current) return;
    active.delete(sessionID);
    await runHook(executable, "opencode", { ...current.turn, hook_event_name: "Interrupt" });
  }

  const events = (async () => {
    for await (const event of ctx.event.subscribe({
      types: ["session.execution.failed", "session.execution.interrupted", "session.deleted"],
      signal: controller.signal,
    })) {
      await discard(event.data.sessionID);
    }
  })().catch((error) => {
    if (!controller.signal.aborted) console.error("Latch could not watch OpenCode sessions:", error);
  });

  return async () => {
    controller.abort();
    await events;
    await Promise.all(settling.values());
    await Promise.all([...active.keys()].map(discard));
  };
}
