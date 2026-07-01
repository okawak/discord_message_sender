// Settings for the Discord plugin
export interface DiscordChannelSettings {
  id: string;
  name: string;
  lastProcessedMessageId?: string;
}

export interface NotificationTemplates {
  saved: string;
  noNew: string;
}

export type MessageStorageMode = "individual" | "daily" | "weekly" | "monthly";

export const CURRENT_SETTINGS_SCHEMA_VERSION = 3;

export interface DiscordPluginSettings {
  settingsVersion: typeof CURRENT_SETTINGS_SCHEMA_VERSION;
  messageDirectoryName: string;
  clippingDirectoryName: string;
  botToken: string;
  channels: DiscordChannelSettings[];
  messagePrefix: string;
  messageStorageMode: MessageStorageMode;
  showAuthorNames: boolean;
  showMessageTime: boolean;
  enableAutoSyncOnStartup: boolean;
  sendSyncNotifications: boolean;
  notificationTemplates: NotificationTemplates;
}

export interface SettingsMigrationResult {
  settings: DiscordPluginSettings;
  didMigrate: boolean;
}

export interface MessageSyncSettingsSnapshot {
  botToken: string;
  messageDirectoryName: string;
  clippingDirectoryName: string;
  messagePrefix: string;
  messageStorageMode: MessageStorageMode;
  showAuthorNames: boolean;
  showMessageTime: boolean;
  sendSyncNotifications: boolean;
  notificationTemplates: NotificationTemplates;
  timeZone: string;
}

export const DEFAULT_NOTIFICATION_TEMPLATES: NotificationTemplates = {
  saved: "✅ {count} messages saved.",
  noNew: "⚠️ No new messages.",
};

// Default settings for the Discord plugin
export const DEFAULT_SETTINGS: DiscordPluginSettings = {
  settingsVersion: CURRENT_SETTINGS_SCHEMA_VERSION,
  messageDirectoryName: "DiscordLogs",
  clippingDirectoryName: "DiscordClippings",
  botToken: "",
  channels: [],
  messagePrefix: "!",
  messageStorageMode: "individual",
  showAuthorNames: false,
  showMessageTime: false,
  enableAutoSyncOnStartup: true,
  sendSyncNotifications: true,
  notificationTemplates: DEFAULT_NOTIFICATION_TEMPLATES,
};

export function migrateSettings(data: unknown): SettingsMigrationResult {
  const raw = isRecord(data) ? data : undefined;
  return {
    settings: normalizeSettings(raw),
    didMigrate: raw ? needsMigration(raw) : false,
  };
}

export function normalizeSettings(data: unknown): DiscordPluginSettings {
  const raw = isRecord(data) ? data : {};
  const legacyChannelId = readString(raw, "channelId");
  const legacyLastProcessedMessageId = readString(
    raw,
    "lastProcessedMessageId",
  );
  const channels = normalizeChannels(raw, {
    channelId: legacyChannelId,
    lastProcessedMessageId: legacyLastProcessedMessageId,
  });

  return {
    settingsVersion: CURRENT_SETTINGS_SCHEMA_VERSION,
    messageDirectoryName:
      readString(raw, "messageDirectoryName") ||
      DEFAULT_SETTINGS.messageDirectoryName,
    clippingDirectoryName:
      readString(raw, "clippingDirectoryName") ||
      DEFAULT_SETTINGS.clippingDirectoryName,
    botToken: readString(raw, "botToken"),
    channels,
    messagePrefix:
      readString(raw, "messagePrefix") || DEFAULT_SETTINGS.messagePrefix,
    messageStorageMode: normalizeMessageStorageMode(raw.messageStorageMode),
    showAuthorNames: readBoolean(
      raw,
      "showAuthorNames",
      DEFAULT_SETTINGS.showAuthorNames,
    ),
    showMessageTime: readBoolean(
      raw,
      "showMessageTime",
      DEFAULT_SETTINGS.showMessageTime,
    ),
    enableAutoSyncOnStartup: readBoolean(
      raw,
      "enableAutoSyncOnStartup",
      DEFAULT_SETTINGS.enableAutoSyncOnStartup,
    ),
    sendSyncNotifications: readBoolean(
      raw,
      "sendSyncNotifications",
      DEFAULT_SETTINGS.sendSyncNotifications,
    ),
    notificationTemplates: normalizeNotificationTemplates(raw),
  };
}

