// Builds the binary-free Git marketplace for the supported harnesses.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pluginLauncher } from "./plugin-launcher.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const [destination, extra] = process.argv.slice(2);
if (!destination || extra) {
  throw new Error("Usage: node distribution/package-marketplace.mjs <new-destination>");
}

const output = resolve(destination);
const plugins = [
  { host: "codex", catalog: ".agents/plugins", manifest: ".codex-plugin/plugin.json", rootVariable: "PLUGIN_ROOT" },
  { host: "claude-code", catalog: ".claude-plugin", manifest: ".claude-plugin/plugin.json", rootVariable: "CLAUDE_PLUGIN_ROOT" },
  { host: "copilot", catalog: ".github/plugin", manifest: "plugin.json", rootVariable: "PLUGIN_ROOT", hooksFile: "hooks.json" },
].map((plugin) => ({
  ...plugin,
  descriptor: readJson(join(root, "plugins", plugin.host, plugin.manifest)),
}));
const version = plugins[0].descriptor.version;
if (plugins.some(({ descriptor }) => descriptor.version !== version)) {
  throw new Error("Marketplace plugin versions must match.");
}
const launcher = pluginLauncher(version);
mkdirSync(output);

for (const plugin of plugins) writePlugin(plugin);

function writePlugin({ host, catalog, descriptor, manifest, rootVariable, hooksFile = "hooks/hooks.json" }) {
  const source = join(root, "plugins", host);
  const target = join(output, "plugins", host);
  const marketplace = readJson(join(root, catalog, "marketplace.json"));
  writeJson(join(output, catalog, "marketplace.json"), { ...marketplace, name: "latch-tools" });
  writeJson(join(target, manifest), descriptor);

  const executable = "${" + rootVariable + "}/bin/latch";
  const hooks = readJson(join(source, hooksFile));
  for (const matchers of Object.values(hooks.hooks)) {
    const handlers = host === "copilot" ? matchers : matchers.flatMap(({ hooks: entries }) => entries);
    for (const handler of handlers) {
      const field = host === "copilot" ? "bash" : "command";
      if (!handler[field]?.startsWith("latch hook ")) {
        throw new Error(`Unexpected ${host} hook command`);
      }
      handler[field] = handler[field].replace("latch ", `"${executable}" `);
      if (host === "copilot") delete handler.powershell;
    }
  }
  writeJson(join(target, hooksFile), hooks);
  writeJson(join(target, ".mcp.json"), { mcpServers: { latch: mcpServer(host, executable) } });
  writeExecutable(join(target, "bin/latch"), launcher);
}

function mcpServer(host, executable) {
  if (host === "claude-code") return { type: "stdio", command: executable, args: ["mcp"] };
  const args = ["mcp", "--require-project-directory"];
  if (host === "codex") return { type: "stdio", command: "/bin/sh", cwd: ".", args: ["./bin/latch", ...args] };
  return { type: "stdio", command: "/bin/sh", args: [executable, ...args] };
}

function writeExecutable(path, contents) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, contents, { mode: 0o755 });
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function writeJson(path, contents) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(contents, null, 2)}\n`);
}
