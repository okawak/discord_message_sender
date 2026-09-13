import { describe, expect, test } from "bun:test";
import {
  discord_create_message_body as createDiscordMessageBody,
  decode_discord_message as decodeDiscordMessage,
  decode_discord_messages as decodeDiscordMessages,
  discord_retry_decision,
  discord_messages_path as getChannelMessagesPath,
  discord_api_version as getDiscordApiVersion,
  discord_page_size as getDiscordMessagePageSize,
  discord_rate_limit_delay as getRateLimitDelay,
  discord_reset_delay as getRateLimitResetDelay,
} from "../pkg/parse_message.js";

describe("Discord JSON boundary", () => {
  const message = {
    id: "1",
    content: "hello",
    timestamp: "2026-01-01T00:00:00Z",
  };

  test("decodes typed message responses in Rust", () => {
    expect(decodeDiscordMessage(JSON.stringify(message))).toEqual(message);
    expect(decodeDiscordMessages(JSON.stringify([message]))).toEqual([message]);
  });

  test("rejects malformed Discord responses", () => {
    expect(() => decodeDiscordMessage('{"id":1}')).toThrow(
      "Discord API returned an invalid message.",
    );
    expect(() => decodeDiscordMessages("{}")).toThrow(
      "Discord API returned an invalid message list.",
    );
  });

  test("rejects invalid JSON and preserves message strings without JS coercion", () => {
    for (const invalid of ["", "<html>error</html>", "null", '{"id":']) {
      expect(() => decodeDiscordMessage(invalid)).toThrow(
        "Discord API returned an invalid message.",
      );
      expect(() => decodeDiscordMessages(invalid)).toThrow(
        "Discord API returned an invalid message list.",
      );
    }
    const original = {
      ...message,
      id: "900719925474099312345",
      content: '日本語\n"quoted" \\ emoji 🎉',
      author: { id: "123", global_name: null },
    };
    // Optional null fields are omitted by the existing Rust serialization.
    expect(decodeDiscordMessage(JSON.stringify(original))).toEqual({
      ...original,
      author: { id: "123" },
    });
  });

  test("serializes notification request bodies in Rust", () => {
    expect(createDiscordMessageBody('line 1\n"quoted"')).toBe(
      '{"content":"line 1\\n\\"quoted\\""}',
    );
  });
});

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

describe("Discord response header boundary", () => {
  const headers = {
    "set-cookie": ["session=example; HttpOnly", "other=example; Secure"],
    "rEtRy-AfTeR": "1.25",
    "X-RateLimit-Remaining": "0",
    "x-ratelimit-reset-after": "0.25",
  };

  test("accepts array-valued cookies on success, failure, and retry responses", () => {
    expect(discord_retry_decision(200, 0, headers, "[]")).toEqual({
      kind: "success",
    });
    expect(discord_retry_decision(403, 0, headers, "")).toEqual({
      kind: "fail",
    });
    expect(discord_retry_decision(429, 0, headers, "")).toEqual({
      kind: "retry",
      delay: 1250,
      rateLimited: true,
    });
    expect(getRateLimitDelay(headers, "")).toBe(1250);
    expect(getRateLimitResetDelay(headers)).toBe(250);
  });

  test("does not read unrelated response header values", () => {
    const responseHeaders = {
      ...headers,
      get "set-cookie"(): string[] {
        throw new Error("Unrelated header must not be read");
      },
    };
    expect(discord_retry_decision(200, 0, responseHeaders, "[]")).toEqual({
      kind: "success",
    });
    expect(getRateLimitDelay(responseHeaders, "")).toBe(1250);
    expect(getRateLimitResetDelay(responseHeaders)).toBe(250);
  });

  test("falls back when rate-limit headers are not strings", () => {
    const malformed = {
      "Retry-After": ["invalid"],
      "X-RateLimit-Remaining": null,
      "X-RateLimit-Reset-After": {},
    };
    expect(getRateLimitDelay(malformed, '{"retry_after":2.5}')).toBe(2500);
    expect(getRateLimitResetDelay(malformed)).toBe(0);
    expect(discord_retry_decision(429, 0, malformed, "")).toEqual({
      kind: "retry",
      delay: 1000,
      rateLimited: true,
    });
  });
});
