import {
  change_channel_id,
  configured_channel_indices,
  type DiscordChannelSettings,
  type DiscordPluginSettings,
  migrate_settings,
  normalize_settings,
} from "../pkg/parse_message.js";

export function normalizeSettings(data: unknown): DiscordPluginSettings {
  return normalize_settings(data ?? null);
}
export function migrateSettings(data: unknown) {
  return migrate_settings(data ?? null);
}
export function getConfiguredChannels(
  channels: readonly DiscordChannelSettings[],
): DiscordChannelSettings[] {
  // Return the original objects: live sync cursor updates must reach plugin.settings.
  return Array.from(configured_channel_indices([...channels]), (index) => {
    const channel = channels[index];
    if (!channel) throw new Error("WASM returned an invalid channel index.");
    return channel;
  });
}
export function updateChannelId(
  channel: DiscordChannelSettings,
  id: string,
): void {
  const updated = change_channel_id(channel, id);
  delete channel.lastProcessedMessageId;
  Object.assign(channel, updated);
}

export async function persistChannelCursor(
  channel: DiscordChannelSettings,
  expectedChannelId: string,
  messageId: string,
  persistSettings: () => Promise<void>,
): Promise<void> {
  if (channel.id !== expectedChannelId) return;

  const previousMessageId = channel.lastProcessedMessageId;
  channel.lastProcessedMessageId = messageId;

  try {
    await persistSettings();
  } catch (error) {
    if (
      channel.id === expectedChannelId &&
      channel.lastProcessedMessageId === messageId
    ) {
      if (previousMessageId === undefined) {
        delete channel.lastProcessedMessageId;
      } else {
        channel.lastProcessedMessageId = previousMessageId;
      }
    }
    throw error;
  }
}
