// Stages registry packages and archives containing the native CLI and harness plugins.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const targets = {
  "aarch64-apple-darwin": ["darwin", "arm64"],
  "x86_64-apple-darwin": ["darwin", "x64"],
  "aarch64-unknown-linux-gnu": ["linux", "arm64"],
  "x86_64-unknown-linux-gnu": ["linux", "x64"],
  "x86_64-pc-windows-msvc": ["win32", "x64"],
};
const pluginManifests = [
  "package.json",
  "plugins/package.json",
  "plugin.json",
  "kimi.plugin.json",
  "plugins/codex/.codex-plugin/plugin.json",
  "plugins/claude-code/.claude-plugin/plugin.json",
  "plugins/copilot/plugin.json",
  "plugins/cursor/.cursor-plugin/plugin.json",
  "plugins/opencode/package.json",
  "plugins/pi/package.json",
  "plugins/kimi/kimi.plugin.json",
  "plugins/gemini/gemini-extension.json",
  "plugins/antigravity/plugin.json",
];
const sharedFiles = ["LICENSE", "plugins", ".agents/plugins", ".claude-plugin", ".cursor-plugin", ".github/plugin", "latch.schema.json"];
const [target, destination = "dist"] = process.argv.slice(2);
const output = resolve(destination);
const packageManifest = readJson(join(root, "distribution/npm/package.json"));
const { name, version, license } = packageManifest;

if (target !== "npm" && !Object.hasOwn(targets, target)) {
  throw new Error(`Use npm or one of these targets: ${Object.keys(targets).join(", ")}`);
}
const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--locked", "--no-deps", "--format-version", "1"], { cwd: root }));
const rustPackage = metadata.packages.find((candidate) => candidate.name === "latch");
if (rustPackage?.version !== version || pluginManifests.some((path) => readJson(join(root, path)).version !== version)) {
  throw new Error("The Cargo, npm and plugin versions must match.");
}
if (process.env.GITHUB_REF_TYPE === "tag" && process.env.GITHUB_REF_NAME !== `v${version}`) {
  throw new Error(`The release tag must be v${version}.`);
}
mkdirSync(output, { recursive: true });
const staging = mkdtempSync(join(tmpdir(), "latch-release-"));

try {
  if (target === "npm") {
    const packageRoot = join(staging, "wrapper");
    cpSync(join(root, "distribution/npm"), packageRoot, { recursive: true });
    copySharedFiles(packageRoot);
    const optionalDependencies = Object.fromEntries(
      Object.values(targets).map(([os, cpu]) => [platformName(os, cpu), version]),
    );
    writeJson(join(packageRoot, "package.json"), { ...packageManifest, private: false, optionalDependencies });
    npmPack(packageRoot);
    const marketplace = join(staging, "marketplace");
    execFileSync(process.execPath, [join(root, "distribution/package-marketplace.mjs"), marketplace]);
    npmPack(marketplace);
  } else {
    packPlatform(target);
  }
} finally {
  rmSync(staging, { recursive: true });
}

function packPlatform(target) {
  const [os, cpu] = targets[target];
  const executable = os === "win32" ? "latch.exe" : "latch";
  const binary = join(metadata.target_directory, target, "release", executable);
  const packageRoot = join(staging, "binary");
  mkdirSync(packageRoot);
  cpSync(binary, join(packageRoot, executable));
  cpSync(join(root, "LICENSE"), join(packageRoot, "LICENSE"));
  chmodSync(join(packageRoot, executable), 0o755);
  writeJson(join(packageRoot, "package.json"), {
    name: platformName(os, cpu), version, license, os: [os], cpu: [cpu], files: [executable],
    ...(os === "linux" ? { libc: ["glibc"] } : {}),
  });
  npmPack(packageRoot);
  const tarball = `${platformName(os, cpu).replace(/^@/, "").replace("/", "-")}-${version}.tgz`;
  writeChecksum(tarball);

  const directory = `latch-${version}-${target}`;
  const bundle = join(staging, directory);
  const bin = join(bundle, "bin");
  mkdirSync(bin, { recursive: true });
  cpSync(join(packageRoot, executable), join(bin, executable));
  copySharedFiles(bundle);
  packGeminiExtension(join(bundle, "plugins", "gemini"), binary, os, cpu, executable);
  if (os !== "win32") {
    bundlePluginExecutables(bundle, join(bin, executable));
    const installer = join(bundle, "install");
    cpSync(join(root, "distribution/native/install.sh"), installer);
    chmodSync(installer, 0o755);
  }
  cpSync(join(root, "distribution/native/README.md"), join(bundle, "README.md"));
  const filename = `${directory}.tar.gz`;
  writeArchive(filename, staging, directory);
  writeChecksum(filename);
}

function packGeminiExtension(plugin, binary, os, cpu, executable) {
  // The native installer picks the first asset with this platform prefix. Do not share it with a checksum asset.
  const extension = join(staging, "gemini-extension");
  cpSync(plugin, extension, { recursive: true });
  if (os === "win32") {
    rmSync(join(extension, "bin", "latch"));
    for (const path of ["gemini-extension.json", "hooks/hooks.json"]) {
      const config = join(extension, path);
      writeFileSync(config, readFileSync(config, "utf8").replaceAll("/bin/latch", "/bin/latch.exe"));
    }
  }
  cpSync(binary, join(extension, "bin", executable));
  chmodSync(join(extension, "bin", executable), 0o755);
  cpSync(join(root, "LICENSE"), join(extension, "LICENSE"));
  writeArchive(`${os}.${cpu}.gemini-extension.tar.gz`, extension, ".");
}

function writeArchive(filename, directory, entry) {
  const archiveOptions = process.platform === "darwin" ? ["--no-xattrs"] : [];
  execFileSync("tar", [...archiveOptions, "-czf", join(output, filename), "-C", directory, entry], {
    env: { ...process.env, COPYFILE_DISABLE: "1" },
    stdio: "inherit",
  });
}

function writeChecksum(filename) {
  const digest = createHash("sha256").update(readFileSync(join(output, filename))).digest("hex");
  writeFileSync(join(output, `${filename}.sha256`), `${digest}  ${filename}\n`);
}

function copySharedFiles(directory) {
  for (const path of sharedFiles) {
    cpSync(join(root, path), join(directory, path), { recursive: true });
  }
}

function bundlePluginExecutables(bundle, binary) {
  for (const harness of ["opencode", "pi", "kimi"]) {
    const bin = join(bundle, "plugins", harness, "bin");
    mkdirSync(bin);
    cpSync(binary, join(bin, "latch"));
  }

  for (const harness of ["codex", "claude-code", "copilot", "cursor", "gemini", "antigravity"]) {
    const plugin = join(bundle, "plugins", harness);
    cpSync(binary, join(plugin, "bin", "latch"));
  }
}

function npmPack(directory) {
  const npm = process.env.npm_execpath;
  if (!npm) throw new Error("Run the packager via npm run package:release.");
  execFileSync(process.execPath, [npm, "pack", "--pack-destination", output], {
    cwd: directory,
    stdio: "inherit",
  });
}

function platformName(os, cpu) {
  return `${name}-${os}-${cpu}`;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function writeJson(path, data) {
  writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
}
