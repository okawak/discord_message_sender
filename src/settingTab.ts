import {
  type App,
  Notice,
  PluginSettingTab,
  type Setting,
  type SettingDefinition,
  type SettingDefinitionItem,
  type TextComponent,
} from "obsidian";
import {
  type DiscordChannelSettings,
  default_settings as getDefaultSettings,
  type MessageStorageMode,
  normalize_setting_control,
  read_setting_control,
  rename_channel,
  type SettingControlValue,
  trim_setting,
} from "../pkg/parse_message.js";
import type DiscordMessageSenderPlugin from "./main";
import { updateChannelId } from "./settings";

type SettingKey =
  | "messageDirectoryName"
  | "clippingDirectoryName"
  | "messagePrefix"
  | "messageStorageMode"
  | "showAuthorNames"
  | "showMessageTime"
  | "enableAutoSyncOnStartup"
  | "sendSyncNotifications"
  | "savedNotificationTemplate"
  | "noNewNotificationTemplate";

const STORAGE_OPTIONS: Record<MessageStorageMode, string> = {
  individual: "One file per message",
  daily: "Daily log",
  weekly: "Weekly log",
  monthly: "Monthly log",
};

export class DiscordMessageSenderSettingTab extends PluginSettingTab {
  plugin: DiscordMessageSenderPlugin;

  constructor(app: App, plugin: DiscordMessageSenderPlugin) {
    super(app, plugin);
    this.plugin = plugin;
  }

  override getSettingDefinitions(): SettingDefinitionItem<SettingKey>[] {
    const defaults = getDefaultSettings();
    return [
      {
        type: "group",
        heading: "Directory",
        items: [
          {
            name: "Messages directory",
            desc: "Directory where regular Discord messages will be saved",
            control: {
              type: "text",
              key: "messageDirectoryName",
              defaultValue: defaults.messageDirectoryName,
              placeholder: "DiscordLogs",
            },
          },
          {
            name: "Clippings directory",
            desc: "Directory where URL clippings will be saved",
            control: {
              type: "text",
              key: "clippingDirectoryName",
              defaultValue: defaults.clippingDirectoryName,
              placeholder: "DiscordClippings",
            },
          },
        ],
      },
      {
        type: "group",
        heading: "Discord",
        items: [
          {
            name: "Bot token",
            desc: "Your Discord bot token",
            render: (setting) => this.renderBotToken(setting),
          },
          {
            name: "Message prefix",
            desc: "Prefix for message processing",
            control: {
              type: "text",
              key: "messagePrefix",
              defaultValue: defaults.messagePrefix,
              placeholder: "!",
            },
          },
        ],
      },
      {
        type: "list",
        heading: "Channels",
        emptyState: "No channels configured",
        addItem: {
          name: "Add channel",
          action: () => {
            this.plugin.settings.channels.push({ id: "", name: "" });
            this.saveChannelChanges();
          },
        },
        onDelete: (index) => {
          this.plugin.settings.channels.splice(index, 1);
          this.saveChannelChanges();
        },
        items: this.plugin.settings.channels.map((channel, index) =>
          this.getChannelDefinition(channel, index),
        ),
      },
      {
        type: "group",
        heading: "Notifications",
        items: [
          {
            name: "Send sync notifications",
            desc: "Send a Discord message after each channel is synced",
            control: { type: "toggle", key: "sendSyncNotifications" },
          },
          {
            name: "Saved messages template",
            desc: "Discord message sent when one or more messages are saved. Available variables: {count}, {channelName}, {channelId}",
            control: {
              type: "textarea",
              key: "savedNotificationTemplate",
              defaultValue: defaults.notificationTemplates.saved,
              placeholder: defaults.notificationTemplates.saved,
            },
          },
          {
            name: "No new messages template",
            desc: "Discord message sent when there are no new messages. Available variables: {count}, {channelName}, {channelId}",
            control: {
              type: "textarea",
              key: "noNewNotificationTemplate",
              defaultValue: defaults.notificationTemplates.noNew,
              placeholder: defaults.notificationTemplates.noNew,
            },
          },
        ],
      },
      {
        type: "group",
        heading: "Behavior",
        items: [
          {
            name: "Message storage",
            desc: "Choose how regular Discord messages are grouped",
            control: {
              type: "dropdown",
              key: "messageStorageMode",
              defaultValue: defaults.messageStorageMode,
              options: STORAGE_OPTIONS,
            },
          },
          {
            name: "Show author names",
            desc: "Include the Discord author in aggregated logs",
            control: { type: "toggle", key: "showAuthorNames" },
          },
          {
            name: "Show message time",
            desc: "Include the local message time in aggregated logs",
            control: { type: "toggle", key: "showMessageTime" },
          },
          {
            name: "Auto-sync on startup",
            desc: "Automatically sync messages when Obsidian starts",
            control: { type: "toggle", key: "enableAutoSyncOnStartup" },
          },
        ],
      },
    ];
  }

