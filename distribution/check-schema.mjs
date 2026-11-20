// Validates the public editor contract and the checked-in configuration example.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import Ajv from "ajv/dist/2020.js";

const readJson = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const validate = new Ajv({ allowUnionTypes: true, strictTuples: false, strictRequired: true })
  .compile(readJson("../latch.schema.json"));

test("accepts discovery defaults, legacy commands, and ordered filtered hooks", () => {
  for (const config of [
    readJson("../examples/python/latch.json"),
    { commands: [["check", ""]] },
    { commands: null, hooks: { after_edit: [] } },
    {
      defaults: { when: "always", on_success: "output", on_failure: "collect", failure_output: "rtk" },
      hooks: {
        turn_end: { commands: [], on_failure: "stop", rtk_filter: "cargo-test" },
        after_edit: [
          { commands: [["npm", "test"]], paths: ["src/**"], extensions: [".TS"], on_success: "silent" },
          { commands: [["npx", "prettier", "--check", "."]], failure_output: "tail", tail_lines: 100 },
        ],
      },
    },
  ]) {
    assert.equal(validate(config), true, JSON.stringify(validate.errors));
  }
});

test("rejects invalid types, hook shapes, filters, commands, and output settings", () => {
  for (const config of [
    { $schema: false },
    { commands: [], hooks: {} },
    { commands: [[""]] },
    { commands: [["check", "\0"]] },
    { defaults: null },
    { defaults: { when: "changed" } },
    { defaults: { on_sucess: "output" } },
    { defaults: { failure_output: "summary" } },
    { defaults: { tail_lines: 0 } },
    { defaults: { tail_lines: 101 } },
    { defaults: { rtk_filter: "-vitest" } },
    { hooks: { before_edit: {} } },
    { hooks: { after_edit: [[[]]] } },
    { hooks: { after_edit: [{}, { commands: [] }] } },
    { hooks: { after_edit: { commands: [], typo: true } } },
    { hooks: { after_edit: { commands: [], extensions: [] } } },
    { hooks: { after_edit: { commands: [], extensions: ["tar.gz"] } } },
    { hooks: { after_edit: { commands: [], paths: ["src/../*"] } } },
  ]) {
    assert.equal(validate(config), false, JSON.stringify(config));
  }
});
