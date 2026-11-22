import { expect, test } from "vitest";
import { greeting } from "../src/message.js";

test("greets the caller by name", () => {
  expect(greeting("Latch")).toBe("Hello, Latch!");
});
