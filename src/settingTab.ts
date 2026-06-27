import {
  type App,
  PluginSettingTab,
  Setting,
  type TextComponent,
} from "obsidian";
import type DiscordMessageSenderPlugin from "./main";
import {
  DEFAULT_NOTIFICATION_TEMPLATES,
  type DiscordChannelSettings,
} from "./settings";

export class DiscordMessageSenderSettingTab extends PluginSettingTab {
  plugin: DiscordMessageSenderPlugin;

  constructor(app: App, plugin: DiscordMessageSenderPlugin) {
    super(app, plugin);
    this.plugin = plugin;
  }

  override display(): void {
    const { containerEl } = this;
    containerEl.empty();

    this.createDirectorySettings(containerEl);
    this.createDiscordSettings(containerEl);
    this.createNotificationSettings(containerEl);
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

    this.createChannelSettings(containerEl);

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

  private createChannelSettings(containerEl: HTMLElement): void {
    new Setting(containerEl)
      .setName("Channels")
      .setDesc("Discord channels to sync messages from")
      .addButton((button) =>
        button
          .setButtonText("Add channel")
          .setCta()
          .onClick(async () => {
            this.plugin.settings.channels.push({ id: "", name: "" });
            await this.plugin.saveSettings();
            this.display();
          }),
      );

    if (this.plugin.settings.channels.length === 0) {
      new Setting(containerEl).setName("No channels configured");
      return;
    }

    this.plugin.settings.channels.forEach((channel, index) => {
      this.addChannelSetting(containerEl, channel, index);
    });
  }

  private addChannelSetting(
    containerEl: HTMLElement,
    channel: DiscordChannelSettings,
    index: number,
  ): void {
    new Setting(containerEl)
      .setName(`Channel ${index + 1}`)
      .addText((text) =>
        text
          .setPlaceholder("Name (optional)")
          .setValue(channel.name)
          .onChange(async (value) => {
            channel.name = value.trim();
            await this.plugin.saveSettings();
          }),
      )
      .addText((text) =>
        text
          .setPlaceholder("Channel ID")
          .setValue(channel.id)
          .onChange(async (value) => {
            channel.id = value.trim();
            await this.plugin.saveSettings();
          }),
      )
      .addExtraButton((button) =>
        button
          .setIcon("trash")
          .setTooltip("Remove channel")
          .onClick(async () => {
            this.plugin.settings.channels.splice(index, 1);
            await this.plugin.saveSettings();
            this.display();
          }),
      );
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

  private createNotificationSettings(containerEl: HTMLElement): void {
    new Setting(containerEl).setName("Notifications").setHeading();

    this.addTextAreaSetting(containerEl, {
      name: "Saved messages template",
      description:
        "Discord message sent when one or more messages are saved. Available variables: {count}, {channelName}, {channelId}",
      placeholder: DEFAULT_NOTIFICATION_TEMPLATES.saved,
      getValue: () => this.plugin.settings.notificationTemplates.saved,
      setValue: (value) => {
        this.plugin.settings.notificationTemplates.saved =
          value || DEFAULT_NOTIFICATION_TEMPLATES.saved;
      },
    });

    this.addTextAreaSetting(containerEl, {
      name: "No new messages template",
      description:
        "Discord message sent when there are no new messages. Available variables: {count}, {channelName}, {channelId}",
      placeholder: DEFAULT_NOTIFICATION_TEMPLATES.noNew,
      getValue: () => this.plugin.settings.notificationTemplates.noNew,
      setValue: (value) => {
        this.plugin.settings.notificationTemplates.noNew =
          value || DEFAULT_NOTIFICATION_TEMPLATES.noNew;
      },
    });
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

  private addTextAreaSetting(
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

    setting.addTextArea((text) =>
      text
        .setPlaceholder(options.placeholder)
        .setValue(options.getValue())
        .onChange(async (value) => {
          options.setValue(value.trim());
          await this.plugin.saveSettings();
        }),
    );
  }
}
