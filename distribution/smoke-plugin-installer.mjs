// Exercises the release installer and generated marketplace launchers without network access.

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { pluginLauncher } from "./plugin-launcher.mjs";
import { latchExecutable } from "../plugins/extension-hook.mjs";

const version = JSON.parse(readFileSync(new URL("./npm/package.json", import.meta.url))).version;
const packager = fileURLToPath(new URL("./package-marketplace.mjs", import.meta.url));

test("Kimi plugin adapts numeric Stop identifiers for the released CLI", () => {
  const fixture = createFixture();
  try {
    fixture.packageAsset("darwin-arm64");
    const hook = fileURLToPath(new URL("../plugins/kimi/dispatch.sh", import.meta.url));
    const input = { hook_event_name: "Stop", turn_id: 0, session_id: "fixture" };
    const result = spawnSync(hook, { env: fixture.env, input: JSON.stringify(input), encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    const [command, payload] = result.stdout.split("\n");
    assert.equal(command, "hook kimi");
    assert.deepEqual(JSON.parse(payload), { ...input, turn_id: "0" });
  } finally {
    rmSync(fixture.root, { recursive: true });
  }
});

test("default-branch extensions find the release launcher and use the pinned CLI", () => {
  const fixture = createFixture();
  try {
    const root = new URL("../", import.meta.url);
    const manifest = JSON.parse(readFileSync(new URL("package.json", root), "utf8"));
    fixture.packageAsset("darwin-arm64");
    for (const entry of [...manifest.pi.extensions, manifest.exports["./server"]]) {
      const launcher = latchExecutable(new URL(entry, root).href);
      assert.equal(launcher, fileURLToPath(new URL("plugins/extension-launch.sh", root)));
      const result = run(launcher, ["--version"], fixture.env);
      assert.equal(result.status, 0, result.stderr);
      assert.match(result.stdout, /^--version/m);
    }
    assert.equal(fixture.downloads().length, 2);
  } finally {
    rmSync(fixture.root, { recursive: true });
  }
});

test("marketplace contains only a launcher and maps each supported platform to one release asset", () => {
  const fixture = createFixture();
  try {
    const catalog = join(fixture.root, "marketplace");
    execFileSync(process.execPath, [packager, catalog]);
    for (const host of ["codex", "claude-code", "copilot", "pi", "opencode"]) {
      const directory = join(catalog, "plugins", host, "bin");
      assert.deepEqual(readdirSync(directory), ["latch"]);
      assert.equal(readFileSync(join(directory, "latch"), "utf8"), pluginLauncher(version));
      if (["codex", "claude-code", "copilot"].includes(host)) {
        const gitLauncher = new URL(`../plugins/${host}/bin/latch`, import.meta.url);
        assert.equal(readFileSync(gitLauncher, "utf8"), pluginLauncher(version));
      }
    }
    const manifest = JSON.parse(readFileSync(join(catalog, "package.json"), "utf8"));
    assert.equal(manifest.version, version);
    assert(manifest.keywords.includes("pi-package"));
    assert(manifest.keywords.includes("opencode-plugin"));
    for (const entry of [...manifest.pi.extensions, manifest.exports["./server"], "plugins/extension-hook.mjs"]) {
      assert(existsSync(join(catalog, entry)), `Missing extension package entry ${entry}`);
    }
    for (const [os, arch, platform] of [
      ["Darwin", "arm64", "darwin-arm64"],
      ["Darwin", "x86_64", "darwin-x64"],
      ["Linux", "aarch64", "linux-arm64"],
      ["Linux", "x86_64", "linux-x64"],
    ]) {
      fixture.packageAsset(platform);
      const env = { ...fixture.env, FIXTURE_OS: os, FIXTURE_ARCH: arch };
      const launcher = join(catalog, "plugins", "codex", "bin", "latch");
      const first = run(launcher, ["hook", "codex"], env);
      assert.equal(first.status, 0, first.stderr);
      assert.equal(first.stdout, "hook codex\nhook JSON on stdin\n");
      const expected = fixture.downloads();
      assert.deepEqual(expected.slice(-2), [
        `https://github.com/BarakChamo/latch/releases/download/v${version}/barakchamo-latch-${platform}-${version}.tgz.sha256`,
        `https://github.com/BarakChamo/latch/releases/download/v${version}/barakchamo-latch-${platform}-${version}.tgz`,
      ]);
      const offline = run(launcher, ["hook", "codex"], { ...env, FIXTURE_OFFLINE: "yes" });
      assert.equal(offline.status, 0, offline.stderr);
      assert.deepEqual(fixture.downloads(), expected);
    }
    const unsupported = run(fixture.launcher, [], { ...fixture.env, FIXTURE_OS: "Other" });
    assert.equal(unsupported.status, 1);
    assert.match(unsupported.stderr, /support macOS and glibc Linux/);
  } finally {
    rmSync(fixture.root, { recursive: true });
  }
});

test("the install command uses native plugin commands and caches a verified binary", () => {
  const fixture = createFixture();
  try {
    const asset = fixture.packageAsset("darwin-arm64");
    writeFileSync(`${asset}.sha256`, "wrong  archive\n");
    const rejected = run(fixture.launcher, ["--install-plugin", "codex"], fixture.env);
    assert.equal(rejected.status, 1);
    assert.match(rejected.stderr, /checksum mismatch/);
    assert.equal(existsSync(fixture.invocations), false);
    assert.deepEqual(readdirSync(join(fixture.env.HOME, ".cache", "latch", "bin", version, "darwin-arm64")), []);

    fixture.packageAsset("darwin-arm64");
    for (const host of ["codex", "claude", "copilot"]) {
      const installed = run(fixture.launcher, ["--install-plugin", host], fixture.env);
      assert.equal(installed.status, 0, installed.stderr);
      assert.match(installed.stdout, /Start a new agent session/);
    }
    assert.equal(fixture.downloads().length, 4);
    assert.deepEqual(readFileSync(fixture.invocations, "utf8").trim().split("\n"), [
      "codex plugin marketplace add BarakChamo/latch",
      "codex plugin add latch@latch-tools",
      "claude plugin marketplace add https://github.com/BarakChamo/latch.git",
      "claude plugin install latch@latch-tools",
      "copilot plugin marketplace add BarakChamo/latch",
      "copilot plugin install latch@latch-tools",
    ]);
  } finally {
    rmSync(fixture.root, { recursive: true });
  }
});

function run(launcher, args, env) {
  return spawnSync("/bin/sh", [launcher, ...args], { env, input: "hook JSON on stdin\n", encoding: "utf8" });
}

function createFixture() {
  const root = mkdtempSync(join(tmpdir(), "latch installer "));
  const bin = join(root, "bin");
  const assets = join(root, "assets");
  const input = join(root, "input");
  const launcher = join(root, "install-latch-plugin.sh");
  const invocations = join(root, "harness-commands");
  const transfers = join(root, "downloads");
  mkdirSync(bin);
  mkdirSync(assets);
  mkdirSync(join(input, "package"), { recursive: true });
  writeFileSync(launcher, pluginLauncher(version));
  writeExecutable(join(input, "package", "latch"), 'printf "%s\\n" "$*"; cat');
  writeExecutable(join(bin, "uname"), 'case "$1" in -s) echo "$FIXTURE_OS";; -m) echo "$FIXTURE_ARCH";; esac');
  writeExecutable(join(bin, "curl"), `
    while [ "$#" -gt 0 ]; do
      case "$1" in
        -o) output=$2; shift ;;
        https://github.com/*) url=$1 ;;
      esac
      shift
    done
    printf '%s\\n' "$url" >> "$FIXTURE_TRANSFERS"
    [ -z "\${FIXTURE_OFFLINE:-}" ] || exit 1
    cp "$FIXTURE_ASSETS/\${url##*/}" "$output"
  `);
  for (const host of ["codex", "claude", "copilot"]) {
    writeExecutable(join(bin, host), `printf '%s %s\\n' '${host}' "$*" >> "$FIXTURE_INVOCATIONS"`);
  }
  return {
    root, launcher, invocations,
    env: {
      ...process.env,
      PATH: `${bin}:${process.env.PATH}`,
      HOME: join(root, "home"),
      FIXTURE_ASSETS: assets,
      FIXTURE_INVOCATIONS: invocations,
      FIXTURE_TRANSFERS: transfers,
      FIXTURE_OS: "Darwin",
      FIXTURE_ARCH: "arm64",
    },
    downloads: () => readFileSync(transfers, "utf8").trim().split("\n"),
    packageAsset(platform) {
      const filename = `barakchamo-latch-${platform}-${version}.tgz`;
      const path = join(assets, filename);
      execFileSync("tar", ["-czf", path, "-C", input, "package"]);
      const checksum = createHash("sha256").update(readFileSync(path)).digest("hex");
      writeFileSync(`${path}.sha256`, `${checksum}  ${filename}\n`);
      return path;
    },
  };
}

function writeExecutable(path, body) {
  writeFileSync(path, `#!/bin/sh\nset -eu\n${body}\n`, { mode: 0o755 });
}
