import {
  type App,
  Notice,
  PluginSettingTab,
  Setting,
  type SettingControl,
  type SettingDefinition,
  type SettingDefinitionGroup,
  type SettingDefinitionItem,
  type SettingDefinitionList,
  type TextComponent,
} from "obsidian";
import {
  findDuplicateChannelPathSegment,
  getChannelNameValidationError,
} from "./channelPaths";
import type DiscordMessageSenderPlugin from "./main";
import {
  DEFAULT_NOTIFICATION_TEMPLATES,
  type DiscordChannelSettings,
  type MessageStorageMode,
  updateChannelId,
} from "./settings";

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

  /**
   * Compatibility renderer for Obsidian versions before 1.13.
   * Newer versions render getSettingDefinitions() without calling this method.
   */
  override display(): void {
    this.containerEl.empty();
    for (const item of this.getSettingDefinitions()) {
      if ("type" in item && (item.type === "group" || item.type === "list")) {
        this.renderLegacyGroup(item);
      }
    }
  }

  override getSettingDefinitions(): SettingDefinitionItem<SettingKey>[] {
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
              defaultValue: "DiscordLogs",
              placeholder: "DiscordLogs",
            },
          },
          {
            name: "Clippings directory",
            desc: "Directory where URL clippings will be saved",
            control: {
              type: "text",
              key: "clippingDirectoryName",
              defaultValue: "DiscordClippings",
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
              defaultValue: "!",
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
          action: async () => {
            this.plugin.settings.channels.push({ id: "", name: "" });
            await this.plugin.saveSettings();
            this.refresh();
          },
        },
        onDelete: async (index) => {
          this.plugin.settings.channels.splice(index, 1);
          await this.plugin.saveSettings();
          this.refresh();
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
              defaultValue: DEFAULT_NOTIFICATION_TEMPLATES.saved,
              placeholder: DEFAULT_NOTIFICATION_TEMPLATES.saved,
            },
          },
          {
            name: "No new messages template",
            desc: "Discord message sent when there are no new messages. Available variables: {count}, {channelName}, {channelId}",
            control: {
              type: "textarea",
              key: "noNewNotificationTemplate",
              defaultValue: DEFAULT_NOTIFICATION_TEMPLATES.noNew,
              placeholder: DEFAULT_NOTIFICATION_TEMPLATES.noNew,
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
              defaultValue: "individual",
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

  override getControlValue(key: string): unknown {
    const settings = this.plugin.settings;
    switch (key) {
      case "messageDirectoryName":
      case "clippingDirectoryName":
      case "messagePrefix":
      case "messageStorageMode":
      case "showAuthorNames":
      case "showMessageTime":
      case "enableAutoSyncOnStartup":
      case "sendSyncNotifications":
        return settings[key];
      case "savedNotificationTemplate":
        return settings.notificationTemplates.saved;
      case "noNewNotificationTemplate":
        return settings.notificationTemplates.noNew;
      default:
        return undefined;
    }
  }

  override async setControlValue(key: string, value: unknown): Promise<void> {
    const settings = this.plugin.settings;
    switch (key) {
      case "messageDirectoryName":
        settings.messageDirectoryName = readString(value, "DiscordLogs");
        break;
      case "clippingDirectoryName":
        settings.clippingDirectoryName = readString(value, "DiscordClippings");
        break;
      case "messagePrefix":
        settings.messagePrefix = readString(value, "!");
        break;
      case "messageStorageMode":
        if (!isMessageStorageMode(value)) {
          throw new TypeError(
            `Invalid message storage mode "${String(value)}".`,
          );
        }
        settings.messageStorageMode = value;
        break;
      case "showAuthorNames":
      case "showMessageTime":
      case "enableAutoSyncOnStartup":
      case "sendSyncNotifications":
        settings[key] = readBoolean(value, key);
        break;
      case "savedNotificationTemplate":
        settings.notificationTemplates.saved = readString(
          value,
          DEFAULT_NOTIFICATION_TEMPLATES.saved,
        );
        break;
      case "noNewNotificationTemplate":
        settings.notificationTemplates.noNew = readString(
          value,
          DEFAULT_NOTIFICATION_TEMPLATES.noNew,
        );
        break;
      default:
        throw new TypeError(`Unknown setting key "${key}".`);
    }
    await this.plugin.saveSettings();
  }

  private renderLegacyGroup(group: SettingDefinitionGroup<SettingKey>): void {
    const list =
      group.type === "list"
        ? (group as SettingDefinitionList<SettingKey>)
        : undefined;

    if (group.heading) {
      const heading = new Setting(this.containerEl).setName(group.heading);
      if (list?.addItem) {
        const { addItem } = list;
        heading.addButton((button) =>
          button
            .setButtonText(addItem.name)
            .setCta()
            .onClick(() => addItem.action(button.buttonEl)),
        );
      } else {
        heading.setHeading();
      }
    }

    if (list?.emptyState && group.items?.length === 0) {
      new Setting(this.containerEl).setName(list.emptyState);
    }

    group.items?.forEach((definition, index) => {
      if ("type" in definition) {
        return;
      }
      const setting = new Setting(this.containerEl).setName(definition.name);
      if (definition.desc) {
        setting.setDesc(definition.desc);
      }
      if ("control" in definition && definition.control) {
        this.renderLegacyControl(setting, definition.control);
      } else if ("render" in definition) {
        const render = definition.render as (setting: Setting) => void;
        render(setting);
      }
      if (list?.onDelete) {
        const { onDelete } = list;
        setting.addExtraButton((button) =>
          button
            .setIcon("trash")
            .setTooltip("Remove channel")
            .onClick(() => onDelete(index)),
        );
      }
    });
  }

  private renderLegacyControl(
    setting: Setting,
    control: SettingControl<SettingKey>,
  ): void {
    const value = this.getControlValue(control.key) ?? control.defaultValue;
    switch (control.type) {
      case "text":
        setting.addText((text) =>
          text
            .setPlaceholder(control.placeholder ?? "")
            .setValue(typeof value === "string" ? value : "")
            .onChange((nextValue) =>
              this.setControlValue(control.key, nextValue),
            ),
        );
        return;
      case "textarea":
        setting.addTextArea((text) =>
          text
            .setPlaceholder(control.placeholder ?? "")
            .setValue(typeof value === "string" ? value : "")
            .onChange((nextValue) =>
              this.setControlValue(control.key, nextValue),
            ),
        );
        return;
      case "dropdown":
        setting.addDropdown((dropdown) =>
          dropdown
            .addOptions(control.options)
            .setValue(typeof value === "string" ? value : "")
            .onChange((nextValue) =>
              this.setControlValue(control.key, nextValue),
            ),
        );
        return;
      case "toggle":
        setting.addToggle((toggle) =>
          toggle
            .setValue(value === true)
            .onChange((nextValue) =>
              this.setControlValue(control.key, nextValue),
            ),
        );
        return;
      default:
        throw new Error(
          `Unsupported legacy setting control "${control.type}".`,
        );
    }
  }

  private refresh(): void {
    const update: unknown = Reflect.get(this, "update");
    if (typeof update === "function") {
      update.call(this);
    } else {
      this.display();
    }
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
                const name = value.trim();
                const error = getChannelNameValidationError(name);
                const duplicate = findDuplicateChannelPathSegment(
                  this.plugin.settings.channels.map((candidate) =>
                    candidate === channel ? { ...candidate, name } : candidate,
                  ),
                );
                if (error || duplicate) {
                  new Notice(
                    error ??
                      `Channel name is already in use as folder "${duplicate}". Enter a unique channel name.`,
                  );
                  text.setValue(channel.name);
                  return;
                }
                channel.name = name;
                await this.plugin.saveSettings();
              }),
          )
          .addText((text) =>
            text
              .setPlaceholder("Channel ID")
              .setValue(channel.id)
              .onChange(async (value) => {
                updateChannelId(channel, value.trim());
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
          .setPlaceholder("xxxxx.yyyyy.zzzzz")
          .setValue(this.plugin.settings.botToken)
          .onChange(async (value) => {
            this.plugin.settings.botToken = value.trim();
            await this.plugin.saveSettings();
          });
      });
  }
}

function readString(value: unknown, fallback: string): string {
  return typeof value === "string" ? value.trim() || fallback : fallback;
}

function readBoolean(value: unknown, key: string): boolean {
  if (typeof value !== "boolean") {
    throw new TypeError(`Setting "${key}" requires a boolean value.`);
  }
  return value;
}

function isMessageStorageMode(value: unknown): value is MessageStorageMode {
  return typeof value === "string" && Object.hasOwn(STORAGE_OPTIONS, value);
}
