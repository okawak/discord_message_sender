import { describe, expect, test } from "bun:test";
import {
  getChannelSyncFailureNotice,
  getSyncCompletionNotice,
  syncChannelMessages,
  syncChannelsSequentially,
} from "../src/channelSync";
import { DiscordApiError } from "../src/discordApiError";
import type { DiscordChannelSettings } from "../src/settings";

const firstChannel = { id: "111", name: "first" };
const secondChannel = { id: "222", name: "second" };
const thirdChannel = { id: "333", name: "third" };
const channels = [
  firstChannel,
  secondChannel,
  thirdChannel,
] satisfies DiscordChannelSettings[];

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
    const fetches = [
      [
        {
          id: "newest",
          content: "second",
          timestamp: "2026-06-27T00:00:02Z",
        },
        {
          id: "oldest",
          content: "first",
          timestamp: "2026-06-27T00:00:01Z",
        },
      ],
      [],
    ];

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
        fetchMessages: async () => fetches.shift() ?? [],
        postNotification: async (_token, _channelId, text) => {
          expect(text).toBe("2 saved from first");
          return {
            id: "notification",
            content: text,
            timestamp: "2026-06-27T00:00:03Z",
          };
        },
        processMessage: async (message) => {
          processed.push(message.id);
          return true;
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
    expect(processed).toEqual(["oldest", "newest"]);
    expect(cursors).toEqual(["newest"]);
    expect(delays).toEqual([50, 50, 1000]);
  });

  test("persists the fetched cursor before a notification failure", async () => {
    const channel = { id: "111", name: "first" };
    const cursors: string[] = [];
    let fetchCount = 0;

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
          fetchMessages: async () => {
            fetchCount++;
            return fetchCount === 1
              ? [
                  {
                    id: "message",
                    content: "content",
                    timestamp: "2026-06-27T00:00:00Z",
                  },
                ]
              : [];
          },
          postNotification: async () => {
            throw new Error("Missing Send Messages");
          },
          processMessage: async () => true,
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
          fetchMessages: async () => [
            {
              id: "message",
              content: "!url https://example.com",
              timestamp: "2026-06-27T00:00:00Z",
            },
          ],
          postNotification: async () => {
            throw new Error("Notification should not be sent.");
          },
          processMessage: async () => {
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
    let fetchCount = 0;

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
        fetchMessages: async () => {
          fetchCount++;
          return fetchCount === 1
            ? [
                {
                  id: "duplicate",
                  content: "existing",
                  timestamp: "2026-06-27T00:00:00Z",
                },
              ]
            : [];
        },
        postNotification: async () => {
          throw new Error("Notification should not be sent.");
        },
        processMessage: async () => false,
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
    let fetchCount = 0;
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
        fetchMessages: async () => {
          fetchCount++;
          return fetchCount === 1
            ? [
                {
                  id: "message",
                  content: "content",
                  timestamp: "2026-06-27T00:00:00Z",
                },
              ]
            : [];
        },
        postNotification: async () => {
          notificationCount++;
          throw new Error("Notification should not be sent.");
        },
        processMessage: async () => true,
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
