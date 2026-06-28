import type { DiscordChannelSettings } from "./settings";

const UNSAFE_PATH_SEGMENT_CHARS = /[\\/:*?"<>|#^]+/g;
const FORBIDDEN_CHANNEL_NAME_CHARS = /[\\/:*?"<>|#^[\]]/;
const WHITESPACE = /\s+/g;

export const INVALID_CHANNEL_NAME_MESSAGE =
  'Channel name is invalid. Forbidden characters are: \\ / : * ? " < > | # ^ [ ]. The names "." and ".." are also not allowed.';

export function getChannelDisplayName(channel: DiscordChannelSettings): string {
  return channel.name || channel.id;
}

export function getChannelNameValidationError(
  name: string,
): string | undefined {
  return FORBIDDEN_CHANNEL_NAME_CHARS.test(name) ||
    name === "." ||
    name === ".."
    ? INVALID_CHANNEL_NAME_MESSAGE
    : undefined;
}

export function getChannelPathSegment(channel: DiscordChannelSettings): string {
  const id = sanitizePathSegment(channel.id) || "channel";
  const name = sanitizePathSegment(channel.name);
  return name && name !== "." && name !== ".." ? name : id;
}

export function findDuplicateChannelPathSegment(
  channels: readonly DiscordChannelSettings[],
): string | undefined {
  const paths = new Set<string>();
  for (const channel of channels) {
    const path = getChannelPathSegment(channel);
    const key = path.normalize("NFC").toLowerCase();
    if (paths.has(key)) {
      return path;
    }
    paths.add(key);
  }
  return undefined;
}

function sanitizePathSegment(value: string): string {
  return value
    .trim()
    .replaceAll("[", "-")
    .replaceAll("]", "-")
    .replace(UNSAFE_PATH_SEGMENT_CHARS, "-")
    .replace(WHITESPACE, " ")
    .replace(/^-+|-+$/g, "");
}

export function createChannelDirectory(
  baseDirectory: string,
  channel: DiscordChannelSettings,
): string {
  const base = baseDirectory.replace(/\/+$/g, "");
  const segment = getChannelPathSegment(channel);

  return base ? `${base}/${segment}` : segment;
}
