import { describe, expect, test } from "bun:test";
import {
  complete_message_instruction,
  type DiscordMessage,
  message_instruction,
  type ProcessedMessageList,
} from "../pkg/parse_message.js";
import { processDiscordMessageBatch } from "../src/channelSync";

function message(id: string, bot = false): DiscordMessage {
  return {
    id,
    content: id,
    timestamp: "2026-07-01T00:00:00.000Z",
    author: { id: `author-${id}`, bot },
  };
}

function processed(
  source: DiscordMessage,
  clippingAlreadySaved = false,
): ProcessedMessageList {
  return complete_message_instruction(
    message_instruction(source.content, "!", clippingAlreadySaved),
    source,
    undefined,
    "Asia/Tokyo",
  );
}

describe("processDiscordMessageBatch", () => {
  test("saves parsed messages before propagating a later parse failure", async () => {
    const parseError = new Error("URL fetch failed");
    const saved: string[][] = [];

    await expect(
      processDiscordMessageBatch(
        [message("1"), message("2"), message("3")],
        async (source) => {
          if (source.id === "3") {
            throw parseError;
          }
          return processed(source);
        },
        async (messages) => {
          saved.push(messages.map(({ messageId }) => messageId));
          return messages.length;
        },
      ),
    ).rejects.toBe(parseError);

    expect(saved).toEqual([["1", "2"]]);
  });

  test("saves a successful page once and skips bot messages", async () => {
    const saved: string[][] = [];
    const count = await processDiscordMessageBatch(
      [message("1"), message("bot", true), message("2")],
      async (source) => processed(source),
      async (messages) => {
        saved.push(messages.map(({ messageId }) => messageId));
        return messages.length;
      },
    );

    expect(count).toBe(2);
    expect(saved).toEqual([["1", "2"]]);
  });

  test("does not save a clipping omitted by the parser", async () => {
    const parsed: string[] = [];
    const saved: string[][] = [];
    const existingClippingIds = new Set(["existing"]);
    const existingClipping = {
      ...message("existing"),
      content: "!url https://example.com/article",
    };

    const count = await processDiscordMessageBatch(
      [existingClipping, message("new")],
      async (source) => {
        parsed.push(source.id);
        return processed(source, existingClippingIds.has(source.id));
      },
      async (messages) => {
        saved.push(messages.map(({ messageId }) => messageId));
        return messages.length;
      },
    );

    expect(count).toBe(1);
    expect(parsed).toEqual(["existing", "new"]);
    expect(saved).toEqual([["new"]]);
  });

  test("omits empty content selected out by Rust", async () => {
    const saved: string[][] = [];
    const count = await processDiscordMessageBatch(
      [{ ...message("empty"), content: "" }, message("new")],
      async (source) => processed(source),
      async (messages) => {
        saved.push(messages.map(({ messageId }) => messageId));
        return messages.length;
      },
    );
    expect(count).toBe(1);
    expect(saved).toEqual([["new"]]);
  });
});
