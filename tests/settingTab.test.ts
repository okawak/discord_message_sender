import { describe, expect, mock, test } from "bun:test";
import type { App, SettingDefinitionList } from "obsidian";
import type DiscordMessageSenderPlugin from "../src/main";
import { normalizeSettings } from "../src/settings";

const notices: string[] = [];
mock.module("obsidian", () => ({
  PluginSettingTab: class {
    update() {}
  },
  Notice: class {
    constructor(message: string) {
      notices.push(message);
    }
  },
}));

const { DiscordMessageSenderSettingTab } = await import("../src/settingTab");

function createTab(save = async () => {}) {
  const plugin = {
    settings: normalizeSettings({
      channels: [
        { id: "123", name: "Notes", lastProcessedMessageId: "456" },
        { id: "789", name: "Links" },
      ],
    }),
    saveSettings: mock(save),
  };
  const tab = new DiscordMessageSenderSettingTab(
    {} as App,
    plugin as unknown as DiscordMessageSenderPlugin,
  );
  const update = mock(() => {});
  tab.update = update;
  const channels = () => {
    const list = tab
      .getSettingDefinitions()
      .find((item) => "type" in item && item.type === "list");
    if (!list) throw new Error("Channel settings are missing");
    return list as SettingDefinitionList;
  };
  return { tab, plugin, update, channels };
}

describe("Obsidian 1.13 settings", () => {
  test("search indexing reads definitions without saving or changing channels", () => {
    const { tab, plugin, update } = createTab();
    const original = structuredClone(plugin.settings);
    tab.getSettingDefinitions();
    tab.getSettingDefinitions();
    expect(plugin.settings).toEqual(original);
    expect(plugin.saveSettings).not.toHaveBeenCalled();
    expect(update).not.toHaveBeenCalled();
  });

  test("adding a channel returns void and refreshes only after persistence", async () => {
    const pending = Promise.withResolvers<void>();
    const { channels, plugin, update } = createTab(() => pending.promise);
    expect(channels().addItem?.action({} as HTMLElement)).toBeUndefined();
    expect(plugin.settings.channels).toHaveLength(3);
    expect(plugin.settings.channels[0]?.lastProcessedMessageId).toBe("456");
    expect(update).not.toHaveBeenCalled();
    pending.resolve();
    await pending.promise;
    expect(update).toHaveBeenCalledTimes(1);
  });

  test("deleting rows uses the current list index after a previous deletion", async () => {
    const { channels, plugin, update } = createTab();
    expect(channels().onDelete?.(0)).toBeUndefined();
    await Promise.resolve();
    expect(plugin.settings.channels.map((channel) => channel.id)).toEqual([
      "789",
    ]);
    expect(channels().onDelete?.(0)).toBeUndefined();
    await Promise.resolve();
    expect(plugin.settings.channels).toEqual([]);
    expect(plugin.saveSettings).toHaveBeenCalledTimes(2);
    expect(update).toHaveBeenCalledTimes(2);
  });

  test("failed channel saves are handled and reported without exposing error contents", async () => {
    const { channels, update } = createTab(async () => {
      throw new Error("private error details");
    });
    notices.length = 0;
    expect(channels().addItem?.action({} as HTMLElement)).toBeUndefined();
    await Promise.resolve();
    expect(notices).toEqual([
      "Could not save Discord channels. Please try again.",
    ]);
    expect(update).toHaveBeenCalledTimes(1);
  });

  test("declarative controls persist nested templates and normalize empty input", async () => {
    const { tab, plugin } = createTab();
    await tab.setControlValue("savedNotificationTemplate", "  Saved {count}  ");
    await tab.setControlValue("messageDirectoryName", "   ");
    await tab.setControlValue("sendSyncNotifications", false);
    expect(plugin.settings.notificationTemplates.saved).toBe("Saved {count}");
    expect(tab.getControlValue("savedNotificationTemplate")).toBe(
      "Saved {count}",
    );
    expect(plugin.settings.messageDirectoryName).toBe("DiscordLogs");
    expect(plugin.settings.sendSyncNotifications).toBe(false);
    expect(plugin.saveSettings).toHaveBeenCalledTimes(3);
  });

  test("invalid control values are rejected without persisting", async () => {
    const { tab, plugin } = createTab();
    await expect(
      tab.setControlValue("messageStorageMode", "invalid"),
    ).rejects.toThrow(TypeError);
    await expect(
      tab.setControlValue("showAuthorNames", "true"),
    ).rejects.toThrow(TypeError);
    expect(plugin.saveSettings).not.toHaveBeenCalled();
  });

  test("returns concrete control values and preserves live channel references", async () => {
    const { tab, plugin } = createTab();
    const channel = plugin.settings.channels[0];
    expect(tab.getControlValue("messageDirectoryName")).toBe("DiscordLogs");
    expect(tab.getControlValue("showAuthorNames")).toBe(false);
    expect(tab.getControlValue("missing")).toBeNull();
    for (const mode of ["individual", "daily", "weekly", "monthly"]) {
      await tab.setControlValue("messageStorageMode", mode);
      expect(tab.getControlValue("messageStorageMode")).toBe(mode);
    }
    expect(plugin.settings.channels[0]).toBe(channel);
    expect(channel?.lastProcessedMessageId).toBe("456");
  });
});
