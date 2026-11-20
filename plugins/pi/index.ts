// Uses Pi's final settlement boundary so diagnostics enter history without continuing the run.

import { randomUUID } from "node:crypto";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import type { Turn } from "../extension-hook.mjs";
import { latchExecutable, runHook } from "../extension-hook.mjs";

const edits = new Set(["edit", "write"]);

/** Registers the local Latch tools and handles native edits and foreground prompts. */
export default function latch(pi: ExtensionAPI): void {
  const executable = latchExecutable(import.meta.url);
  let active: Turn | undefined;

  pi.registerMcpServer("latch", { command: executable, args: ["mcp"], exposure: "direct" });

  pi.on("before_agent_start", async (event, ctx) => {
    if (event.messageType !== "user") return;
    const turn = { cwd: ctx.cwd, session_id: ctx.sessionManager.getSessionId(), turn_id: randomUUID() };
    active = turn;
    await runHook(executable, "pi", { ...turn, hook_event_name: "UserPromptSubmit" });
  });

  pi.on("tool_call", async (event) => {
    if (active && edits.has(event.toolName)) {
      await runHook(executable, "pi", {
        ...active, hook_event_name: "PreToolUse", tool_name: event.toolName, tool_use_id: event.toolCallId,
      });
    }
  });

  pi.on("tool_result", async (event) => {
    if (!active || event.isError || !edits.has(event.toolName)) return;
    const report = await runHook(executable, "pi", {
      ...active, hook_event_name: "PostToolUse", tool_name: event.toolName, tool_use_id: event.toolCallId,
    });
    if (report) return { content: [...event.content, { type: "text", text: report }] };
  });

  pi.on("agent_before_settle", async (event) => {
    if (!active || event.continue || event.outcome !== "completed") return;
    const turn = active;
    active = undefined;
    const report = await runHook(executable, "pi", {
      ...turn, hook_event_name: "Stop", stop_hook_active: false,
    });
    if (report) return {
      entries: [...event.entries, { type: "custom_message", customType: "latch", content: report, display: true }],
    };
  });

  async function discardTurn(): Promise<void> {
    if (!active) return;
    const turn = active;
    active = undefined;
    await runHook(executable, "pi", { ...turn, hook_event_name: "Interrupt" });
  }

  pi.on("agent_settled", discardTurn);
  pi.on("session_shutdown", discardTurn);
}
