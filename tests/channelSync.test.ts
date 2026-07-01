import { describe, expect, test } from "bun:test";
import {
  getChannelSyncFailureNotice,
  getSyncCompletionNotice,
  syncChannelMessages,
  syncChannelsSequentially,
} from "../src/channelSync";
import { DiscordApiError } from "../src/discordApiError";
import { DISCORD_MESSAGE_PAGE_SIZE } from "../src/discordRoutes";
import type { DiscordMessage } from "../src/messages";
import type { DiscordChannelSettings } from "../src/settings";

const firstChannel = { id: "111", name: "first" };
const secondChannel = { id: "222", name: "second" };
const thirdChannel = { id: "333", name: "third" };
const channels = [
  firstChannel,
  secondChannel,
  thirdChannel,
] satisfies DiscordChannelSettings[];

function messagePage(messages: DiscordMessage[], nextRequestDelayMs = 0) {
  return { messages, nextRequestDelayMs };
}

function createMessage(id: number): DiscordMessage {
  return {
    id: id.toString(),
    content: `message ${id}`,
    timestamp: "2026-06-27T00:00:00Z",
  };
}

function createHistory(
  cursor: number,
  newMessageCount: number,
): DiscordMessage[] {
  return Array.from({ length: newMessageCount + 121 }, (_, index) =>
    createMessage(cursor + newMessageCount - index),
  );
}

function getHistoryPage(
  history: DiscordMessage[],
  before?: string,
): DiscordMessage[] {
  return history
    .filter((message) => !before || BigInt(message.id) < BigInt(before))
    .slice(0, DISCORD_MESSAGE_PAGE_SIZE);
}

describe("syncChannelsSequentially", () => {
  test("continues syncing after a channel-specific API failure", async () => {
    const synced: string[] = [];
    const forbidden = new DiscordApiError(
      403,
      "GET",
      "/channels/222/messages",
      '{"message":"Missing Access","code":50001}',
    );

    const summary = await syncChannelsSequentially(
      channels,
      async (channel) => {
        synced.push(channel.id);
        if (channel.id === "222") {
          throw forbidden;
        }
        return channel.id === "111" ? 2 : 1;
      },
    );

    expect(synced).toEqual(["111", "222", "333"]);
    expect(summary.processedMessageCount).toBe(3);
    expect(summary.failures).toEqual([
      { channel: secondChannel, error: forbidden },
    ]);
    const [failure] = summary.failures;
    expect(failure).toBeDefined();
    if (!failure) {
      throw new Error("Expected one channel failure.");
    }
    expect(getChannelSyncFailureNotice(failure)).toBe(
      'Discord sync skipped "second": missing Discord permission (View Channel / Read Message History).',
    );
    expect(getSyncCompletionNotice(summary)).toBe(
      "Discord sync finished. 3 messages saved; 1 channel failed.",
    );
  });

  test("stops immediately when the bot token is invalid", async () => {
    const synced: string[] = [];
    const unauthorized = new DiscordApiError(
      401,
      "GET",
      "/channels/111/messages",
      '{"message":"401: Unauthorized","code":0}',
    );

    let caught: unknown;
    try {
      await syncChannelsSequentially(channels, async (channel) => {
        synced.push(channel.id);
        throw unauthorized;
      });
    } catch (error) {
      caught = error;
    }

    expect(caught).toBe(unauthorized);
    expect(synced).toEqual(["111"]);
  });
});

