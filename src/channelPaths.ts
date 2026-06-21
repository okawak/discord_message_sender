import type { DiscordChannelSettings } from "./settings";

const UNSAFE_PATH_SEGMENT_CHARS = /[\\/:*?"<>|#^]+/g;
const WHITESPACE = /\s+/g;

export function getChannelDisplayName(channel: DiscordChannelSettings): string {
  return channel.name || channel.id;
}

export function getChannelPathSegment(channel: DiscordChannelSettings): string {
  const displayName = getChannelDisplayName(channel);
  const sanitized = displayName
    .trim()
    .replaceAll("[", "-")
    .replaceAll("]", "-")
    .replace(UNSAFE_PATH_SEGMENT_CHARS, "-")
    .replace(WHITESPACE, " ")
    .replace(/^-+|-+$/g, "");

  return sanitized || channel.id;
}

export function createChannelDirectory(
  baseDirectory: string,
  channel: DiscordChannelSettings,
): string {
  const base = baseDirectory.replace(/\/+$/g, "");
  const segment = getChannelPathSegment(channel);

  return base ? `${base}/${segment}` : segment;
}
