import { describe, expect, test } from "bun:test";
import {
  processed_message as createProcessedMessage,
  type DiscordMessage,
  message_instruction,
} from "../pkg/parse_message.js";

describe("Rust message instructions", () => {
  test("returns a typed regular-message instruction", () => {
    expect(message_instruction("# title", "!", false)).toEqual({
      kind: "message",
      markdown: "# title",
    });
  });
  test("returns a typed URL instruction", () => {
    expect(message_instruction("!url https://example.com", "!", false)).toEqual(
      {
        kind: "url",
        url: "https://example.com",
      },
    );
  });
  test("rejects a URL command without an argument", () => {
    expect(() => message_instruction("!url", "!", false)).toThrow();
  });
  test("rejects unknown commands", () => {
    expect(() => message_instruction("!unknown", "!", false)).toThrow();
  });
});

describe("saved clipping instructions", () => {
  test("skips only current URL commands whose clipping already exists", () => {
    expect(message_instruction("!url https://example.com", "!", true)).toEqual({
      kind: "skip",
    });
    expect(message_instruction("regular message", "!", true)).toEqual({
      kind: "message",
      markdown: "regular message",
    });
    expect(message_instruction("!url https://example.com", "?", true)).toEqual({
      kind: "message",
      markdown: "!url https://example.com",
    });
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
