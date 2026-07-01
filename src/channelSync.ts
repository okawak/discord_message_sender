import { getChannelDisplayName } from "./channelPaths";
import type { DiscordMessagePage } from "./discordApi";
import { DiscordApiError, getDiscordApiFailureNotice } from "./discordApiError";
import { DISCORD_MESSAGE_PAGE_SIZE } from "./discordRoutes";
import type { DiscordMessage } from "./messages";
import { renderNotificationTemplate } from "./notificationTemplates";
import type { DiscordChannelSettings, NotificationTemplates } from "./settings";

export interface SingleChannelSyncOptions {
  botToken: string;
  channel: DiscordChannelSettings;
  sendSyncNotifications: boolean;
  notificationTemplates: NotificationTemplates;
}

export interface SingleChannelSyncDependencies {
  fetchMessages: (
    botToken: string,
    channelId: string,
    before?: string,
  ) => Promise<DiscordMessagePage>;
  postNotification: (
    botToken: string,
    channelId: string,
    text: string,
  ) => Promise<DiscordMessage>;
  processMessages: (
    messages: readonly DiscordMessage[],
    channel: DiscordChannelSettings,
  ) => Promise<number>;
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
  const { botToken, channel, sendSyncNotifications, notificationTemplates } =
    options;
  const lastMessageId = channel.lastProcessedMessageId;
  let processedMessageCount = 0;
  const pages: DiscordMessage[][] = [];
  let before: string | undefined;

  while (true) {
    const page = await dependencies.fetchMessages(botToken, channel.id, before);
    const messages = lastMessageId
      ? page.messages.filter(
          (message) => BigInt(message.id) > BigInt(lastMessageId),
        )
      : page.messages;
    if (messages.length > 0) {
      pages.push(messages);
    }

    if (
      !lastMessageId ||
      page.messages.length < DISCORD_MESSAGE_PAGE_SIZE ||
      messages.length < page.messages.length
    ) {
      break;
    }

    const oldestMessage = page.messages.at(-1);
    if (!oldestMessage) {
      break;
    }
    before = oldestMessage.id;
    if (page.nextRequestDelayMs > 0) {
      await dependencies.sleep(page.nextRequestDelayMs);
    }
  }

  for (const messages of pages.reverse()) {
    processedMessageCount += await dependencies.processMessages(
      [...messages].reverse(),
      channel,
    );

    const newestMessage = messages[0];
    if (newestMessage) {
      await dependencies.persistCursor(channel, newestMessage.id);
    }
  }

  if (sendSyncNotifications) {
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
  }

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
