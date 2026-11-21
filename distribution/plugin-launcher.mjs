// Pins the shared marketplace launcher and release installer to the package version.

import { readFileSync } from "node:fs";

export function pluginLauncher(version) {
  const source = readFileSync(new URL("./native/plugin-launch.sh", import.meta.url), "utf8");
  return source.replace("@LATCH_VERSION@", version);
}
