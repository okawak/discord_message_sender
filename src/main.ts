import { Notice, Plugin } from "obsidian";
import { createChannelDirectory, getChannelDisplayName } from "./channelPaths";
import {
  getChannelSyncFailureNotice,
  getSyncCompletionNotice,
  syncChannelsSequentially,
} from "./channelSync";
import { fetchMessages, postNotification } from "./discordApi";
import { DiscordApiError, getDiscordApiFailureNotice } from "./discordApiError";
import { cleanupGlobalNamespace } from "./global";
import { renderNotificationTemplate } from "./notificationTemplates";
import {
  type DiscordChannelSettings,
  type DiscordMessage,
  type DiscordPluginSettings,
  migrateSettings,
  normalizeSettings,
  type ProcessedMessage,
} from "./settings";
import { DiscordMessageSenderSettingTab } from "./settingTab";
import { saveToVault } from "./vault";
import { initWasmBridge, parseMessageWasm } from "./wasmBridge";

const MESSAGE_PROCESSING_DELAY = 50; // ms
const REQUEST_INTERVAL_DELAY = 1000; // ms

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
    this.registerCommands();
    this.setupAutoSync();
    this.addSettingTab(new DiscordMessageSenderSettingTab(this.app, this));
  }

  override onunload(): void {
    cleanupGlobalNamespace();
  }

  private registerCommands(): void {
    this.addCommand({
      id: "sync-discord-messages",
      name: "Sync Discord messages",
      callback: () => this.syncDiscordMessages(),
    });
  }

  private setupAutoSync(): void {
    if (this.settings.enableAutoSyncOnStartup) {
      this.syncDiscordMessages().catch(console.error);
    }
  }

  private async syncDiscordMessages(): Promise<void> {
    if (this.syncing) {
      new Notice("Discord sync is already running.");
      return;
    }

    if (!this.validateSettings()) {
      new Notice(
        "Discord message sender: bot token or channel is not configured.",
      );
      return;
    }

    this.syncing = true;
    new Notice("Starting Discord sync.");

    try {
      const summary = await syncChannelsSequentially(
        this.configuredChannels(),
        (channel) => this.syncChannelMessages(channel),
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

  private async syncChannelMessages(
    channel: DiscordChannelSettings,
  ): Promise<number> {
    let lastMessageId = channel.lastProcessedMessageId;
    let processedMessageCount = 0;

    while (true) {
      const messages = await fetchMessages(
        this.settings.botToken,
        channel.id,
        lastMessageId,
      );
      if (messages.length === 0) {
        break;
      }

      const newestMessageId = messages[0]?.id;

      for (const message of messages.reverse()) {
        const wasProcessed = await this.processDiscordMessage(message, channel);
        if (wasProcessed) {
          processedMessageCount++;
        }
        await sleep(MESSAGE_PROCESSING_DELAY);
      }

      lastMessageId = newestMessageId;
      if (newestMessageId) {
        await this.updateLastProcessedMessage(channel, newestMessageId);
      }
      await sleep(REQUEST_INTERVAL_DELAY);
    }

    const notificationText = renderNotificationTemplate(
      processedMessageCount === 0
        ? this.settings.notificationTemplates.noNew
        : this.settings.notificationTemplates.saved,
      { channel, count: processedMessageCount },
    );
    const notification = await postNotification(
      this.settings.botToken,
      channel.id,
      notificationText,
    );

    if (notification.id) {
      await this.updateLastProcessedMessage(channel, notification.id);
    }

    return processedMessageCount;
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
      processedMessage = await this.parseMessage(
        message.content,
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

  private async parseMessage(
    content: string,
    timestamp: string,
  ): Promise<ProcessedMessage> {
    if (!this.manifest.dir) {
      throw new Error("Plugin directory not found.");
    }

    try {
      const result = await parseMessageWasm(
        content,
        this.settings.messagePrefix,
        timestamp,
      );
      const { md, is_clip, name } = result;
      return {
        markdown: md,
        isClipping: is_clip,
        fileName: name,
      };
    } catch (error) {
      throw new Error(String(error));
    }
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

  private validateSettings(): boolean {
    return !!(this.settings.botToken && this.configuredChannels().length > 0);
  }

  private configuredChannels(): DiscordChannelSettings[] {
    return this.settings.channels.filter((channel) => channel.id);
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
