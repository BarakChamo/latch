// Builds a Git marketplace containing the Codex plugin and verified release executables.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const [archiveDir, destination] = process.argv.slice(2);
if (!archiveDir || !destination) {
  throw new Error("Usage: node distribution/package-codex-marketplace.mjs <archive-dir> <new-destination>");
}
const source = join(root, "plugins/codex");
const manifest = readJson(join(source, ".codex-plugin/plugin.json"));
const targets = [
  "aarch64-apple-darwin", "x86_64-apple-darwin",
  "aarch64-unknown-linux-gnu", "x86_64-unknown-linux-gnu",
];
const binaries = new Map(targets.map((target) => {
  const bundle = `latch-${manifest.version}-${target}`;
  const archive = resolve(archiveDir, `${bundle}.tar.gz`);
  const expected = readFileSync(`${archive}.sha256`, "utf8").split(/\s+/)[0];
  const actual = createHash("sha256").update(readFileSync(archive)).digest("hex");
  if (actual !== expected) throw new Error(`Checksum mismatch for ${archive}`);
  const executable = execFileSync("tar", ["-xOf", archive, `${bundle}/bin/latch`], {
    maxBuffer: 32 * 1024 * 1024,
  });
  return [target, executable];
}));

const output = resolve(destination);
mkdirSync(output);
const plugin = join(output, "plugins/codex");
for (const directory of [".agents/plugins", "plugins/codex/.codex-plugin", "plugins/codex/hooks", "plugins/codex/bin"]) {
  mkdirSync(join(output, directory), { recursive: true });
}

const marketplace = readJson(join(root, ".agents/plugins/marketplace.json"));
writeJson(join(output, ".agents/plugins/marketplace.json"), { ...marketplace, name: "latch-tools" });
writeJson(join(plugin, ".codex-plugin/plugin.json"), manifest);
const hooks = readJson(join(source, "hooks/hooks.json"));
for (const matchers of Object.values(hooks.hooks)) {
  for (const { hooks: handlers } of matchers) {
    for (const handler of handlers) {
      if (handler.command !== "latch hook codex") throw new Error("Unexpected Codex hook command");
      handler.command = '"${PLUGIN_ROOT}/bin/latch" hook codex';
    }
  }
}
writeJson(join(plugin, "hooks/hooks.json"), hooks);
writeJson(join(plugin, ".mcp.json"), {
  mcpServers: {
    latch: {
      type: "stdio", command: "/bin/sh", cwd: ".",
      args: ["./bin/latch", "mcp", "--require-project-directory"],
    },
  },
});
writeFileSync(join(plugin, "bin/latch"), readFileSync(join(root, "distribution/native/codex-launch.sh")));
chmodSync(join(plugin, "bin/latch"), 0o755);
for (const [target, executable] of binaries) {
  const path = join(plugin, "bin", target);
  writeFileSync(path, executable);
  chmodSync(path, 0o755);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function writeJson(path, contents) {
  writeFileSync(path, `${JSON.stringify(contents, null, 2)}\n`);
}
