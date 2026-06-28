import { Notice, Plugin } from "obsidian";
import { createChannelDirectory, getChannelDisplayName } from "./channelPaths";
import {
  getChannelSyncFailureNotice,
  getSyncCompletionNotice,
  syncChannelMessages,
  syncChannelsSequentially,
} from "./channelSync";
import { fetchMessages, postNotification } from "./discordApi";
import { DiscordApiError, getDiscordApiFailureNotice } from "./discordApiError";
import { cleanupGlobalNamespace } from "./global";
import type { DiscordMessage, ProcessedMessage } from "./messages";
import {
  type DiscordChannelSettings,
  type DiscordPluginSettings,
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

    const channels = this.settings.channels.filter((channel) => channel.id);
    if (!this.settings.botToken || channels.length === 0) {
      new Notice(
        "Discord message sender: bot token or channel is not configured.",
      );
      return;
    }

    this.syncing = true;
    new Notice("Starting Discord sync.");

    try {
      const summary = await syncChannelsSequentially(channels, (channel) =>
        syncChannelMessages(
          {
            botToken: this.settings.botToken,
            channel,
            notificationTemplates: this.settings.notificationTemplates,
          },
          {
            fetchMessages,
            postNotification,
            processMessage: (message, currentChannel) =>
              this.processDiscordMessage(message, currentChannel),
            persistCursor: (currentChannel, messageId) =>
              this.updateLastProcessedMessage(currentChannel, messageId),
            sleep,
          },
        ),
      );

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

    let processedMessage: ProcessedMessage;
    try {
      processedMessage = await parseMessageWasm(
        message.content,
        this.settings.messagePrefix,
        message.timestamp,
      );
    } catch (error) {
      console.error("parseMessage error:", error);
      return false;
    }

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
    id: string,
  ): Promise<void> {
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
