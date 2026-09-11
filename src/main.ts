import { Notice, Plugin } from "obsidian";
import {
  channel_directory as createChannelDirectory,
  type DiscordChannelSettings,
  type DiscordMessage,
  type DiscordPluginSettings,
  channel_display_name as getChannelDisplayName,
  type MessageSyncSettingsSnapshot,
  prepare_sync,
  type SyncPreparation,
} from "../pkg/parse_message.js";
import {
  getChannelSyncFailureNotice,
  getSyncCompletionNotice,
  processDiscordMessageBatch,
  syncChannelMessages,
  syncChannelsSequentially,
} from "./channelSync";
import { fetchMessages, postNotification } from "./discordApi";
import { migrateSettings, persistChannelCursor } from "./settings";
import { DiscordMessageSenderSettingTab } from "./settingTab";
import {
  getExistingIndividualMessageIds,
  saveProcessedMessages,
} from "./vault";
import { initWasmBridge, parseMessageWasm } from "./wasmBridge";
import { DiscordApiError, getDiscordApiFailureNotice } from "./wasmCore";

export default class DiscordMessageSenderPlugin extends Plugin {
  declare settings: DiscordPluginSettings;
  private syncing = false;

  override async onload() {
    if (!this.manifest.dir) {
      new Notice("Discord message sender: plugin directory not found.");
      return;
    }

    await initWasmBridge();
    await this.loadSettings();
    this.addCommand({
      id: "sync-discord-messages",
      name: "Sync Discord messages",
      callback: () => this.syncDiscordMessages(),
    });
    if (this.settings.enableAutoSyncOnStartup) {
      this.syncDiscordMessages().catch(console.error);
    }
    this.addSettingTab(new DiscordMessageSenderSettingTab(this.app, this));
  }

  private async syncDiscordMessages(): Promise<void> {
    if (this.syncing) {
      new Notice("Discord sync is already running.");
      return;
    }

    let preparation: SyncPreparation;
    try {
      preparation = prepare_sync(
        this.settings,
        Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
      );
    } catch (error) {
      new Notice(error instanceof Error ? error.message : String(error));
      return;
    }
    const channels = preparation.channelIndices
      .map((index) => this.settings.channels[index])
      .filter((channel): channel is DiscordChannelSettings => !!channel);
    const settingsSnapshot = preparation.settings;
    this.syncing = true;
    new Notice("Starting Discord sync.");

    try {
      const summary = await syncChannelsSequentially(channels, (channel) => {
        const snapshot = { ...channel };
        return syncChannelMessages(
          {
            botToken: settingsSnapshot.botToken,
            channel: snapshot,
            sendSyncNotifications: settingsSnapshot.sendSyncNotifications,
            notificationTemplates: settingsSnapshot.notificationTemplates,
          },
          {
            fetchMessages,
            postNotification,
            processMessages: (messages, currentChannel) =>
              this.processDiscordMessages(
                messages,
                currentChannel,
                settingsSnapshot,
              ),
            persistCursor: (_currentChannel, messageId) =>
              this.updateLastProcessedMessage(channel, snapshot.id, messageId),
            sleep,
          },
        );
      });

      for (const failure of summary.failures) {
        console.error(
          `Discord sync failed for ${getChannelDisplayName(failure.channel)}:`,
          failure.error,
        );
        new Notice(getChannelSyncFailureNotice(failure));
      }

      new Notice(getSyncCompletionNotice(summary));
    } catch (error) {
      console.error("Discord sync failed:", error);

      if (error instanceof DiscordApiError) {
        new Notice(
          `Discord sync failed: ${getDiscordApiFailureNotice(error)}.`,
        );
      } else {
        new Notice("Discord sync failed. See console for details.");
      }
    } finally {
      this.syncing = false;
    }
  }

  private async processDiscordMessages(
    messages: readonly DiscordMessage[],
    channel: DiscordChannelSettings,
    settings: MessageSyncSettingsSnapshot,
  ): Promise<number> {
    const messageDirectory = createChannelDirectory(
      settings.messageDirectoryName,
      channel,
    );
    const clippingDirectory = createChannelDirectory(
      settings.clippingDirectoryName,
      channel,
    );
    // Clippings are always individual files, so skip their IDs before parsing
    // to avoid fetching the same external URL again during a retry.
    const existingClippingIds = getExistingIndividualMessageIds(
      this.app.vault,
      clippingDirectory,
    );

    return processDiscordMessageBatch(
      messages,
      (message) =>
        parseMessageWasm(message, settings.messagePrefix, settings.timeZone),
      (processedMessages) =>
        saveProcessedMessages(
          this.app.vault,
          messageDirectory,
          clippingDirectory,
          processedMessages,
          settings,
        ),
      existingClippingIds,
    );
  }

  private async updateLastProcessedMessage(
    channel: DiscordChannelSettings,
    expectedChannelId: string,
    id: string,
  ): Promise<void> {
    await persistChannelCursor(channel, expectedChannelId, id, () =>
      this.saveSettings(),
    );
  }

  async loadSettings(): Promise<void> {
    const migration = migrateSettings(await this.loadData());
    this.settings = migration.settings;

    if (migration.didMigrate) {
      try {
        await this.saveSettings();
      } catch (error) {
        console.warn("Could not persist migrated Discord settings:", error);
      }
    }
  }

  async saveSettings(): Promise<void> {
    await this.saveData(this.settings);
  }
}
