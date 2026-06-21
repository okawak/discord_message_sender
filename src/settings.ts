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

export interface DiscordPluginSettings {
  messageDirectoryName: string;
  clippingDirectoryName: string;
  botToken: string;
  channels: DiscordChannelSettings[];
  /** @deprecated Use channels instead. Kept during the v0.3 migration. */
  channelId: string;
  messagePrefix: string;
  enableAutoSyncOnStartup: boolean;
  notificationTemplates: NotificationTemplates;
  /** @deprecated Use channels[].lastProcessedMessageId instead. */
  lastProcessedMessageId?: string;
}

// Messages from Discord API
export interface DiscordMessage {
  id: string;
  content: string;
  timestamp: string;
  author?: { bot?: boolean };
}

// Interface for WASM processing
export interface ProcessedMessage {
  markdown: string;
  isClipping: boolean;
  fileName: string;
}

export const DEFAULT_NOTIFICATION_TEMPLATES: NotificationTemplates = {
  saved: "✅ {count} messages saved.",
  noNew: "⚠️ No new messages.",
};

// Default settings for the Discord plugin
export const DEFAULT_SETTINGS: DiscordPluginSettings = {
  messageDirectoryName: "DiscordLogs",
  clippingDirectoryName: "DiscordClippings",
  botToken: "",
  channels: [],
  channelId: "",
  messagePrefix: "!",
  enableAutoSyncOnStartup: true,
  notificationTemplates: DEFAULT_NOTIFICATION_TEMPLATES,
};

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
    messageDirectoryName:
      readString(raw, "messageDirectoryName") ||
      DEFAULT_SETTINGS.messageDirectoryName,
    clippingDirectoryName:
      readString(raw, "clippingDirectoryName") ||
      DEFAULT_SETTINGS.clippingDirectoryName,
    botToken: readString(raw, "botToken"),
    channels,
    channelId: legacyChannelId || channels[0]?.id || "",
    messagePrefix:
      readString(raw, "messagePrefix") || DEFAULT_SETTINGS.messagePrefix,
    enableAutoSyncOnStartup: readBoolean(
      raw,
      "enableAutoSyncOnStartup",
      DEFAULT_SETTINGS.enableAutoSyncOnStartup,
    ),
    notificationTemplates: normalizeNotificationTemplates(raw),
    ...(legacyLastProcessedMessageId
      ? { lastProcessedMessageId: legacyLastProcessedMessageId }
      : {}),
  };
}

function normalizeChannels(
  raw: Record<string, unknown>,
  legacy: { channelId: string; lastProcessedMessageId: string },
): DiscordChannelSettings[] {
  const channels = raw.channels;
  if (Array.isArray(channels)) {
    return channels
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
