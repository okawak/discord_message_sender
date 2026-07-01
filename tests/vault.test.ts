import { describe, expect, test } from "bun:test";
import type { TAbstractFile, TFile, TFolder, Vault } from "obsidian";
import { createProcessedMessage } from "../src/messages";
import { saveProcessedMessage } from "../src/vault";

function createVaultMock() {
  const files = new Map<string, string>();
  const folders = new Set<string>();
  const createdPaths: string[] = [];

  const vault = {
    getFolderByPath: (path: string) =>
      folders.has(path) ? ({} as TFolder) : null,
    getAbstractFileByPath: (path: string) =>
      files.has(path) ? ({} as TAbstractFile) : null,
    createFolder: async (path: string) => {
      folders.add(path);
      return {} as TFolder;
    },
    create: async (path: string, content: string) => {
      createdPaths.push(path);
      files.set(path, content);
      return {} as TFile;
    },
  } satisfies Pick<
    Vault,
    "getFolderByPath" | "getAbstractFileByPath" | "createFolder" | "create"
  >;

  return { vault: vault as Vault, files, folders, createdPaths };
}

describe("saveProcessedMessage", () => {
  test("keeps the existing individual message path and content", async () => {
    const { vault, files, folders } = createVaultMock();
    const message = createProcessedMessage(
      "hello",
      false,
      "2026-06-21T03:00:00.000Z",
      "123",
    );

    const result = await saveProcessedMessage(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      message,
    );

    expect(result).toBe("saved");
    expect(folders).toEqual(new Set(["DiscordLogs", "DiscordLogs/general"]));
    expect(files).toEqual(
      new Map([["DiscordLogs/general/20260621_120000_123.md", "hello"]]),
    );
  });

  test("routes URL clippings to the clipping directory", async () => {
    const { vault, files } = createVaultMock();
    const message = createProcessedMessage(
      "# Example",
      true,
      "2026-06-21T03:00:00.000Z",
      "123",
    );

    const result = await saveProcessedMessage(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      message,
    );

    expect(result).toBe("saved");
    expect(files).toEqual(
      new Map([
        ["DiscordClippings/general/20260621_120000_123.md", "# Example"],
      ]),
    );
  });

  test("does not overwrite an existing individual file", async () => {
    const { vault, files, folders, createdPaths } = createVaultMock();
    folders.add("DiscordLogs");
    folders.add("DiscordLogs/general");
    const path = "DiscordLogs/general/20260621_120000_123.md";
    files.set(path, "existing");
    const message = createProcessedMessage(
      "replacement",
      false,
      "2026-06-21T03:00:00.000Z",
      "123",
    );

    const result = await saveProcessedMessage(
      vault,
      "DiscordLogs/general",
      "DiscordClippings/general",
      message,
    );

    expect(result).toBe("duplicate");
    expect(files.get(path)).toBe("existing");
    expect(createdPaths).toEqual([]);
  });

  test("rejects a file that blocks the target directory", async () => {
    const { vault, files } = createVaultMock();
    files.set("DiscordLogs", "blocking file");
    const message = createProcessedMessage(
      "hello",
      false,
      "2026-06-21T03:00:00.000Z",
      "123",
    );

    await expect(
      saveProcessedMessage(
        vault,
        "DiscordLogs/general",
        "DiscordClippings/general",
        message,
      ),
    ).rejects.toThrow(
      'Cannot create directory "DiscordLogs": a file with the same name already exists',
    );
  });
});