export function createMessageSyncSettingsSnapshot(
  settings: DiscordPluginSettings,
  timeZone: string,
): MessageSyncSettingsSnapshot {
  return {
    botToken: settings.botToken,
    messageDirectoryName: settings.messageDirectoryName,
    clippingDirectoryName: settings.clippingDirectoryName,
    messagePrefix: settings.messagePrefix,
    messageStorageMode: settings.messageStorageMode,
    showAuthorNames: settings.showAuthorNames,
    showMessageTime: settings.showMessageTime,
    sendSyncNotifications: settings.sendSyncNotifications,
    notificationTemplates: { ...settings.notificationTemplates },
    timeZone,
  };
}

export function getConfiguredChannels(
  channels: readonly DiscordChannelSettings[],
): DiscordChannelSettings[] {
  const ids = new Set<string>();
  return channels.filter((channel) => {
    if (!channel.id || ids.has(channel.id)) {
      return false;
    }
    ids.add(channel.id);
    return true;
  });
}

export function updateChannelId(
  channel: DiscordChannelSettings,
  id: string,
): void {
  if (id !== channel.id) {
    channel.id = id;
    delete channel.lastProcessedMessageId;
  }
}

function needsMigration(raw: Record<string, unknown>): boolean {
  return (
    raw.settingsVersion !== CURRENT_SETTINGS_SCHEMA_VERSION ||
    Object.hasOwn(raw, "channelId") ||
    Object.hasOwn(raw, "lastProcessedMessageId")
  );
}

function normalizeChannels(
  raw: Record<string, unknown>,
  legacy: { channelId: string; lastProcessedMessageId: string },
): DiscordChannelSettings[] {
  const channels = raw.channels;
  if (Array.isArray(channels)) {
    const normalized = channels
      .filter(isRecord)
      .map((channel) => {
        const id = readString(channel, "id");
        if (!id) {
          return undefined;
        }

        const lastProcessedMessageId = readString(
          channel,
          "lastProcessedMessageId",
        );
        return {
          id,
          name: readString(channel, "name"),
          ...(lastProcessedMessageId ? { lastProcessedMessageId } : {}),
        };
      })
      .filter((channel): channel is DiscordChannelSettings => !!channel);
    return getConfiguredChannels(normalized);
  }

  if (!legacy.channelId) {
    return [];
  }

  return [
    {
      id: legacy.channelId,
      name: "",
      ...(legacy.lastProcessedMessageId
        ? { lastProcessedMessageId: legacy.lastProcessedMessageId }
        : {}),
    },
  ];
}

function normalizeNotificationTemplates(
  raw: Record<string, unknown>,
): NotificationTemplates {
  const templates = raw.notificationTemplates;
  const value = isRecord(templates) ? templates : {};

  return {
    saved: readString(value, "saved") || DEFAULT_NOTIFICATION_TEMPLATES.saved,
    noNew: readString(value, "noNew") || DEFAULT_NOTIFICATION_TEMPLATES.noNew,
  };
}

function normalizeMessageStorageMode(value: unknown): MessageStorageMode {
  switch (value) {
    case "daily":
    case "weekly":
    case "monthly":
      return value;
    default:
      return "individual";
  }
}

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  return typeof value === "string" ? value.trim() : "";
}

function readBoolean(
  record: Record<string, unknown>,
  key: string,
  fallback: boolean,
): boolean {
  const value = record[key];
  return typeof value === "boolean" ? value : fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}
