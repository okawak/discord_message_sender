import { TFolder, type Vault } from "obsidian";
import type { ProcessedMessage } from "./settings";

// Save to Obsidian vault
export async function saveToVault(
  vault: Vault,
  msgDir: string,
  clipDir: string,
  data: ProcessedMessage,
): Promise<void> {
  const dir = data.isClipping ? clipDir : msgDir;
  await ensureDir(vault, dir);

  const fileName = data.fileName || Date.now().toString();
  const path = `${dir}/${fileName}.md`;

  // Check if the file already exists
  if (!vault.getAbstractFileByPath(path)) {
    await vault.create(path, data.markdown);
  }
}

async function ensureDir(vault: Vault, p: string): Promise<void> {
  const existing = vault.getAbstractFileByPath(p);
  if (existing instanceof TFolder) return;

  if (existing) {
    // If it exists but is not a folder, throw an error
    throw new Error(
      `Cannot create directory "${p}": a file with the same name already exists`,
    );
  }

  const parent = p.split("/").slice(0, -1).join("/");
  if (parent) {
    // recursively ensure parent directory exists
    await ensureDir(vault, parent);
  }
  await vault.createFolder(p);
}
