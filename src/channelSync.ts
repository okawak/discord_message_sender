import { getChannelDisplayName } from "./channelPaths";
import { DiscordApiError, getDiscordApiFailureNotice } from "./discordApiError";
import type { DiscordChannelSettings } from "./settings";

export interface ChannelSyncFailure {
  channel: DiscordChannelSettings;
  error: unknown;
}

export interface ChannelSyncSummary {
  processedMessageCount: number;
  failures: ChannelSyncFailure[];
}

export async function syncChannelsSequentially(
  channels: readonly DiscordChannelSettings[],
  syncChannel: (channel: DiscordChannelSettings) => Promise<number>,
): Promise<ChannelSyncSummary> {
  let processedMessageCount = 0;
  const failures: ChannelSyncFailure[] = [];

  for (const channel of channels) {
    try {
      processedMessageCount += await syncChannel(channel);
    } catch (error) {
      if (error instanceof DiscordApiError && error.status === 401) {
        throw error;
      }
      failures.push({ channel, error });
    }
  }

  return { processedMessageCount, failures };
}

export function getChannelSyncFailureNotice(
  failure: ChannelSyncFailure,
): string {
  const channelName = getChannelDisplayName(failure.channel);
  const reason =
    failure.error instanceof DiscordApiError
      ? getDiscordApiFailureNotice(failure.error)
      : "unexpected error; see console for details";

  return `Discord sync skipped "${channelName}": ${reason}.`;
}

export function getSyncCompletionNotice(summary: ChannelSyncSummary): string {
  const saved =
    summary.processedMessageCount === 0
      ? "No new messages"
      : `${summary.processedMessageCount} messages saved`;

  if (summary.failures.length === 0) {
    return `Discord sync finished. ${saved}.`;
  }

  const channels =
    summary.failures.length === 1
      ? "1 channel failed"
      : `${summary.failures.length} channels failed`;
  return `Discord sync finished. ${saved}; ${channels}.`;
}
