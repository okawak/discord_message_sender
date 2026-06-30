import { describe, expect, test } from "bun:test";
import {
  createProcessedMessage,
  parseWasmMessageInstruction,
} from "../src/messages";

describe("parseWasmMessageInstruction", () => {
  test("parses a regular message instruction", () => {
    expect(parseWasmMessageInstruction(["message", "# title"])).toEqual({
      kind: "message",
      markdown: "# title",
    });
  });

  test("parses a URL instruction", () => {
    expect(parseWasmMessageInstruction(["url", "https://example.com"])).toEqual(
      {
        kind: "url",
        url: "https://example.com",
      },
    );
  });

  test("rejects malformed wasm responses", () => {
    expect(() => parseWasmMessageInstruction(["message", false])).toThrow(
      "WASM returned an invalid message instruction.",
    );
  });

  test("rejects unknown message kinds", () => {
    expect(() => parseWasmMessageInstruction(["unknown", "value"])).toThrow(
      'WASM returned unknown message kind "unknown".',
    );
  });
});

describe("createProcessedMessage", () => {
  test("maps a message to the TypeScript domain model", () => {
    expect(
      createProcessedMessage(
        "# title",
        true,
        "2026-06-21T03:00:00.000Z",
        "123",
      ),
    ).toEqual({
      markdown: "# title",
      isClipping: true,
      fileName: "20260621_120000_123",
    });
  });

  test("uses the original timestamp when it is invalid", () => {
    expect(createProcessedMessage("message", false, "invalid", "123")).toEqual({
      markdown: "message",
      isClipping: false,
      fileName: "invalid_123",
    });
  });
});
