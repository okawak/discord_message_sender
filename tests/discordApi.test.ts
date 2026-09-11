import { describe, expect, test } from "bun:test";
import {
  discord_messages_path as getChannelMessagesPath,
  discord_api_version as getDiscordApiVersion,
  discord_page_size as getDiscordMessagePageSize,
  discord_rate_limit_delay as getRateLimitDelay,
  discord_reset_delay as getRateLimitResetDelay,
} from "../pkg/parse_message.js";

describe("Discord message route", () => {
  test("uses API v10 and Discord's maximum page size", () => {
    expect(getDiscordApiVersion()).toBe(10);
    expect(getDiscordMessagePageSize()).toBe(100);
    expect(getChannelMessagesPath("123")).toBe(
      "/channels/123/messages?limit=100",
    );
    expect(getChannelMessagesPath("123", "456")).toBe(
      "/channels/123/messages?limit=100&before=456",
    );
  });
});

describe("getRateLimitDelay", () => {
  test("uses the Retry-After header without multiplying by retry count", () => {
    expect(getRateLimitDelay({ "Retry-After": "1.25" }, "")).toBe(1250);
  });

  test("falls back to the Discord retry_after response field", () => {
    expect(getRateLimitDelay({}, '{"retry_after":2.5}')).toBe(2500);
  });

  test("uses one second for invalid rate-limit data", () => {
    expect(getRateLimitDelay({}, '{"retry_after":"invalid"}')).toBe(1000);
  });
});

describe("getRateLimitResetDelay", () => {
  test("waits for an exhausted rate-limit bucket", () => {
    expect(
      getRateLimitResetDelay({
        "x-ratelimit-remaining": "0",
        "x-ratelimit-reset-after": "0.25",
      }),
    ).toBe(250);
  });

  test("does not wait while the bucket has remaining requests", () => {
    expect(
      getRateLimitResetDelay({
        "X-RateLimit-Remaining": "1",
        "X-RateLimit-Reset-After": "10",
      }),
    ).toBe(0);
  });
});
