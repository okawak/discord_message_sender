import { describe, expect, test } from "bun:test";
import { parseWasmMessageResult } from "../src/messages";

describe("parseWasmMessageResult", () => {
  test("maps the wasm response to the TypeScript domain model", () => {
    expect(
      parseWasmMessageResult({
        md: "# title",
        is_clip: true,
        name: "example",
      }),
    ).toEqual({
      markdown: "# title",
      isClipping: true,
      fileName: "example",
    });
  });

  test("rejects malformed wasm responses", () => {
    expect(() =>
      parseWasmMessageResult({
        md: "# title",
        is_clip: "true",
        name: "example",
      }),
    ).toThrow("WASM returned an invalid processed message.");
  });
});
