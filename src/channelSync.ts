import { getChannelDisplayName } from "./channelPaths";
import { DiscordApiError, getDiscordApiFailureNotice } from "./discordApiError";
import type { DiscordMessage } from "./messages";
import { renderNotificationTemplate } from "./notificationTemplates";
import type { DiscordChannelSettings, NotificationTemplates } from "./settings";

const MESSAGE_PROCESSING_DELAY = 50;
const REQUEST_INTERVAL_DELAY = 1000;

export interface SingleChannelSyncOptions {
  botToken: string;
  channel: DiscordChannelSettings;
  notificationTemplates: NotificationTemplates;
}

export interface SingleChannelSyncDependencies {
  fetchMessages: (
    botToken: string,
    channelId: string,
    after?: string,
  ) => Promise<DiscordMessage[]>;
  postNotification: (
    botToken: string,
    channelId: string,
    text: string,
  ) => Promise<DiscordMessage>;
  processMessage: (
    message: DiscordMessage,
    channel: DiscordChannelSettings,
  ) => Promise<boolean>;
  persistCursor: (
    channel: DiscordChannelSettings,
    messageId: string,
  ) => Promise<void>;
  sleep: (milliseconds: number) => Promise<void>;
}

export interface ChannelSyncFailure {
  channel: DiscordChannelSettings;
  error: unknown;
}

export interface ChannelSyncSummary {
  processedMessageCount: number;
  failures: ChannelSyncFailure[];
}

export async function syncChannelMessages(
  options: SingleChannelSyncOptions,
  dependencies: SingleChannelSyncDependencies,
): Promise<number> {
  const { botToken, channel, notificationTemplates } = options;
  let lastMessageId = channel.lastProcessedMessageId;
  let processedMessageCount = 0;

  while (true) {
    const messages = await dependencies.fetchMessages(
      botToken,
      channel.id,
      lastMessageId,
    );
    const newestMessage = messages[0];
    if (!newestMessage) {
      break;
    }

    for (const message of [...messages].reverse()) {
      if (await dependencies.processMessage(message, channel)) {
        processedMessageCount++;
      }
      await dependencies.sleep(MESSAGE_PROCESSING_DELAY);
    }

    lastMessageId = newestMessage.id;
    await dependencies.persistCursor(channel, newestMessage.id);
    await dependencies.sleep(REQUEST_INTERVAL_DELAY);
  }

  const template =
    processedMessageCount === 0
      ? notificationTemplates.noNew
      : notificationTemplates.saved;
  await dependencies.postNotification(
    botToken,
    channel.id,
    renderNotificationTemplate(template, {
      channel,
      count: processedMessageCount,
    }),
  );

  return processedMessageCount;
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
