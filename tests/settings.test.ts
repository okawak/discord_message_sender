import { describe, expect, test } from "bun:test";
import {
  createChannelDirectory,
  getChannelDisplayName,
  getChannelPathSegment,
} from "../src/channelPaths";
import { renderNotificationTemplate } from "../src/notificationTemplates";
import { normalizeSettings } from "../src/settings";

describe("normalizeSettings", () => {
  test("migrates legacy channel fields into channels", () => {
    const settings = normalizeSettings({
      messageDirectoryName: "Logs",
      clippingDirectoryName: "Clips",
      botToken: "token",
      channelId: " 1234567890 ",
      lastProcessedMessageId: " 9876543210 ",
      messagePrefix: "!",
      enableAutoSyncOnStartup: false,
    });

    expect(settings.channels).toEqual([
      {
        id: "1234567890",
        name: "",
        lastProcessedMessageId: "9876543210",
      },
    ]);
    expect(settings.channelId).toBe("1234567890");
    expect(settings.enableAutoSyncOnStartup).toBe(false);
  });

  test("keeps valid channels and drops blank channel ids", () => {
    const settings = normalizeSettings({
      channels: [
        { id: "111", name: " inbox " },
        { id: "   ", name: "ignored" },
        { id: "222", lastProcessedMessageId: "999" },
      ],
    });

    expect(settings.channels).toEqual([
      { id: "111", name: "inbox" },
      { id: "222", name: "", lastProcessedMessageId: "999" },
    ]);
    expect(settings.channelId).toBe("111");
  });

  test("fills default notification templates", () => {
    const settings = normalizeSettings({
      notificationTemplates: {
        saved: "Saved {count} from {channelName}",
      },
    });

    expect(settings.notificationTemplates.saved).toBe(
      "Saved {count} from {channelName}",
    );
    expect(settings.notificationTemplates.noNew).toBe("⚠️ No new messages.");
  });
});

describe("channel paths", () => {
  test("uses the channel name for display and safe path segment", () => {
    const channel = { id: "123", name: "notes/inbox #1" };

    expect(getChannelDisplayName(channel)).toBe("notes/inbox #1");
    expect(getChannelPathSegment(channel)).toBe("notes-inbox -1");
    expect(createChannelDirectory("DiscordLogs/", channel)).toBe(
      "DiscordLogs/notes-inbox -1",
    );
  });

  test("falls back to channel id when the name is blank", () => {
    const channel = { id: "123", name: "" };

    expect(getChannelDisplayName(channel)).toBe("123");
    expect(createChannelDirectory("", channel)).toBe("123");
  });
});

describe("renderNotificationTemplate", () => {
  test("replaces supported variables", () => {
    const text = renderNotificationTemplate(
      "{count} saved from {channelName} ({channelId})",
      {
        count: 3,
        channel: { id: "123", name: "inbox" },
      },
    );

    expect(text).toBe("3 saved from inbox (123)");
  });
});
