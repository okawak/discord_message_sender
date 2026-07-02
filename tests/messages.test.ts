import { describe, expect, test } from "bun:test";
import {
  createProcessedMessage,
  type DiscordMessage,
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
    const message: DiscordMessage = {
      id: "123",
      content: "content",
      timestamp: "2026-06-21T03:00:00.000Z",
      author: {
        id: "author-id",
        username: "username",
        global_name: "Global name",
      },
      member: { nick: "Nickname" },
    };

    expect(createProcessedMessage("# title", true, message)).toEqual({
      messageId: "123",
      timestamp: "2026-06-21T03:00:00.000Z",
      authorId: "author-id",
      authorName: "Nickname",
      markdown: "# title",
      isClipping: true,
      fileName: "20260621_120000_123",
    });
  });

  test("falls back through the Discord author fields", () => {
    const base = {
      id: "123",
      content: "content",
      timestamp: "2026-06-21T03:00:00.000Z",
    };

    expect(
      createProcessedMessage("message", false, {
        ...base,
        author: {
          id: "author-id",
          username: "username",
          global_name: "Global name",
        },
      }).authorName,
    ).toBe("Global name");
    expect(
      createProcessedMessage("message", false, {
        ...base,
        author: { id: "author-id", username: "username" },
      }).authorName,
    ).toBe("username");
    expect(
      createProcessedMessage("message", false, {
        ...base,
        author: { id: "author-id" },
      }).authorName,
    ).toBe("author-id");
  });

  test("keeps the legacy JST file name outside Japan", () => {
    const processed = createProcessedMessage("message", false, {
      id: "123",
      content: "content",
      timestamp: "2026-06-30T15:30:45.000Z",
    });

    expect(processed.fileName).toBe("20260701_003045_123");
  });

  test("leaves invalid timestamps for storage validation", () => {
    const processed = createProcessedMessage("message", false, {
      id: "123",
      content: "content",
      timestamp: "invalid",
    });

    expect(processed.fileName).toBe("invalid_123");
  });
});
