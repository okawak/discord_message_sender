import {
  type App,
  type DataAdapter,
  type TextComponent,
  type Vault,
  Notice,
  Plugin,
  PluginSettingTab,
  Setting,
  requestUrl,
} from "obsidian";
import { process_message, default as initWasm } from "../pkg/parse_message.js";

// =============================================
// Global type extension for WASM integration
// =============================================
declare global {
  var fetchUrlContent: (url: string) => Promise<string>;
}

globalThis.fetchUrlContent = async function (url: string): Promise<string> {
  try {
    const response = await requestUrl({
      url: url,
      method: "GET",
      headers: {
        "User-Agent": "Obsidian Discord Sender Plugin",
      },
    });
    return response.text;
  } catch (error) {
    console.error("Failed to fetch URL content:", error);
    return `<!-- Failed to fetch content from ${url}: ${error} -->`;
  }
};

// =============================================
// Types & Interfaces
// =============================================
interface DiscordPluginSettings {
  messageDirectoryName: string;
  clippingDirectoryName: string;
  botToken: string;
  channelId: string;
  enableAutoSyncOnStartup: boolean;
  messagePrefix: string;
  lastProcessedMessageId?: string;
}

interface ProcessedMessage {
  markdown: string;
  isClipping: boolean;
  fileName: string;
}

interface DiscordMessage {
  id: string;
  content: string;
  timestamp: string;
  author?: {
    bot?: boolean;
  };
}

// =============================================
// Constants
// =============================================
const DEFAULT_SETTINGS: DiscordPluginSettings = {
  messageDirectoryName: "DiscordLogs",
  clippingDirectoryName: "DiscordClippings",
  botToken: "",
  channelId: "",
  enableAutoSyncOnStartup: true,
  messagePrefix: "!",
};

const DISCORD_API_BASE_URL = "https://discord.com/api/v10";
const WASM_FILE_NAME = "parse_message_bg.wasm";
const MESSAGES_PER_REQUEST = 100;
const MAX_RETRIES = 3;
const RATE_LIMIT_STATUS_CODE = 429;
const MESSAGE_PROCESSING_DELAY = 50; // ms
const REQUEST_INTERVAL_DELAY = 1000; // ms

// =============================================
// Main Plugin Class
// =============================================
export default class DiscordMessageSyncPlugin extends Plugin {
  settings!: DiscordPluginSettings;

  override async onload() {
    await this.initializeWasm();
    await this.loadSettings();
    this.registerCommands();
    this.setupAutoSync();
    this.addSettingTab(new DiscordPluginSettingTab(this.app, this));
  }

  override onunload(): void {}

  // =============================================
  // Initialization Methods
  // =============================================
  private async initializeWasm(): Promise<void> {
    const wasmPath = `${this.manifest.dir}/${WASM_FILE_NAME}`;

    let bytes: Uint8Array;

    const adapter = this.app.vault.adapter;
    if (this.isDataAdapter(adapter)) {
      const buf = await adapter.readBinary(wasmPath);
      bytes = new Uint8Array(buf);
    } else {
      const res = await fetch(wasmPath);
      bytes = new Uint8Array(await res.arrayBuffer());
    }

    await initWasm(bytes);
  }

  private isDataAdapter(a: unknown): a is DataAdapter {
    return !!a && typeof (a as DataAdapter).readBinary === "function";
  }

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

  // =============================================
  // Discord API Integration
  // =============================================
  private async syncDiscordMessages(): Promise<void> {
    if (!this.validateSettings()) {
      new Notice("Discord Sync: Bot token or channel ID is not configured.");
      return;
    }

    let lastMessageId = this.settings.lastProcessedMessageId;
    let processedMessageCount = 0;

    try {
      while (true) {
        const messages = await this.fetchDiscordMessages(lastMessageId);
        if (messages.length === 0) {
          break;
        }

        const newestMessageId = messages[0]?.id;

        for (const message of messages.reverse()) {
          const wasProcessed = await this.processDiscordMessage(message);
          if (wasProcessed) {
            processedMessageCount++;
          }
          await this.delay(MESSAGE_PROCESSING_DELAY);
        }

        lastMessageId = newestMessageId;
        await this.delay(REQUEST_INTERVAL_DELAY);
      }
      await this.notifyCompletion(processedMessageCount);
    } catch (error) {
      console.error("Error during Discord message sync:", error);
      new Notice("Failed to sync Discord messages. Check console for details.");
    }
  }

  private async fetchDiscordMessages(
    afterMessageId?: string
  ): Promise<DiscordMessage[]> {
    const url = this.buildMessagesApiUrl(afterMessageId);

    let retryCount = 0;
    while (retryCount < MAX_RETRIES) {
      try {
        const response = await this.makeDiscordApiRequest(url, "GET");

        if (response.status === RATE_LIMIT_STATUS_CODE) {
          await this.handleRateLimit(response, retryCount);
          retryCount++;
          continue;
        }

        if (response.status !== 200) {
          console.error("Discord API error:", response.status, response.text);
          new Notice("Discord API error. Check console for details.");
          return [];
        }

        return JSON.parse(response.text);
      } catch (error) {
        console.error("Request failed:", error);

        if (retryCount >= MAX_RETRIES) {
          new Notice("Failed to fetch messages after multiple retries.");
          return [];
        }
        retryCount++;
        await this.delay(1000 * retryCount);
      }
    }
    new Notice("Maximum retries exceeded. Please try again later.");
    return [];
  }

