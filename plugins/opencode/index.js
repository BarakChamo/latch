// Exposes one plugin entry point with the native OpenCode v1 and v2 adapters.

import { randomUUID } from "node:crypto";
import { latchExecutable, runHook } from "../extension-hook.mjs";
import { setup } from "./v2.js";

const edits = new Set(["edit", "write", "apply_patch"]);

/** @type {import("@opencode-ai/plugin").Plugin} */
async function server({ client, directory }) {
  const executable = latchExecutable(import.meta.url);
  /** @type {Map<string, import("../extension-hook.mjs").Turn>} */
  const active = new Map();
  /** @type {Map<string, Promise<void>>} */
  const settling = new Map();

  return {
    async config(config) {
      config.mcp ??= {};
      config.mcp.latch ??= { type: "local", command: [executable, "mcp"], enabled: true };
    },

    async "chat.message"({ sessionID }, { message, parts }) {
      if (parts.length > 0 && parts.every((part) => part.type === "text" && part.synthetic)) return;
      const session = await client.session.get({ path: { id: sessionID } });
      if (session.data?.parentID) return;

      await settling.get(sessionID);
      if (active.has(sessionID)) return;
      const turn = { cwd: directory, session_id: sessionID, turn_id: message.id };
      active.set(sessionID, turn);
      const report = await runHook(executable, "opencode", { ...turn, hook_event_name: "UserPromptSubmit" });
      if (!report) return;
      parts.push({
        type: "text",
        id: `prt_latch_${randomUUID()}`,
        sessionID,
        messageID: message.id,
        synthetic: true,
        text: report,
      });
    },

    async "tool.execute.before"({ tool, sessionID, callID }) {
      const turn = active.get(sessionID);
      if (turn && edits.has(tool)) {
        await runHook(executable, "opencode", {
          ...turn, hook_event_name: "PreToolUse", tool_name: tool, tool_use_id: callID,
        });
      }
    },

    async "tool.execute.after"({ tool, sessionID, callID }, output) {
      const turn = active.get(sessionID);
      if (turn && edits.has(tool)) {
        const report = await runHook(executable, "opencode", {
          ...turn, hook_event_name: "PostToolUse", tool_name: tool, tool_use_id: callID,
        });
        if (report) output.output += `\n\n${report}`;
      }
    },

    async event({ event }) {
      if (event.type !== "session.status" || event.properties.status.type !== "idle") return;
      const { sessionID } = event.properties;
      const turn = active.get(sessionID);
      if (!turn) return;
      active.delete(sessionID);
      const check = runHook(executable, "opencode", {
        ...turn, hook_event_name: "Stop", stop_hook_active: false,
      }).then(async (report) => {
        if (!report) return;
        // Headless runs retain stderr. Interactive sessions also receive a toast.
        if (!process.stdout.isTTY) console.error(report);
        try {
          await client.tui.showToast({ body: { title: "Latch", message: report, variant: "warning" } });
        } catch (error) {
          console.error("Latch could not show the OpenCode toast:", error);
        }
      });
      settling.set(sessionID, check);
      try {
        await check;
      } finally {
        settling.delete(sessionID);
      }
    },

    async dispose() {
      // OpenCode dispatches bus listeners without awaiting them.
      await Promise.all(settling.values());
      await Promise.all([...active.values()].map((turn) => runHook(executable, "opencode", {
        ...turn, hook_event_name: "Interrupt",
      })));
    },
  };
}

export default { id: "latch", server, setup };