  override getControlValue(key: string): SettingControlValue {
    return read_setting_control(this.plugin.settings, key);
  }

  override async setControlValue(key: string, value: unknown): Promise<void> {
    const normalized = normalize_setting_control(key, value);
    // Apply the single changed field without replacing channels used by active syncs.
    if (
      key === "savedNotificationTemplate" ||
      key === "noNewNotificationTemplate"
    ) {
      Reflect.set(
        this.plugin.settings.notificationTemplates,
        key === "savedNotificationTemplate" ? "saved" : "noNew",
        normalized,
      );
    } else {
      Reflect.set(this.plugin.settings, key, normalized);
    }
    await this.plugin.saveSettings();
  }

  private saveChannelChanges(): void {
    // Obsidian's list callbacks return void; handle persistence failures here.
    void this.plugin.saveSettings().then(
      () => this.update(),
      () => {
        new Notice("Could not save Discord channels. Please try again.");
        this.update();
      },
    );
  }

  private getChannelDefinition(
    channel: DiscordChannelSettings,
    index: number,
  ): SettingDefinition<SettingKey> {
    return {
      name: `Channel ${index + 1}`,
      render: (setting) => {
        setting
          .addText((text) =>
            text
              .setPlaceholder("Name (optional)")
              .setValue(channel.name)
              .onChange(async (value) => {
                try {
                  channel.name = rename_channel(
                    this.plugin.settings.channels,
                    this.plugin.settings.channels.indexOf(channel),
                    value,
                  );
                } catch (error) {
                  new Notice(
                    error instanceof Error ? error.message : String(error),
                  );
                  text.setValue(channel.name);
                  return;
                }
                await this.plugin.saveSettings();
              }),
          )
          .addText((text) =>
            text
              .setPlaceholder("Channel ID")
              .setValue(channel.id)
              .onChange(async (value) => {
                updateChannelId(channel, trim_setting(value));
                await this.plugin.saveSettings();
              }),
          );
      },
    };
  }

  private renderBotToken(setting: Setting): void {
    let textComponent: TextComponent | undefined;
    setting
      .addExtraButton((button) => {
        let isVisible = false;
        button
          .setIcon("eye-off")
          .setTooltip("Toggle password visibility")
          .onClick(() => {
            if (!textComponent) {
              return;
            }
            isVisible = !isVisible;
            textComponent.inputEl.type = isVisible ? "text" : "password";
            button.setIcon(isVisible ? "eye" : "eye-off");
          });
      })
      .addText((text) => {
        textComponent = text;
        text.inputEl.type = "password";
        text
          .setPlaceholder("Paste your bot token")
          .setValue(this.plugin.settings.botToken)
          .onChange(async (value) => {
            this.plugin.settings.botToken = trim_setting(value);
            await this.plugin.saveSettings();
          });
      });
  }
}
