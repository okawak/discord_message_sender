import { describe, expect, test } from "bun:test";
import {
  createChannelDirectory,
  findDuplicateChannelPathSegment,
  getChannelDisplayName,
  getChannelNameValidationError,
  getChannelPathSegment,
  INVALID_CHANNEL_NAME_MESSAGE,
} from "../src/channelPaths";
import { renderNotificationTemplate } from "../src/notificationTemplates";
import {
  CURRENT_SETTINGS_SCHEMA_VERSION,
  createMessageSyncSettingsSnapshot,
  type DiscordChannelSettings,
  getConfiguredChannels,
  migrateSettings,
  normalizeSettings,
  updateChannelId,
} from "../src/settings";

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
    expect(settings.settingsVersion).toBe(CURRENT_SETTINGS_SCHEMA_VERSION);
    expect(Object.hasOwn(settings, "channelId")).toBe(false);
    expect(Object.hasOwn(settings, "lastProcessedMessageId")).toBe(false);
    expect(settings.enableAutoSyncOnStartup).toBe(false);
    expect(settings.sendSyncNotifications).toBe(true);
    expect(settings.messageStorageMode).toBe("individual");
    expect(settings.showAuthorNames).toBe(false);
    expect(settings.showMessageTime).toBe(false);
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
  });

  test("keeps only the first configured channel for each id", () => {
    const first = { id: "111", name: "first" };
    expect(
      getConfiguredChannels([
        first,
        { id: "", name: "blank" },
        { id: "111", name: "duplicate" },
        { id: "222", name: "second" },
      ]),
    ).toEqual([first, { id: "222", name: "second" }]);
  });

  test("clears the sync cursor when a channel id changes", () => {
    const channel: DiscordChannelSettings = {
      id: "111",
      name: "inbox",
      lastProcessedMessageId: "999",
    };

    updateChannelId(channel, "222");

    expect(channel).toEqual({ id: "222", name: "inbox" });
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

  test("keeps disabled sync notifications", () => {
    expect(
      normalizeSettings({ sendSyncNotifications: false }).sendSyncNotifications,
    ).toBe(false);
  });

  test("keeps supported storage modes and rejects unknown values", () => {
    for (const mode of ["daily", "weekly", "monthly"] as const) {
      expect(
        normalizeSettings({ messageStorageMode: mode }).messageStorageMode,
      ).toBe(mode);
    }
    expect(
      normalizeSettings({ messageStorageMode: "yearly" }).messageStorageMode,
    ).toBe("individual");
  });

  test("keeps aggregated log display settings", () => {
    const settings = normalizeSettings({
      showAuthorNames: true,
      showMessageTime: true,
    });

    expect(settings.showAuthorNames).toBe(true);
    expect(settings.showMessageTime).toBe(true);
  });
});

