import {
  type App,
  PluginSettingTab,
  Setting,
  type TextComponent,
} from "obsidian";
import type DiscordMessageSenderPlugin from "./main";

export class DiscordMessageSenderSettingTab extends PluginSettingTab {
  plugin: DiscordMessageSenderPlugin;

  constructor(app: App, plugin: DiscordMessageSenderPlugin) {
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

  private createDirectorySettings(containerEl: HTMLElement): void {
    new Setting(containerEl).setName("Directory").setHeading();

    this.addTextSetting(containerEl, {
      name: "Messages directory",
      description: "Directory where regular Discord messages will be saved",
      placeholder: "DiscordLogs",
      getValue: () => this.plugin.settings.messageDirectoryName,
      setValue: (value) => {
        this.plugin.settings.messageDirectoryName = value || "DiscordLogs";
      },
    });

    this.addTextSetting(containerEl, {
      name: "Clippings directory",
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
    new Setting(containerEl).setName("Discord").setHeading();

    this.addPasswordSetting(containerEl, {
      name: "Bot token",
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
      name: "Message prefix",
      description: "Prefix for message processing",
      placeholder: "!",
      getValue: () => this.plugin.settings.messagePrefix,
      setValue: (value) => {
        this.plugin.settings.messagePrefix = value || "!";
      },
    });
  }

  private createBehaviorSettings(containerEl: HTMLElement): void {
    new Setting(containerEl).setName("Behavior").setHeading();

    new Setting(containerEl)
      .setName("Auto-sync on startup")
      .setDesc("Automatically sync messages when Obsidian starts")
      .addToggle((toggle) =>
        toggle
          .setValue(this.plugin.settings.enableAutoSyncOnStartup)
          .onChange(async (value) => {
            this.plugin.settings.enableAutoSyncOnStartup = value;
            await this.plugin.saveSettings();
          }),
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
    },
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
        }),
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
    },
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
