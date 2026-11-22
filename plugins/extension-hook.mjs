// Invokes Latch from extension hosts and preserves their native hook output channel.

import { execFile } from "node:child_process";
import { existsSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Prefer the colocated binary or launcher. A checkout can use the release launcher.
 * @param {string} entryUrl
 * @returns {string}
 */
export function latchExecutable(entryUrl) {
  const directory = dirname(realpathSync(fileURLToPath(entryUrl)));
  const bundled = join(directory, "bin", process.platform === "win32" ? "latch.exe" : "latch");
  if (existsSync(bundled)) return bundled;
  const launcher = join(directory, "..", "extension-launch.sh");
  return process.platform !== "win32" && existsSync(launcher) ? launcher : "latch";
}

/** @typedef {{cwd: string, session_id: string, turn_id: string}} Turn */
/** @typedef {{hook_event_name: "PreToolUse" | "PostToolUse", tool_name: string, tool_use_id: string} | {hook_event_name: "UserPromptSubmit" | "Interrupt"} | {hook_event_name: "Stop", stop_hook_active: false, extension_can_continue?: boolean}} HookEvent */

/**
 * An empty result means there is no feedback. Hook transport errors go to stderr
 * so they never reject an edit or start a new model request.
 * @param {string} executable
 * @param {"opencode" | "pi"} harness
 * @param {Turn & HookEvent} event
 * @returns {Promise<string>}
 */
export function runHook(executable, harness, event) {
  return new Promise((resolve) => {
    const child = execFile(executable, ["hook", harness], { cwd: event.cwd }, (error, stdout, stderr) => {
      if (stderr.trim()) console.error(stderr.trimEnd());
      else if (error) console.error(`Latch hook failed to run for ${harness}: ${error.message}`);
      resolve(error ? "" : stdout);
    });
    // The process can exit before consuming stdin, for example when the binary cannot start.
    child.stdin?.on("error", (error) => {
      if (!("code" in error) || error.code !== "EPIPE") {
        console.error(`Latch could not send hook input: ${error.message}`);
      }
    });
    child.stdin?.end(JSON.stringify(event));
  });
}

/**
 * @param {string} executable
 * @param {"opencode" | "pi"} harness
 * @param {Turn} turn
 * @returns {Promise<{systemMessage?: string, continue?: boolean}>}
 */
export async function runStopHook(executable, harness, turn) {
  const output = await runHook(executable, harness, {
    ...turn, hook_event_name: "Stop", stop_hook_active: false, extension_can_continue: true,
  });
  return output.startsWith("{") ? JSON.parse(output) : { systemMessage: output };
}