describe("migrateSettings", () => {
  test("enables sync notifications when migrating v1 settings", () => {
    const migration = migrateSettings({
      settingsVersion: 1,
      channels: [{ id: "123", name: "inbox" }],
    });

    expect(migration.didMigrate).toBe(true);
    expect(migration.settings.settingsVersion).toBe(
      CURRENT_SETTINGS_SCHEMA_VERSION,
    );
    expect(migration.settings.sendSyncNotifications).toBe(true);
  });

  test("migrates v2 settings without changing channels or cursors", () => {
    const migration = migrateSettings({
      settingsVersion: 2,
      channels: [
        {
          id: "123",
          name: "notes",
          lastProcessedMessageId: "456",
        },
      ],
    });

    expect(migration.didMigrate).toBe(true);
    expect(migration.settings.channels).toEqual([
      {
        id: "123",
        name: "notes",
        lastProcessedMessageId: "456",
      },
    ]);
    expect(migration.settings.messageStorageMode).toBe("individual");
  });

  test("rewrites v0.2.8 settings into the current schema", () => {
    const migration = migrateSettings({
      messageDirectoryName: "DiscordLogs",
      clippingDirectoryName: "DiscordClippings",
      botToken: "token",
      channelId: "123",
      messagePrefix: "!",
      enableAutoSyncOnStartup: true,
      lastProcessedMessageId: "456",
    });

    expect(migration.didMigrate).toBe(true);
    expect(migration.settings).toMatchObject({
      settingsVersion: CURRENT_SETTINGS_SCHEMA_VERSION,
      channels: [
        {
          id: "123",
          name: "",
          lastProcessedMessageId: "456",
        },
      ],
    });
    expect(Object.hasOwn(migration.settings, "channelId")).toBe(false);
    expect(Object.hasOwn(migration.settings, "lastProcessedMessageId")).toBe(
      false,
    );
  });

  test("is idempotent after migrated settings are persisted", () => {
    const first = migrateSettings({
      channelId: "123",
      lastProcessedMessageId: "456",
    });
    const second = migrateSettings(first.settings);

    expect(first.didMigrate).toBe(true);
    expect(second.didMigrate).toBe(false);
    expect(second.settings).toEqual(first.settings);
  });

  test("removes stale legacy fields from an existing channels schema", () => {
    const migration = migrateSettings({
      settingsVersion: CURRENT_SETTINGS_SCHEMA_VERSION,
      channels: [{ id: "new-channel", name: "new" }],
      channelId: "stale-channel",
      lastProcessedMessageId: "stale-cursor",
    });

    expect(migration.didMigrate).toBe(true);
    expect(migration.settings.channels).toEqual([
      { id: "new-channel", name: "new" },
    ]);
    expect(Object.hasOwn(migration.settings, "channelId")).toBe(false);
    expect(Object.hasOwn(migration.settings, "lastProcessedMessageId")).toBe(
      false,
    );
  });
});

describe("createMessageSyncSettingsSnapshot", () => {
  test("keeps one sync isolated from later setting changes", () => {
    const settings = normalizeSettings({
      botToken: "token",
      messageDirectoryName: "Logs",
      clippingDirectoryName: "Clips",
      messagePrefix: "!",
      messageStorageMode: "weekly",
      showAuthorNames: true,
      showMessageTime: true,
      sendSyncNotifications: true,
      notificationTemplates: { saved: "saved", noNew: "none" },
    });
    const snapshot = createMessageSyncSettingsSnapshot(settings, "Asia/Tokyo");

    settings.messageStorageMode = "monthly";
    settings.showAuthorNames = false;
    settings.notificationTemplates.saved = "changed";

    expect(snapshot).toMatchObject({
      messageStorageMode: "weekly",
      showAuthorNames: true,
      showMessageTime: true,
      timeZone: "Asia/Tokyo",
      notificationTemplates: { saved: "saved", noNew: "none" },
    });
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

  test("falls back to the channel id for unsafe traversal names", () => {
    expect(getChannelPathSegment({ id: "333", name: ".." })).toBe("333");
  });

  test("rejects forbidden and reserved channel names", () => {
    for (const name of [
      "\\",
      "/",
      ":",
      "*",
      "?",
      '"',
      "<",
      ">",
      "|",
      "#",
      "^",
      "[",
      "]",
    ]) {
      expect(getChannelNameValidationError(`name${name}`)).toBe(
        INVALID_CHANNEL_NAME_MESSAGE,
      );
    }
    expect(getChannelNameValidationError(".")).toBe(
      INVALID_CHANNEL_NAME_MESSAGE,
    );
    expect(getChannelNameValidationError("..")).toBe(
      INVALID_CHANNEL_NAME_MESSAGE,
    );
    expect(getChannelNameValidationError("inbox")).toBeUndefined();
  });

  test("detects duplicate sanitized channel folders", () => {
    expect(
      findDuplicateChannelPathSegment([
        { id: "111", name: "a/b" },
        { id: "222", name: "A:B" },
      ]),
    ).toBe("A-B");
  });

  test("allows distinct channel folders", () => {
    expect(
      findDuplicateChannelPathSegment([
        { id: "111", name: "first" },
        { id: "222", name: "second" },
      ]),
    ).toBeUndefined();
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
