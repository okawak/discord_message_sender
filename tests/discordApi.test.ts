import { describe, expect, test } from "bun:test";
import { getRateLimitDelay } from "../src/discordRateLimit";

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