  private buildMessagesApiUrl(afterMessageId?: string): string {
    const baseUrl = `${DISCORD_API_BASE_URL}/channels/${this.settings.channelId}/messages`;
    const params = `?limit=${MESSAGES_PER_REQUEST}${
      afterMessageId ? `&after=${afterMessageId}` : ""
    }`;
    return baseUrl + params;
  }

  private async makeDiscordApiRequest(
    url: string,
    method: string,
    body?: string
  ) {
    return await requestUrl({
      url,
      method,
      headers: {
        Authorization: `Bot ${this.settings.botToken}`,
        "User-Agent": "DiscordBot (Obsidian Discord Message Sync)",
        ...(body && { "Content-Type": "application/json" }),
      },
      ...(body && { body }),
    });
  }

  private async handleRateLimit(
    response: any,
    retryCount: number
  ): Promise<void> {
    const retryAfterHeader = response.headers["Retry-After"];
    const waitTime = retryAfterHeader
      ? Number.parseInt(retryAfterHeader) * 1000
      : 1000 * 2 ** retryCount;

    console.warn(`Rate limited. Waiting ${waitTime}ms before retry...`);
    new Notice(`Rate limited. Waiting ${Math.ceil(waitTime / 1000)}s...`);

    await this.delay(waitTime);
  }

  // =============================================
  // Message Processing
  // =============================================
  private async processDiscordMessage(
    message: DiscordMessage
  ): Promise<boolean> {
    if (message.author?.bot) {
      return false;
    }

    let processedMessage: ProcessedMessage;
    try {
      processedMessage = await this.parseMessage(
        message.content,
        message.timestamp
      );
    } catch (error) {
      console.error("parseMessage error:", error);
      return false;
    }

    if (!processedMessage.markdown) {
      return false;
    }

    await this.saveMessageToVault(message, processedMessage);
    await this.updateLastProcessedMessage(message.id);

    return true;
  }

