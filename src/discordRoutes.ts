export const DISCORD_API_VERSION = 10;
export const DISCORD_MESSAGE_PAGE_SIZE = 100;

export function getChannelMessagesPath(
  channelId: string,
  before?: string,
): string {
  const beforeQuery = before ? `&before=${encodeURIComponent(before)}` : "";
  return `/channels/${encodeURIComponent(channelId)}/messages?limit=${DISCORD_MESSAGE_PAGE_SIZE}${beforeQuery}`;
}
