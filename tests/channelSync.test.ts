import { describe, expect, test } from "bun:test";
import {
  getChannelSyncFailureNotice,
  getSyncCompletionNotice,
  syncChannelsSequentially,
} from "../src/channelSync";
import { createDiscordApiError, DiscordApiError } from "../src/discordApiError";
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
    const forbidden = createDiscordApiError(
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
    const unauthorized = createDiscordApiError(
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

describe("DiscordApiError", () => {
  test("includes Discord response details without losing typed fields", () => {
    const error = createDiscordApiError(
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
