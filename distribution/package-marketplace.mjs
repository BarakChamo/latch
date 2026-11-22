// Builds the binary-free Git marketplace for the supported harnesses.

import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pluginLauncher } from "./plugin-launcher.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));
const [destination, extra] = process.argv.slice(2);
if (!destination || extra) {
  throw new Error("Usage: node distribution/package-marketplace.mjs <new-destination|--refresh-launchers>");
}

const output = resolve(destination);
const plugins = [
  { host: "codex", catalog: ".agents/plugins", manifest: ".codex-plugin/plugin.json" },
  { host: "claude-code", catalog: ".claude-plugin", manifest: ".claude-plugin/plugin.json" },
  { host: "copilot", catalog: ".github/plugin", manifest: "plugin.json" },
].map((plugin) => ({
  ...plugin,
  descriptor: readJson(join(root, "plugins", plugin.host, plugin.manifest)),
}));
const extensions = ["pi", "opencode"];
const cli = readJson(join(root, "distribution/npm/package.json"));
const { version } = cli;
if (plugins.some(({ descriptor }) => descriptor.version !== version)
  || extensions.some((host) => readJson(join(root, "plugins", host, "package.json")).version !== version)) {
  throw new Error("Marketplace plugin versions must match.");
}
const launcher = pluginLauncher(version);
if (destination === "--refresh-launchers") {
  for (const { host } of plugins) {
    writeExecutable(join(root, "plugins", host, "bin/latch"), launcher);
  }
  process.exit(0);
}
mkdirSync(output);
cpSync(join(root, "LICENSE"), join(output, "LICENSE"));

for (const plugin of plugins) writePlugin(plugin);
writeExtensions();

function writeExtensions() {
  const server = "./plugins/opencode/index.js";
  writeJson(join(output, "package.json"), {
    name: "@barakchamo/latch-agent-plugins",
    version,
    description: "Run Latch checks automatically in Pi and OpenCode.",
    license: cli.license,
    homepage: "https://barakchamo.github.io/latch/",
    repository: { type: "git", url: "git+https://github.com/BarakChamo/latch.git" },
    keywords: ["pi-package", "opencode-plugin", "latch"],
    type: "module",
    exports: { ".": server, "./server": server },
    pi: { extensions: ["./plugins/pi/index.ts"] },
    peerDependencies: { "@earendil-works/pi-coding-agent": "*" },
    peerDependenciesMeta: { "@earendil-works/pi-coding-agent": { optional: true } },
    files: ["plugins/extension-hook.mjs", "plugins/pi", "plugins/opencode"],
    publishConfig: { access: "public" },
  });
  const source = join(root, "plugins");
  const target = join(output, "plugins");
  cpSync(join(source, "extension-hook.mjs"), join(target, "extension-hook.mjs"));
  for (const host of extensions) {
    cpSync(join(source, host), join(target, host), { recursive: true });
    writeExecutable(join(target, host, "bin/latch"), launcher);
  }
  cpSync(join(root, "distribution/extension-package.md"), join(output, "README.md"));
}

function writePlugin({ host, catalog, manifest }) {
  const source = join(root, "plugins", host);
  const target = join(output, "plugins", host);
  for (const path of [manifest, host === "copilot" ? "hooks.json" : "hooks/hooks.json", ".mcp.json", "bin/latch"]) {
    const destination = join(target, path);
    mkdirSync(dirname(destination), { recursive: true });
    cpSync(join(source, path), destination);
  }
  cpSync(join(root, catalog, "marketplace.json"), join(output, catalog, "marketplace.json"));
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
