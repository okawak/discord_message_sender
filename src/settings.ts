// Settings for the Discord plugin
export interface DiscordPluginSettings {
  messageDirectoryName: string;
  clippingDirectoryName: string;
  botToken: string;
  channelId: string;
  messagePrefix: string;
  enableAutoSyncOnStartup: boolean;
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

// Default settings for the Discord plugin
export const DEFAULT_SETTINGS: DiscordPluginSettings = {
  messageDirectoryName: "DiscordLogs",
  clippingDirectoryName: "DiscordClippings",
  botToken: "",
  channelId: "",
  messagePrefix: "!",
  enableAutoSyncOnStartup: true,
};
