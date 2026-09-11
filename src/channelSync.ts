import {
  type DiscordChannelSettings,
  type DiscordMessage,
  type NotificationTemplates,
  type ProcessedMessage,
  select_message_page,
  should_process_message,
  sync_batches,
  sync_completion_notice,
  sync_failure_notice,
  sync_notification_text,
} from "../pkg/parse_message.js";
import type { DiscordMessagePage } from "./discordApi";
import { MessageStorageError } from "./vault";
import { DiscordApiError, getDiscordApiFailureNotice } from "./wasmCore";

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
    const selection = select_message_page(page.messages, lastMessageId);
    if (selection.messages.length > 0) pages.push(selection.messages);
    if (!selection.before) break;
    before = selection.before;
    if (page.nextRequestDelayMs > 0) {
      await dependencies.sleep(page.nextRequestDelayMs);
    }
  }

  for (const batch of sync_batches(pages)) {
    processedMessageCount += await dependencies.processMessages(
      batch.messages,
      channel,
    );
    // Persist only after all storage operations for this page have succeeded.
    await dependencies.persistCursor(channel, batch.cursor);
  }

  if (sendSyncNotifications) {
    await dependencies.postNotification(
      botToken,
      channel.id,
      sync_notification_text(
        notificationTemplates,
        channel,
        processedMessageCount,
      ),
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
  const reason =
    failure.error instanceof DiscordApiError
      ? getDiscordApiFailureNotice(failure.error)
      : failure.error instanceof MessageStorageError
        ? failure.error.message
        : "unexpected error; see console for details";

  return sync_failure_notice(failure.channel, reason);
}

export function getSyncCompletionNotice(summary: ChannelSyncSummary): string {
  return sync_completion_notice(
    summary.processedMessageCount,
    summary.failures.length,
  );
}

export async function processDiscordMessageBatch(
  messages: readonly DiscordMessage[],
  parseMessage: (
    message: DiscordMessage,
  ) => Promise<ProcessedMessage | undefined>,
  saveMessages: (messages: readonly ProcessedMessage[]) => Promise<number>,
): Promise<number> {
  const processedMessages: ProcessedMessage[] = [];

  try {
    for (const message of messages) {
      if (!should_process_message(message)) {
        continue;
      }

      const processedMessage = await parseMessage(message);
      if (processedMessage?.markdown) {
        processedMessages.push(processedMessage);
      }
    }
  } catch (error) {
    if (processedMessages.length > 0) {
      await saveMessages(processedMessages);
    }
    throw error;
  }

  return saveMessages(processedMessages);
}
