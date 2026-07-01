import { describe, expect, test } from "bun:test";
import type { TFile, TFolder, Vault } from "obsidian";
import { createProcessedMessage, type ProcessedMessage } from "../src/messages";
import {
  type MessageStorageOptions,
  saveProcessedMessages,
} from "../src/vault";

function createVaultMock() {
  const files = new Map<string, string>();
  const folders = new Set<string>();
  const createdPaths: string[] = [];
  const processCalls = new Map<string, number>();

  const asFile = (path: string) =>
    ({
      path,
      name: path.slice(path.lastIndexOf("/") + 1),
    }) as TFile;
  const asFolder = (path: string) =>
    ({
      path,
      name: path.slice(path.lastIndexOf("/") + 1),
      children: Array.from(files.keys())
        .filter(
          (filePath) => filePath.slice(0, filePath.lastIndexOf("/")) === path,
        )
        .map(asFile),
    }) as unknown as TFolder;

  const vault = {
    getFolderByPath: (path: string) =>
      folders.has(path) ? asFolder(path) : null,
    getFileByPath: (path: string) => (files.has(path) ? asFile(path) : null),
    getAbstractFileByPath: (path: string) => {
      if (files.has(path)) {
        return asFile(path);
      }
      return folders.has(path) ? asFolder(path) : null;
    },
    createFolder: async (path: string) => {
      folders.add(path);
      return asFolder(path);
    },
    create: async (path: string, content: string) => {
      createdPaths.push(path);
      files.set(path, content);
      return asFile(path);
    },
    cachedRead: async (file: TFile) => files.get(file.path) ?? "",
    process: async (file: TFile, update: (content: string) => string) => {
      processCalls.set(file.path, (processCalls.get(file.path) ?? 0) + 1);
      const content = update(files.get(file.path) ?? "");
      files.set(file.path, content);
      return content;
    },
  } satisfies Pick<
    Vault,
    | "getFolderByPath"
    | "getFileByPath"
    | "getAbstractFileByPath"
    | "createFolder"
    | "create"
    | "cachedRead"
    | "process"
  >;

  return {
    vault: vault as Vault,
    files,
    folders,
    createdPaths,
    processCalls,
  };
}

function createMessage(
  id: string,
  timestamp: string,
  markdown = "hello",
  isClipping = false,
): ProcessedMessage {
  return createProcessedMessage(
    markdown,
    isClipping,
    {
      id,
      content: markdown,
      timestamp,
      author: { id: `author-${id}`, username: "Alice" },
    },
    "Asia/Tokyo",
  );
}

function storageOptions(
  messageStorageMode: MessageStorageOptions["messageStorageMode"],
): MessageStorageOptions {
  return {
    messageStorageMode,
    showAuthorNames: false,
    showMessageTime: false,
    timeZone: "Asia/Tokyo",
  };
}

