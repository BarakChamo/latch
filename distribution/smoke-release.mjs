// Exercises the packed npm launcher and standalone archive on their target platform.

import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const { name, version } = JSON.parse(readFileSync(join(root, "distribution/npm/package.json"), "utf8"));
const [target, destination = "dist"] = process.argv.slice(2);
const archives = resolve(destination);
const npm = process.env.npm_execpath;
assert(target && npm, "Use npm run test:release -- <target> [archive-directory].");

const windows = process.platform === "win32";
const packageStem = name.replace(/^@/, "").replace("/", "-");
const bundle = `latch-${version}-${target}`;
const archive = join(archives, `${bundle}.tar.gz`);
const platformTarball = join(archives, `${packageStem}-${process.platform}-${process.arch}-${version}.tgz`);
const scratch = mkdtempSync(join(tmpdir(), "latch release test "));

try {
  execFileSync(process.execPath, [npm, "run", "package:release", "--", "npm", scratch], {
    cwd: root,
    stdio: "inherit",
  });
  const prefix = join(scratch, "install");
  execFileSync(process.execPath, [npm, "install", "--global", "--offline", "--ignore-scripts", "--prefix",
    prefix, join(scratch, `${packageStem}-${version}.tgz`), platformTarball], { stdio: "inherit" });

  const bin = windows ? prefix : join(prefix, "bin");
  const env = { ...process.env, PATH: `${bin}${delimiter}${process.env.PATH ?? ""}` };
  const project = join(scratch, "project");
  mkdirSync(project);
  const run = (...args) => spawnSync(
    windows ? "cmd.exe" : join(bin, "latch"),
    windows ? ["/d", "/c", "latch", ...args] : args,
    { cwd: project, env, encoding: "utf8" },
  );
  const installed = run("--version");
  assert.equal(installed.status, 0, installed.stderr);
  assert.equal(installed.stdout.trim(), `latch ${version}`);

  writeFileSync(join(project, "latch.json"), JSON.stringify({
    hooks: { after_edit: { commands: [["node", "-e", "console.log('release smoke check')"]] } },
  }));
  const success = run("run", "after_edit");
  assert.equal(success.status, 0, success.stderr);
  assert.equal(success.stdout, "");
  writeFileSync(join(project, "latch.json"), JSON.stringify({
    commands: [["node", "-e", "console.error('release smoke failure'); process.exit(17)"]],
  }));
  const failure = run();
  assert.equal(failure.status, 1, failure.stderr);
  assert.match(failure.stderr, /release smoke failure/);

  const checksum = readFileSync(`${archive}.sha256`, "utf8").split(/\s+/)[0];
  assert.equal(createHash("sha256").update(readFileSync(archive)).digest("hex"), checksum);
  execFileSync("tar", ["-xzf", archive, "-C", scratch], { stdio: "inherit" });
  const extracted = join(scratch, bundle);
  const binary = join(extracted, "bin", windows ? "latch.exe" : "latch");
  assert.equal(execFileSync(binary, ["--version"], { encoding: "utf8" }).trim(), `latch ${version}`);
  for (const catalog of [".agents/plugins", ".claude-plugin", ".github/plugin"]) {
    assert(existsSync(join(extracted, catalog, "marketplace.json")), `Missing marketplace ${catalog}`);
  }
  console.log(`Verified the npm install and standalone archive for ${target}.`);
} finally {
  rmSync(scratch, { recursive: true });
}
