import { Notice, Plugin } from "obsidian";
import { fetchMessages, postNotification } from "./discordApi";
import { DiscordMessageSenderSettingTab } from "./settingTab";
import {
  DEFAULT_SETTINGS,
  type DiscordMessage,
  type DiscordPluginSettings,
  type ProcessedMessage,
} from "./settings";
import { delay } from "./utils";
import { saveToVault } from "./vault";
import { initWasmBridge, parseMessageWasm } from "./wasmBridge";
import "./global";

const MESSAGE_PROCESSING_DELAY = 50; // ms
const REQUEST_INTERVAL_DELAY = 1000; // ms

export default class DiscordMessageSenderPlugin extends Plugin {
  settings!: DiscordPluginSettings;
  private syncing = false;

  override async onload() {
    await initWasmBridge(this.app, this.manifest.dir!);
    await this.loadSettings();
    this.registerCommands();
    this.setupAutoSync();
    this.addSettingTab(new DiscordMessageSenderSettingTab(this.app, this));
  }

  override onunload(): void {}

  private registerCommands(): void {
    this.addCommand({
      id: "sync-discord-messages",
      name: "Sync Discord Messages",
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
    this.syncing = true;

    if (!this.validateSettings()) {
      new Notice("Discord Sync: Bot token or channel ID is not configured.");
      this.syncing = false;
      return;
    }

    let lastMessageId = this.settings.lastProcessedMessageId;
    let processedMessageCount = 0;
    let newestMessageIdProcessed: string | undefined;

    try {
      while (true) {
        const messages = await fetchMessages(this.settings, lastMessageId);
        if (messages.length === 0) {
          break;
        }

        const newestMessageId = messages[0]?.id;

        for (const message of messages.reverse()) {
          const wasProcessed = await this.processDiscordMessage(message);
          if (wasProcessed) {
            processedMessageCount++;
            newestMessageIdProcessed = message.id;
          }
          await delay(MESSAGE_PROCESSING_DELAY);
        }

        lastMessageId = newestMessageId;
        await delay(REQUEST_INTERVAL_DELAY);
      }
      await postNotification(
        this.settings,
        processedMessageCount === 0
          ? "⚠️ No new messages."
          : `✅ ${processedMessageCount} messages saved.`,
      );
    } catch (error) {
      console.error("Discord sync failed:", error);
      new Notice("Discord sync failed. See console for details.");
    } finally {
      if (newestMessageIdProcessed) {
        await this.updateLastProcessedMessage(newestMessageIdProcessed);
      }
      this.syncing = false;
    }
  }

  private async processDiscordMessage(
    message: DiscordMessage,
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
      this.settings.messageDirectoryName,
      this.settings.clippingDirectoryName,
      processedMessage,
    );
    return true;
  }

  private async parseMessage(
    content: string,
    timestamp: string,
  ): Promise<ProcessedMessage> {
    try {
      const result = await parseMessageWasm(
        this.app,
        this.manifest.dir!,
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

  private async updateLastProcessedMessage(id: string) {
    this.settings.lastProcessedMessageId = id;
    try {
      await this.saveSettings();
    } catch (e) {
      console.warn("Could not persist lastProcessedMessageId:", e);
    }
  }

  private validateSettings(): boolean {
    return !!(this.settings.botToken && this.settings.channelId);
  }

  async loadSettings(): Promise<void> {
    this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
  }

  async saveSettings(): Promise<void> {
    await this.saveData(this.settings);
  }
}