describe("syncChannelMessages", () => {
  test("processes messages oldest first without checkpointing the notification", async () => {
    const channel = { id: "111", name: "first" };
    const processed: string[] = [];
    const cursors: string[] = [];
    const delays: number[] = [];
    const messages = [
      {
        id: "2",
        content: "second",
        timestamp: "2026-06-27T00:00:02Z",
      },
      {
        id: "1",
        content: "first",
        timestamp: "2026-06-27T00:00:01Z",
      },
    ];
    let fetchCount = 0;

    const count = await syncChannelMessages(
      {
        botToken: "token",
        channel,
        sendSyncNotifications: true,
        notificationTemplates: {
          saved: "{count} saved from {channelName}",
          noNew: "none",
        },
      },
      {
        fetchMessages: async () => {
          fetchCount++;
          return messagePage(messages);
        },
        postNotification: async (_token, _channelId, text) => {
          expect(text).toBe("2 saved from first");
          return {
            id: "notification",
            content: text,
            timestamp: "2026-06-27T00:00:03Z",
          };
        },
        processMessages: async (pageMessages) => {
          processed.push(...pageMessages.map((message) => message.id));
          return pageMessages.length;
        },
        persistCursor: async (_currentChannel, messageId) => {
          cursors.push(messageId);
        },
        sleep: async (milliseconds) => {
          delays.push(milliseconds);
        },
      },
    );

    expect(count).toBe(2);
    expect(processed).toEqual(["1", "2"]);
    expect(cursors).toEqual(["2"]);
    expect(fetchCount).toBe(1);
    expect(delays).toEqual([]);
  });

  test("limits the first sync to the latest 100 messages", async () => {
    const history = createHistory(10_000, 300);
    const processed: string[] = [];
    const requests: (string | undefined)[] = [];
    const cursors: string[] = [];

    const count = await syncChannelMessages(
      {
        botToken: "token",
        channel: { id: "111", name: "first" },
        sendSyncNotifications: false,
        notificationTemplates: { saved: "saved", noNew: "none" },
      },
      {
        fetchMessages: async (_token, _channelId, before) => {
          requests.push(before);
          return messagePage(getHistoryPage(history, before));
        },
        postNotification: async () => {
          throw new Error("Notification should not be sent.");
        },
        processMessages: async (pageMessages) => {
          processed.push(...pageMessages.map((message) => message.id));
          return pageMessages.length;
        },
        persistCursor: async (_channel, messageId) => {
          cursors.push(messageId);
        },
        sleep: async () => {},
      },
    );

    expect(count).toBe(100);
    expect(requests).toEqual([undefined]);
    expect(processed).toEqual(
      Array.from({ length: 100 }, (_, index) => (10_201 + index).toString()),
    );
    expect(cursors).toEqual(["10300"]);
  });

  for (const newMessageCount of [50, 100, 250, 1000]) {
    test(`syncs ${newMessageCount} new messages without gaps`, async () => {
      const cursor = 10_000;
      const history = createHistory(cursor, newMessageCount);
      const processed: string[] = [];
      const requests: (string | undefined)[] = [];
      const cursors: string[] = [];
      let processCalls = 0;

      const count = await syncChannelMessages(
        {
          botToken: "token",
          channel: {
            id: "111",
            name: "first",
            lastProcessedMessageId: cursor.toString(),
          },
          sendSyncNotifications: false,
          notificationTemplates: { saved: "saved", noNew: "none" },
        },
        {
          fetchMessages: async (_token, _channelId, before) => {
            requests.push(before);
            return messagePage(getHistoryPage(history, before));
          },
          postNotification: async () => {
            throw new Error("Notification should not be sent.");
          },
          processMessages: async (pageMessages) => {
            processCalls++;
            processed.push(...pageMessages.map((message) => message.id));
            return pageMessages.length;
          },
          persistCursor: async (_channel, messageId) => {
            cursors.push(messageId);
          },
          sleep: async () => {},
        },
      );

      expect(count).toBe(newMessageCount);
      expect(processed).toEqual(
        Array.from({ length: newMessageCount }, (_, index) =>
          (cursor + index + 1).toString(),
        ),
      );
      expect(requests.length).toBe(
        Math.floor(newMessageCount / DISCORD_MESSAGE_PAGE_SIZE) + 1,
      );
      expect(cursors.length).toBe(
        Math.ceil(newMessageCount / DISCORD_MESSAGE_PAGE_SIZE),
      );
      expect(processCalls).toBe(
        Math.ceil(newMessageCount / DISCORD_MESSAGE_PAGE_SIZE),
      );
      expect(cursors.at(-1)).toBe((cursor + newMessageCount).toString());
    });
  }

  test("waits only when the current rate-limit bucket is exhausted", async () => {
    const cursor = 10_000;
    const history = createHistory(cursor, 150);
    const delays: number[] = [];

    await syncChannelMessages(
      {
        botToken: "token",
        channel: {
          id: "111",
          name: "first",
          lastProcessedMessageId: cursor.toString(),
        },
        sendSyncNotifications: false,
        notificationTemplates: { saved: "saved", noNew: "none" },
      },
      {
        fetchMessages: async (_token, _channelId, before) =>
          messagePage(getHistoryPage(history, before), before ? 0 : 250),
        postNotification: async () => {
          throw new Error("Notification should not be sent.");
        },
        processMessages: async (pageMessages) => pageMessages.length,
        persistCursor: async () => {},
        sleep: async (milliseconds) => {
          delays.push(milliseconds);
        },
      },
    );

    expect(delays).toEqual([250]);
  });

  test("persists the fetched cursor before a notification failure", async () => {
    const channel = { id: "111", name: "first" };
    const cursors: string[] = [];

    let caught: unknown;
    try {
      await syncChannelMessages(
        {
          botToken: "token",
          channel,
          sendSyncNotifications: true,
          notificationTemplates: {
            saved: "saved",
            noNew: "none",
          },
        },
        {
          fetchMessages: async () =>
            messagePage([
              {
                id: "message",
                content: "content",
                timestamp: "2026-06-27T00:00:00Z",
              },
            ]),
          postNotification: async () => {
            throw new Error("Missing Send Messages");
          },
          processMessages: async (pageMessages) => pageMessages.length,
          persistCursor: async (_currentChannel, messageId) => {
            cursors.push(messageId);
          },
          sleep: async () => {},
        },
      );
    } catch (error) {
      caught = error;
    }

    expect(caught).toBeInstanceOf(Error);
    expect(cursors).toEqual(["message"]);
  });

  test("does not persist the cursor when message processing fails", async () => {
    const cursors: string[] = [];
    const processingError = new Error("URL fetch failed");

    let caught: unknown;
    try {
      await syncChannelMessages(
        {
          botToken: "token",
          channel: { id: "111", name: "first" },
          sendSyncNotifications: false,
          notificationTemplates: {
            saved: "saved",
            noNew: "none",
          },
        },
        {
          fetchMessages: async () =>
            messagePage([
              {
                id: "message",
                content: "!url https://example.com",
                timestamp: "2026-06-27T00:00:00Z",
              },
            ]),
          postNotification: async () => {
            throw new Error("Notification should not be sent.");
          },
          processMessages: async () => {
            throw processingError;
          },
          persistCursor: async (_currentChannel, messageId) => {
            cursors.push(messageId);
          },
          sleep: async () => {},
        },
      );
    } catch (error) {
      caught = error;
    }

    expect(caught).toBe(processingError);
    expect(cursors).toEqual([]);
  });

  test("does not count messages that were already saved", async () => {
    const cursors: string[] = [];

    const count = await syncChannelMessages(
      {
        botToken: "token",
        channel: { id: "111", name: "first" },
        sendSyncNotifications: false,
        notificationTemplates: {
          saved: "saved",
          noNew: "none",
        },
      },
      {
        fetchMessages: async () =>
          messagePage([
            {
              id: "duplicate",
              content: "existing",
              timestamp: "2026-06-27T00:00:00Z",
            },
          ]),
        postNotification: async () => {
          throw new Error("Notification should not be sent.");
        },
        processMessages: async () => 0,
        persistCursor: async (_currentChannel, messageId) => {
          cursors.push(messageId);
        },
        sleep: async () => {},
      },
    );

    expect(count).toBe(0);
    expect(cursors).toEqual(["duplicate"]);
  });

  test("does not post a notification when notifications are disabled", async () => {
    const cursors: string[] = [];
    let notificationCount = 0;

    const count = await syncChannelMessages(
      {
        botToken: "token",
        channel: { id: "111", name: "first" },
        sendSyncNotifications: false,
        notificationTemplates: {
          saved: "saved",
          noNew: "none",
        },
      },
      {
        fetchMessages: async () =>
          messagePage([
            {
              id: "message",
              content: "content",
              timestamp: "2026-06-27T00:00:00Z",
            },
          ]),
        postNotification: async () => {
          notificationCount++;
          throw new Error("Notification should not be sent.");
        },
        processMessages: async (pageMessages) => pageMessages.length,
        persistCursor: async (_currentChannel, messageId) => {
          cursors.push(messageId);
        },
        sleep: async () => {},
      },
    );

    expect(count).toBe(1);
    expect(cursors).toEqual(["message"]);
    expect(notificationCount).toBe(0);
  });
});

describe("DiscordApiError", () => {
  test("includes Discord response details without losing typed fields", () => {
    const error = new DiscordApiError(
      403,
      "POST",
      "/channels/222/messages",
      '{"message":"Missing Permissions","code":50013}',
    );

    expect(error).toBeInstanceOf(DiscordApiError);
    expect(error.status).toBe(403);
    expect(error.method).toBe("POST");
    expect(error.message).toContain("Discord says: Missing Permissions");
  });
});
