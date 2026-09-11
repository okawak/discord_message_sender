import type { Vault } from "obsidian";
import {
  type AggregatedLogGroup,
  create_aggregated_log as createAggregatedLog,
  aggregated_message_ids as getAggregatedMessageIds,
  has_aggregated_log_marker as hasAggregatedLogMarker,
  individual_message_id,
  is_managed_log as isManagedAggregatedLog,
  type MessageStorageOptions,
  merge_aggregated_log as mergeAggregatedLog,
  type ProcessedMessage,
  plan_message_storage,
  type StorageInput,
  storage_candidate_paths,
} from "../pkg/parse_message.js";

export class MessageStorageError extends Error {
  override name = "MessageStorageError";
}

export async function saveProcessedMessages(
  vault: Vault,
  messageDirectory: string,
  clippingDirectory: string,
  messages: readonly ProcessedMessage[],
  options: MessageStorageOptions,
): Promise<number> {
  const input: StorageInput = {
    messageDirectory,
    clippingDirectory,
    messages: [...messages],
    options,
    existingIds: [],
    existingClippingIds: [],
  };
  // Rust determines which files need inspection, including previous time zones/modes.
  const paths = storage_candidate_paths(input);
  input.existingIds = findIndividualMessageIds(vault, messageDirectory);
  input.existingClippingIds = findIndividualMessageIds(
    vault,
    clippingDirectory,
  );
  for (const path of paths) {
    const file = vault.getFileByPath(path);
    if (!file) continue;
    const content = await vault.read(file);
    if (isManagedAggregatedLog(content))
      input.existingIds.push(...getAggregatedMessageIds(content));
  }

  const plan = plan_message_storage(input);
  let savedCount = 0;
  for (const write of plan.individual) {
    await ensureDir(vault, write.directory);
    const existing = vault.getAbstractFileByPath(write.path);
    if (existing) {
      if (!vault.getFileByPath(write.path)) {
        throw new MessageStorageError(
          `a folder exists at "${write.path}"; move or rename it, then sync again`,
        );
      }
      continue;
    }
    await vault.create(write.path, write.content);
    savedCount++;
  }
  for (const group of plan.groups)
    savedCount += await saveAggregatedLog(vault, group, options);
  return savedCount;
}

function findIndividualMessageIds(vault: Vault, directory: string): string[] {
  const ids: string[] = [];
  const folder = vault.getFolderByPath(directory);
  for (const child of folder?.children ?? []) {
    const file = vault.getFileByPath(child.path);
    const id = file ? individual_message_id(file.name) : undefined;
    if (id) ids.push(id);
  }
  return ids;
}

async function saveAggregatedLog(
  vault: Vault,
  group: AggregatedLogGroup,
  options: MessageStorageOptions,
): Promise<number> {
  const directory = group.path.slice(0, group.path.lastIndexOf("/"));
  await ensureDir(vault, directory);

  const existing = vault.getAbstractFileByPath(group.path);
  if (!existing) {
    const result = mergeAggregatedLog(
      createAggregatedLog(group.mode, group.period),
      group.entries,
      { mode: group.mode, ...options },
    );
    await vault.create(group.path, result.content);
    return result.addedCount;
  }

  const file = vault.getFileByPath(group.path);
  if (!file) {
    throw new MessageStorageError(
      `a folder exists at "${group.path}"; move or rename it, then sync again`,
    );
  }

  let addedCount = 0;
  await vault.process(file, (content) => {
    if (!hasAggregatedLogMarker(content, group.mode)) {
      throw new MessageStorageError(
        `"${group.path}" is not a ${group.mode} log managed by Discord Message Sender; move or rename it, then sync again`,
      );
    }
    const result = mergeAggregatedLog(content, group.entries, {
      mode: group.mode,
      ...options,
    });
    addedCount = result.addedCount;
    return result.content;
  });
  return addedCount;
}

async function ensureDir(vault: Vault, path: string): Promise<void> {
  if (vault.getFolderByPath(path)) {
    return;
  }

  if (vault.getAbstractFileByPath(path)) {
    throw new MessageStorageError(
      `a file blocks the directory "${path}"; move or rename it, then sync again`,
    );
  }

  const parent = path.split("/").slice(0, -1).join("/");
  if (parent) {
    await ensureDir(vault, parent);
  }
  await vault.createFolder(path);
}
