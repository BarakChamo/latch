// Packs one registry CLI from the checksummed platform packages of the same release.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const [assets, destination = "dist", extra] = process.argv.slice(2);
const npm = process.env.npm_execpath;
if (!assets || extra || !npm) {
  throw new Error("Usage: npm run package:npm-cli -- <platform-packages-directory> [output-directory]");
}

const { name: artifactName, version, description, license, engines, bin } = JSON.parse(
  readFileSync(join(root, "distribution/npm/package.json"), "utf8"),
);
const manifest = {
  name: "latch-hooks",
  version,
  description,
  license,
  engines,
  bin,
  homepage: "https://barakchamo.github.io/latch/",
  repository: { type: "git", url: "git+https://github.com/BarakChamo/latch.git" },
  files: ["cli.cjs", "bin", "latch.schema.json"],
};
const platforms = ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64", "win32-x64"];
const output = resolve(destination);
const staging = mkdtempSync(join(tmpdir(), "latch-npm-cli-"));

try {
  for (const platform of platforms) {
    const filename = `${artifactName.replace(/^@/, "").replace("/", "-")}-${platform}-${version}.tgz`;
    const asset = resolve(assets, filename);
    const [expected, recorded] = readFileSync(`${asset}.sha256`, "utf8").trim().split(/\s+/);
    const archive = readFileSync(asset);
    const actual = createHash("sha256").update(archive).digest("hex");
    if (actual !== expected || recorded !== filename) {
      throw new Error(`Checksum mismatch for ${filename}.`);
    }
    const target = join(staging, "bin", platform);
    const executable = platform.startsWith("win32-") ? "latch.exe" : "latch";
    mkdirSync(target, { recursive: true });
    const binary = execFileSync("tar", ["-xOzf", "-", `package/${executable}`], {
      input: archive,
      maxBuffer: 32 * 1024 * 1024,
    });
    writeFileSync(join(target, executable), binary, { mode: 0o755 });
  }

  cpSync(join(root, "distribution/npm/cli.cjs"), join(staging, "cli.cjs"));
  cpSync(join(root, "distribution/npm/README.md"), join(staging, "README.md"));
  cpSync(join(root, "LICENSE"), join(staging, "LICENSE"));
  cpSync(join(root, "latch.schema.json"), join(staging, "latch.schema.json"));
  writeFileSync(join(staging, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  mkdirSync(output, { recursive: true });
  execFileSync(process.execPath, [npm, "pack", "--pack-destination", output], {
    cwd: staging,
    stdio: "inherit",
  });
} finally {
  rmSync(staging, { recursive: true });
}