  private async parseMessage(
    content: string,
    timestamp: string
  ): Promise<ProcessedMessage> {
    try {
      const result = await process_message(
        content,
        this.settings.messagePrefix,
        timestamp
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

  private async saveMessageToVault(
    message: DiscordMessage,
    processedMessage: ProcessedMessage
  ): Promise<void> {
    const directoryName = processedMessage.isClipping
      ? this.settings.clippingDirectoryName
      : this.settings.messageDirectoryName;

    await this.ensureDirectoryExists(this.app.vault, directoryName);

    const fileName = processedMessage.fileName || message.id;
    const filePath = `${directoryName}/${fileName}.md`;

    if (!this.app.vault.getAbstractFileByPath(filePath)) {
      await this.app.vault.create(filePath, processedMessage.markdown);
    }
  }

  private async updateLastProcessedMessage(messageId: string): Promise<void> {
    this.settings.lastProcessedMessageId = messageId;
    await this.saveSettings();
  }

  // =============================================
  // Discord Notifications
  // =============================================
  private async notifyCompletion(processedCount: number): Promise<void> {
    const message =
      processedCount === 0
        ? "⚠️ No new messages to save."
        : `✅ ${processedCount} new messages saved.`;

    await this.sendDiscordNotification(message);
  }

  private async sendDiscordNotification(text: string): Promise<void> {
    const url = `${DISCORD_API_BASE_URL}/channels/${this.settings.channelId}/messages`;
    const body = JSON.stringify({ content: text });

    let retryCount = 0;
    while (retryCount <= MAX_RETRIES) {
      try {
        const response = await this.makeDiscordApiRequest(url, "POST", body);

        if (response.status === RATE_LIMIT_STATUS_CODE) {
          await this.handleRateLimit(response, retryCount);
          retryCount++;
          continue;
        }

        if (response.status >= 200 && response.status < 300) {
          return;
        }

        console.error(
          "Failed to send Discord notification:",
          response.status,
          response.text
        );
        break;
      } catch (error) {
        console.error("Error sending Discord notification:", error);

        if (retryCount >= MAX_RETRIES) {
          break;
        }

        retryCount++;
        await this.delay(1000 * retryCount);
      }
    }
  }

  // =============================================
  // Utility Methods
  // =============================================
  private validateSettings(): boolean {
    return !!(this.settings.botToken && this.settings.channelId);
  }

  private async ensureDirectoryExists(
    vault: Vault,
    directoryPath: string
  ): Promise<void> {
    if (vault.getAbstractFileByPath(directoryPath)) {
      return;
    }

    try {
      await vault.createFolder(directoryPath);
    } catch (error: any) {
      if (error.message?.includes("no such file or directory")) {
        const parentPath = directoryPath.split("/").slice(0, -1).join("/");
        if (parentPath) {
          await this.ensureDirectoryExists(vault, parentPath);
        }
        await vault.createFolder(directoryPath);
      } else {
        throw error;
      }
    }
  }

  private delay(milliseconds: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
  }

  // =============================================
  // Settings Management
  // =============================================
  async loadSettings(): Promise<void> {
    this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
  }

  async saveSettings(): Promise<void> {
    await this.saveData(this.settings);
  }
}

// =============================================
// Setting Tab
// =============================================
class DiscordPluginSettingTab extends PluginSettingTab {
  plugin: DiscordMessageSyncPlugin;

  constructor(app: App, plugin: DiscordMessageSyncPlugin) {
    super(app, plugin);
    this.plugin = plugin;
  }

  display(): void {
    const { containerEl } = this;
    containerEl.empty();

    this.createDirectorySettings(containerEl);
    this.createDiscordSettings(containerEl);
    this.createBehaviorSettings(containerEl);
  }

  // =============================================
  // Grouped Settings Creation Methods
  // =============================================
  private createDirectorySettings(containerEl: HTMLElement): void {
    containerEl.createEl("h2", { text: "Directory" });

    this.addTextSetting(containerEl, {
      name: "Messages Directory",
      description: "Directory where regular Discord messages will be saved",
      placeholder: "DiscordLogs",
      getValue: () => this.plugin.settings.messageDirectoryName,
      setValue: (value) => {
        this.plugin.settings.messageDirectoryName = value || "DiscordLogs";
      },
    });

    this.addTextSetting(containerEl, {
      name: "Clippings Directory",
      description: "Directory where URL clippings will be saved",
      placeholder: "DiscordClippings",
      getValue: () => this.plugin.settings.clippingDirectoryName,
      setValue: (value) => {
        this.plugin.settings.clippingDirectoryName =
          value || "DiscordClippings";
      },
    });
  }

  private createDiscordSettings(containerEl: HTMLElement): void {
    containerEl.createEl("h2", { text: "Discord" });

    this.addPasswordSetting(containerEl, {
      name: "Bot Token",
      description: "Your Discord bot token",
      placeholder: "xxxxx.yyyyy.zzzzz",
      getValue: () => this.plugin.settings.botToken,
      setValue: (value) => {
        this.plugin.settings.botToken = value;
      },
    });

    this.addPasswordSetting(containerEl, {
      name: "Channel ID",
      description: "Discord channel ID to sync messages from",
      placeholder: "123456789012345678",
      getValue: () => this.plugin.settings.channelId,
      setValue: (value) => {
        this.plugin.settings.channelId = value;
      },
    });

    this.addTextSetting(containerEl, {
      name: "Message Prefix",
      description: "Prefix for message processing",
      placeholder: "!",
      getValue: () => this.plugin.settings.messagePrefix,
      setValue: (value) => {
        this.plugin.settings.messagePrefix = value || "!";
      },
    });
  }

  private createBehaviorSettings(containerEl: HTMLElement): void {
    containerEl.createEl("h2", { text: "Others" });

    new Setting(containerEl)
      .setName("Auto-sync on startup")
      .setDesc("Automatically sync messages when Obsidian starts")
      .addToggle((toggle) =>
        toggle
          .setValue(this.plugin.settings.enableAutoSyncOnStartup)
          .onChange(async (value) => {
            this.plugin.settings.enableAutoSyncOnStartup = value;
            await this.plugin.saveSettings();
          })
      );
  }

  // =============================================
  // Utility Methods for Settings
  // =============================================
  private addTextSetting(
    containerEl: HTMLElement,
    options: {
      name: string;
      description?: string;
      placeholder: string;
      getValue: () => string;
      setValue: (value: string) => void;
    }
  ): void {
    const setting = new Setting(containerEl).setName(options.name);

    if (options.description) {
      setting.setDesc(options.description);
    }

    setting.addText((text) =>
      text
        .setPlaceholder(options.placeholder)
        .setValue(options.getValue())
        .onChange(async (value) => {
          options.setValue(value.trim());
          await this.plugin.saveSettings();
        })
    );
  }

  private addPasswordSetting(
    containerEl: HTMLElement,
    options: {
      name: string;
      description?: string;
      placeholder: string;
      getValue: () => string;
      setValue: (value: string) => void;
    }
  ): void {
    const setting = new Setting(containerEl).setName(options.name);

    if (options.description) {
      setting.setDesc(options.description);
    }

    let textComponent!: TextComponent;
    // Toggle password visibility
    setting.addExtraButton((button) => {
      let isVisible = false;
      const toggleVisibility = () => {
        isVisible = !isVisible;
        textComponent.inputEl.type = isVisible ? "text" : "password";
        button.setIcon(isVisible ? "eye" : "eye-off");
      };

      button
        .setIcon("eye-off")
        .setTooltip("Toggle password visibility")
        .onClick(toggleVisibility);
    });

    setting.addText((text) => {
      textComponent = text;
      text.inputEl.type = "password";
      text
        .setPlaceholder(options.placeholder)
        .setValue(options.getValue())
        .onChange(async (value) => {
          options.setValue(value.trim());
          await this.plugin.saveSettings();
        });
    });
  }
}
