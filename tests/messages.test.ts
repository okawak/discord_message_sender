import { describe, expect, test } from "bun:test";
import {
  processed_message as createProcessedMessage,
  type DiscordMessage,
  message_instruction,
} from "../pkg/parse_message.js";
import { isSavedClippingInstruction } from "../src/messageParsing";

describe("Rust message instructions", () => {
  test("returns a typed regular-message instruction", () => {
    expect(message_instruction("# title", "!")).toEqual({
      kind: "message",
      markdown: "# title",
    });
  });
  test("returns a typed URL instruction", () => {
    expect(message_instruction("!url https://example.com", "!")).toEqual({
      kind: "url",
      url: "https://example.com",
    });
  });
  test("rejects a URL command without an argument", () => {
    expect(() => message_instruction("!url", "!")).toThrow();
  });
  test("rejects unknown commands", () => {
    expect(() => message_instruction("!unknown", "!")).toThrow();
  });
});

describe("isSavedClippingInstruction", () => {
  const existingIds = new Set(["123"]);

  test("skips only current URL commands with an existing clipping ID", () => {
    expect(
      isSavedClippingInstruction(
        "123",
        { kind: "url", url: "https://example.com" },
        existingIds,
      ),
    ).toBe(true);
    expect(
      isSavedClippingInstruction(
        "456",
        { kind: "url", url: "https://example.com" },
        existingIds,
      ),
    ).toBe(false);
  });

  test("does not skip regular content that previously used the same ID", () => {
    expect(
      isSavedClippingInstruction(
        "123",
        { kind: "message", markdown: "regular message" },
        existingIds,
      ),
    ).toBe(false);
  });

  test("does not skip a former URL command after the prefix changes", () => {
    const instruction = message_instruction("!url https://example.com", "?");

    expect(isSavedClippingInstruction("123", instruction, existingIds)).toBe(
      false,
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

    expect(
      createProcessedMessage("# title", true, message, "Asia/Tokyo"),
    ).toEqual({
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
      createProcessedMessage(
        "message",
        false,
        {
          ...base,
          author: {
            id: "author-id",
            username: "username",
            global_name: "Global name",
          },
        },
        "UTC",
      ).authorName,
    ).toBe("Global name");
    expect(
      createProcessedMessage(
        "message",
        false,
        {
          ...base,
          author: { id: "author-id", username: "username" },
        },
        "UTC",
      ).authorName,
    ).toBe("username");
    expect(
      createProcessedMessage(
        "message",
        false,
        {
          ...base,
          author: { id: "author-id" },
        },
        "UTC",
      ).authorName,
    ).toBe("author-id");
  });

  test("uses the requested local time zone for file names", () => {
    const processed = createProcessedMessage(
      "message",
      false,
      {
        id: "123",
        content: "content",
        timestamp: "2026-06-30T15:30:45.000Z",
      },
      "America/New_York",
    );

    expect(processed.fileName).toBe("20260630_113045_123");
  });

  test("rejects invalid timestamps", () => {
    expect(() =>
      createProcessedMessage(
        "message",
        false,
        {
          id: "123",
          content: "content",
          timestamp: "invalid",
        },
        "UTC",
      ),
    ).toThrow('Invalid Discord message timestamp: "invalid".');
  });
});