describe("saveProcessedMessages", () => {
  test("keeps the existing individual message path and content", async () => {
    const { vault, files, folders } = createVaultMock();

    const count = await saveProcessedMessages(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      [createMessage("123", "2026-06-21T03:00:00.000Z")],
      storageOptions("individual"),
    );

    expect(count).toBe(1);
    expect(folders).toEqual(new Set(["DiscordLogs", "DiscordLogs/general"]));
    expect(files).toEqual(
      new Map([["DiscordLogs/general/20260621_120000_123.md", "hello"]]),
    );
  });

  test("keeps URL clippings as individual files in aggregated modes", async () => {
    const { vault, files } = createVaultMock();

    const count = await saveProcessedMessages(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      [createMessage("123", "2026-06-21T03:00:00.000Z", "# Example", true)],
      storageOptions("monthly"),
    );

    expect(count).toBe(1);
    expect(files).toEqual(
      new Map([
        ["DiscordClippings/general/20260621_120000_123.md", "# Example"],
      ]),
    );
  });

  test("creates daily, weekly, and monthly files", async () => {
    for (const [mode, fileName, marker] of [
      ["daily", "2026-06-29.md", "daily-log"],
      ["weekly", "2026-W27.md", "weekly-log"],
      ["monthly", "2026-06.md", "monthly-log"],
    ] as const) {
      const { vault, files } = createVaultMock();

      const count = await saveProcessedMessages(
        vault,
        "DiscordLogs/general",
        "DiscordClippings/general",
        [createMessage("123", "2026-06-29T12:34:00.000Z", "yes")],
        storageOptions(mode),
      );
      const content = files.get(`DiscordLogs/general/${fileName}`);

      expect(count).toBe(1);
      expect(content).toStartWith(`<!-- discord-message-sender: ${marker} -->`);
      expect(content).toContain("<!-- discord-message-id: 123 -->\nyes");
      if (mode === "daily") {
        expect(content).not.toContain("## 2026-06-29");
      } else {
        expect(content?.match(/^## 2026-06-29$/gm)).toHaveLength(1);
      }
    }
  });

  test("processes one existing file once for multiple messages", async () => {
    const { vault, files, folders, processCalls } = createVaultMock();
    folders.add("DiscordLogs");
    folders.add("DiscordLogs/general");
    const path = "DiscordLogs/general/2026-06.md";
    files.set(
      path,
      "<!-- discord-message-sender: monthly-log -->\n# 2026-06\n",
    );

    const count = await saveProcessedMessages(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      [
        createMessage("123", "2026-06-29T12:34:00.000Z", "first"),
        createMessage("124", "2026-06-29T12:35:00.000Z", "second"),
      ],
      storageOptions("monthly"),
    );

    expect(count).toBe(2);
    expect(processCalls.get(path)).toBe(1);
    expect(files.get(path)?.match(/^## 2026-06-29$/gm)).toHaveLength(1);
  });

  test("does not duplicate messages after a retry", async () => {
    const { vault, files, folders, processCalls } = createVaultMock();
    folders.add("DiscordLogs");
    folders.add("DiscordLogs/general");
    const path = "DiscordLogs/general/2026-06.md";
    const existing = [
      "<!-- discord-message-sender: monthly-log -->",
      "# 2026-06",
      "",
      "User edit",
      "",
      "## 2026-06-29",
      "",
      "<!-- discord-message-id: 123 -->",
      "first",
      "",
    ].join("\n");
    files.set(path, existing);

    const count = await saveProcessedMessages(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      [createMessage("123", "2026-06-29T12:34:00.000Z", "replacement")],
      storageOptions("monthly"),
    );

    expect(count).toBe(0);
    expect(files.get(path)).toBe(existing);
    expect(processCalls.get(path)).toBeUndefined();
  });

  test("detects duplicates across individual and aggregated formats", async () => {
    const { vault, files, folders } = createVaultMock();
    folders.add("DiscordLogs");
    folders.add("DiscordLogs/general");
    files.set("DiscordLogs/general/20260629_213400_123.md", "individual");

    const monthlyCount = await saveProcessedMessages(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      [createMessage("123", "2026-06-29T12:34:00.000Z")],
      storageOptions("monthly"),
    );

    files.set(
      "DiscordLogs/general/2026-W27.md",
      [
        "<!-- discord-message-sender: weekly-log -->",
        "# 2026-W27",
        "",
        "## 2026-06-29",
        "",
        "<!-- discord-message-id: 124 -->",
        "weekly",
        "",
      ].join("\n"),
    );
    const changedFormatCount = await saveProcessedMessages(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      [createMessage("124", "2026-06-29T12:35:00.000Z")],
      storageOptions("monthly"),
    );

    expect(monthlyCount).toBe(0);
    expect(changedFormatCount).toBe(0);
    expect(files.has("DiscordLogs/general/2026-06.md")).toBe(false);
  });

  test("rejects unmanaged files and folder collisions", async () => {
    const unmanaged = createVaultMock();
    unmanaged.folders.add("DiscordLogs");
    unmanaged.folders.add("DiscordLogs/general");
    unmanaged.files.set("DiscordLogs/general/2026-06.md", "User file");

    await expect(
      saveProcessedMessages(
        unmanaged.vault,
        "DiscordLogs/general",
        "DiscordClippings/general",
        [createMessage("123", "2026-06-29T12:34:00.000Z")],
        storageOptions("monthly"),
      ),
    ).rejects.toThrow("the existing file is not managed");

    const collision = createVaultMock();
    collision.folders.add("DiscordLogs");
    collision.folders.add("DiscordLogs/general");
    collision.folders.add("DiscordLogs/general/2026-06.md");

    await expect(
      saveProcessedMessages(
        collision.vault,
        "DiscordLogs/general",
        "DiscordClippings/general",
        [createMessage("123", "2026-06-29T12:34:00.000Z")],
        storageOptions("monthly"),
      ),
    ).rejects.toThrow("a folder exists at this path");
  });

  test("keeps two channel logs separate", async () => {
    const { vault, files } = createVaultMock();

    await saveProcessedMessages(
      vault,
      "DiscordLogs/first",
      "DiscordClippings/first",
      [createMessage("123", "2026-06-29T12:34:00.000Z")],
      storageOptions("monthly"),
    );
    await saveProcessedMessages(
      vault,
      "DiscordLogs/second",
      "DiscordClippings/second",
      [createMessage("124", "2026-06-29T12:35:00.000Z")],
      storageOptions("monthly"),
    );

    expect(files.has("DiscordLogs/first/2026-06.md")).toBe(true);
    expect(files.has("DiscordLogs/second/2026-06.md")).toBe(true);
  });

  test("rejects invalid timestamps before writing files", async () => {
    const { vault, files, createdPaths } = createVaultMock();
    const invalid = {
      ...createMessage("123", "2026-06-29T12:34:00.000Z"),
      timestamp: "invalid",
    };

    await expect(
      saveProcessedMessages(
        vault,
        "DiscordLogs/general",
        "DiscordClippings/general",
        [invalid],
        storageOptions("daily"),
      ),
    ).rejects.toThrow('Invalid Discord message timestamp: "invalid".');
    expect(files.size).toBe(0);
    expect(createdPaths).toEqual([]);
  });
});
