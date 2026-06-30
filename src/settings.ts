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

export const CURRENT_SETTINGS_VERSION = 2;

export interface DiscordPluginSettings {
  settingsVersion: typeof CURRENT_SETTINGS_VERSION;
  messageDirectoryName: string;
  clippingDirectoryName: string;
  botToken: string;
  channels: DiscordChannelSettings[];
  messagePrefix: string;
  enableAutoSyncOnStartup: boolean;
  sendSyncNotifications: boolean;
  notificationTemplates: NotificationTemplates;
}

export interface SettingsMigrationResult {
  settings: DiscordPluginSettings;
  didMigrate: boolean;
}

export const DEFAULT_NOTIFICATION_TEMPLATES: NotificationTemplates = {
  saved: "✅ {count} messages saved.",
  noNew: "⚠️ No new messages.",
};

// Default settings for the Discord plugin
export const DEFAULT_SETTINGS: DiscordPluginSettings = {
  settingsVersion: CURRENT_SETTINGS_VERSION,
  messageDirectoryName: "DiscordLogs",
  clippingDirectoryName: "DiscordClippings",
  botToken: "",
  channels: [],
  messagePrefix: "!",
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
    settingsVersion: CURRENT_SETTINGS_VERSION,
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
    raw.settingsVersion !== CURRENT_SETTINGS_VERSION ||
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
