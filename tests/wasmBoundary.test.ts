import { describe, expect, test } from "bun:test";
import {
  change_channel_id,
  type DiscordChannelSettings,
  type DiscordMessage,
  duplicate_channel_path,
  type LocalDateTime,
  local_date_time,
  message_instruction,
  normalize_settings,
  processed_message,
} from "../pkg/parse_message.js";
import { getConfiguredChannels, normalizeSettings } from "../src/settings";
import { initWasmCore } from "../src/wasmCore";

describe("Rust/TypeScript boundary", () => {
  test("shares one in-flight initialization", async () => {
    expect(initWasmCore()).toBe(initWasmCore());
    await initWasmCore();
  });
  test("preserves references used to persist live sync cursors", () => {
    const settings = normalizeSettings({
      channels: [{ id: "123", name: "notes" }],
    });
    const channel = getConfiguredChannels(settings.channels)[0];
    expect(channel).toBe(settings.channels[0]);
    if (!channel) throw new Error("Missing test channel");
    channel.lastProcessedMessageId = "456";
    expect(settings.channels[0]?.lastProcessedMessageId).toBe("456");
    expect(change_channel_id(channel, "123").lastProcessedMessageId).toBe(
      "456",
    );
  });
  test("returns plain objects and arrays that Obsidian can persist", () => {
    const settings = normalize_settings({ channels: [{ id: "123" }] });
    expect(settings).not.toBeInstanceOf(Map);
    expect(Array.isArray(settings.channels)).toBe(true);
    expect(JSON.parse(JSON.stringify(settings))).toEqual(settings);
    expect(
      Object.hasOwn(settings.channels[0] ?? {}, "lastProcessedMessageId"),
    ).toBe(false);
  });
  test("malformed typed inputs throw without poisoning the WASM instance", () => {
    expect(() =>
      processed_message(
        "hello",
        false,
        { id: 123 } as unknown as DiscordMessage,
        "UTC",
      ),
    ).toThrow();
    expect(message_instruction("hello", "!")).toEqual({
      kind: "message",
      markdown: "hello",
    });
  });
  test("repeated conversion errors release temporary WASM allocations", async () => {
    const wasm = await initWasmCore();
    const invalid = {
      id: "x".repeat(10_000),
      content: "a".repeat(10_000),
      timestamp: 42,
    } as unknown as DiscordMessage;
    const fail = () => {
      try {
        processed_message("hello", false, invalid, "UTC");
      } catch {
        return;
      }
      throw new Error("Malformed input was accepted");
    };
    for (let i = 0; i < 100; i++) fail();
    const afterWarmup = wasm.memory.buffer.byteLength;
    for (let i = 0; i < 1_000; i++) fail();
    expect(wasm.memory.buffer.byteLength).toBeLessThanOrEqual(
      afterWarmup + 65_536,
    );
  });
  test("uses host Intl across the repeated DST hour", () => {
    expect(
      local_date_time("2026-11-01T05:30:00Z", "America/New_York").time,
    ).toBe("01:30");
    expect(
      local_date_time("2026-11-01T06:30:00Z", "America/New_York").time,
    ).toBe("01:30");
  });
});

test("host date conversion and Unicode normalization match pre-migration fixtures", async () => {
  const fixtures: {
    dates: { timestamp: string; zone: string; expected: LocalDateTime }[];
    duplicates: { channels: DiscordChannelSettings[]; expected?: string }[];
  } = await Bun.file(
    new URL(
      "../crates/domain/tests/fixtures/compatibility.json",
      import.meta.url,
    ),
  ).json();
  for (const entry of fixtures.dates)
    expect(local_date_time(entry.timestamp, entry.zone)).toEqual(
      entry.expected,
    );
  for (const entry of fixtures.duplicates)
    expect(duplicate_channel_path(entry.channels)).toBe(entry.expected);
});

test("host date conversion handles midnight, fractional offsets, and year zero", () => {
  expect(local_date_time("2026-01-01T00:00:00.999Z", "UTC").fileTimestamp).toBe(
    "20260101_000000",
  );
  expect(local_date_time("2026-01-01T00:00:00Z", "Asia/Kathmandu").time).toBe(
    "05:45",
  );
  expect(local_date_time("2026-01-01T00:00:00Z", "Pacific/Chatham").time).toBe(
    "13:45",
  );
  expect(local_date_time("0000-01-01T00:00:00Z", "UTC").date).toBe(
    "0000-01-01",
  );
  expect(local_date_time("2016-12-31T23:59:60Z", "UTC").fileTimestamp).toBe(
    "20161231_235960",
  );
});

test("invalid zones throw without poisoning the host formatter cache or leaking memory", async () => {
  const timestamp = "2026-01-01T00:00:00Z";
  const wasm = await initWasmCore();
  const invalid = () =>
    expect(() => local_date_time(timestamp, "Invalid/Zone")).toThrow(
      "Invalid time zone: Invalid/Zone",
    );
  for (let i = 0; i < 100; i++) invalid();
  const afterWarmup = wasm.memory.buffer.byteLength;
  for (let i = 0; i < 1_000; i++) invalid();
  expect(wasm.memory.buffer.byteLength).toBeLessThanOrEqual(
    afterWarmup + 65_536,
  );
  expect(local_date_time(timestamp, "Asia/Tokyo").time).toBe("09:00");
  expect(local_date_time(timestamp, "UTC").time).toBe("00:00");
  expect(local_date_time(timestamp, "Asia/Tokyo").time).toBe("09:00");
});
