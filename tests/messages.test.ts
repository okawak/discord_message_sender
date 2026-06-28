import { describe, expect, test } from "bun:test";
import { parseWasmMessageResult } from "../src/messages";

describe("parseWasmMessageResult", () => {
  test("maps the wasm response to the TypeScript domain model", () => {
    expect(
      parseWasmMessageResult(["# title", true], "2026-06-21T03:00:00.000Z"),
    ).toEqual({
      markdown: "# title",
      isClipping: true,
      fileName: "20260621_120000",
    });
  });

  test("rejects malformed wasm responses", () => {
    expect(() =>
      parseWasmMessageResult(["# title", "true"], "timestamp"),
    ).toThrow("WASM returned an invalid processed message.");
  });

  test("uses the original timestamp when it is invalid", () => {
    expect(parseWasmMessageResult(["message", false], "invalid")).toMatchObject(
      {
        fileName: "invalid",
      },
    );
  });
});
