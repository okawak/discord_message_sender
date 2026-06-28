import { Notice, Plugin } from "obsidian";
import {
  createChannelDirectory,
  findDuplicateChannelPathSegment,
  getChannelDisplayName,
  getChannelNameValidationError,
} from "./channelPaths";
import {
  getChannelSyncFailureNotice,
  getSyncCompletionNotice,
  syncChannelMessages,
  syncChannelsSequentially,
} from "./channelSync";
import { fetchMessages, postNotification } from "./discordApi";
import { DiscordApiError, getDiscordApiFailureNotice } from "./discordApiError";
import { cleanupGlobalNamespace } from "./global";
import type { DiscordMessage } from "./messages";
import {
  type DiscordChannelSettings,
  type DiscordPluginSettings,
  getConfiguredChannels,
  migrateSettings,
  normalizeSettings,
} from "./settings";
import { DiscordMessageSenderSettingTab } from "./settingTab";
import { saveToVault } from "./vault";
import { initWasmBridge, parseMessageWasm } from "./wasmBridge";

export default class DiscordMessageSenderPlugin extends Plugin {
  override settings: DiscordPluginSettings = normalizeSettings(undefined);
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

  override onunload(): void {
    cleanupGlobalNamespace();
  }

  private async syncDiscordMessages(): Promise<void> {
    if (this.syncing) {
      new Notice("Discord sync is already running.");
      return;
    }

    const channels = getConfiguredChannels(this.settings.channels);
    if (!this.settings.botToken || channels.length === 0) {
      new Notice(
        "Discord message sender: bot token or channel is not configured.",
      );
      return;
    }

    for (const channel of channels) {
      const error = getChannelNameValidationError(channel.name);
      if (error) {
        new Notice(error);
        return;
      }
    }

    const duplicatePath = findDuplicateChannelPathSegment(channels);
    if (duplicatePath) {
      new Notice(
        `Discord message sender: duplicate channel folder "${duplicatePath}". Use unique channel names.`,
      );
      return;
    }

    this.syncing = true;
    new Notice("Starting Discord sync.");

    try {
      const summary = await syncChannelsSequentially(channels, (channel) => {
        const snapshot = { ...channel };
        return syncChannelMessages(
          {
            botToken: this.settings.botToken,
            channel: snapshot,
            notificationTemplates: this.settings.notificationTemplates,
          },
          {
            fetchMessages,
            postNotification,
            processMessage: (message, currentChannel) =>
              this.processDiscordMessage(message, currentChannel),
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

  private async processDiscordMessage(
    message: DiscordMessage,
    channel: DiscordChannelSettings,
  ): Promise<boolean> {
    if (message.author?.bot) {
      return false;
    }

    const processedMessage = await parseMessageWasm(
      message,
      this.settings.messagePrefix,
    );

    if (!processedMessage.markdown) {
      return false;
    }

    await saveToVault(
      this.app.vault,
      createChannelDirectory(this.settings.messageDirectoryName, channel),
      createChannelDirectory(this.settings.clippingDirectoryName, channel),
      processedMessage,
    );
    return true;
  }

  private async updateLastProcessedMessage(
    channel: DiscordChannelSettings,
    expectedChannelId: string,
    id: string,
  ): Promise<void> {
    if (channel.id !== expectedChannelId) {
      return;
    }
    channel.lastProcessedMessageId = id;
    try {
      await this.saveSettings();
    } catch (e) {
      console.warn(
        `Could not persist lastProcessedMessageId for ${getChannelDisplayName(
          channel,
        )}:`,
        e,
      );
    }
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
