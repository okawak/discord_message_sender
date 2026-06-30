import type { Vault } from "obsidian";
import type { ProcessedMessage } from "./messages";

export type SaveResult = "saved" | "duplicate";

export function saveProcessedMessage(
  vault: Vault,
  messageDirectory: string,
  clippingDirectory: string,
  message: ProcessedMessage,
): Promise<SaveResult> {
  const directory = message.isClipping ? clippingDirectory : messageDirectory;
  return saveIndividualMessage(vault, directory, message);
}

async function saveIndividualMessage(
  vault: Vault,
  directory: string,
  message: ProcessedMessage,
): Promise<SaveResult> {
  await ensureDir(vault, directory);

  const path = `${directory}/${message.fileName}.md`;
  if (vault.getAbstractFileByPath(path)) {
    return "duplicate";
  }

  await vault.create(path, message.markdown);
  return "saved";
}

async function ensureDir(vault: Vault, p: string): Promise<void> {
  if (vault.getFolderByPath(p)) {
    return;
  }

  if (vault.getAbstractFileByPath(p)) {
    throw new Error(
      `Cannot create directory "${p}": a file with the same name already exists`,
    );
  }

  const parent = p.split("/").slice(0, -1).join("/");
  if (parent) {
    await ensureDir(vault, parent);
  }
  await vault.createFolder(p);
}
