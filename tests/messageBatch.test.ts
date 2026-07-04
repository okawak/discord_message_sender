import { describe, expect, test } from "bun:test";
import { processDiscordMessageBatch } from "../src/messageBatch";
import type { DiscordMessage, ProcessedMessage } from "../src/messages";

function message(id: string, bot = false): DiscordMessage {
  return {
    id,
    content: id,
    timestamp: "2026-07-01T00:00:00.000Z",
    author: { id: `author-${id}`, bot },
  };
}

function processed(source: DiscordMessage): ProcessedMessage {
  return {
    messageId: source.id,
    timestamp: source.timestamp,
    authorId: source.author?.id ?? "",
    authorName: source.author?.id ?? "Unknown",
    markdown: source.content,
    isClipping: false,
    fileName: `20260701_090000_${source.id}`,
  };
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
});
