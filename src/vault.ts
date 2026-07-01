import type { Vault } from "obsidian";
import {
  type AggregatedLogEntry,
  type AggregatedStorageMode,
  createAggregatedLog,
  getAggregatedLogMarker,
  getAggregatedMessageIds,
  isManagedAggregatedLog,
  mergeAggregatedLog,
} from "./aggregatedLog";
import { type LocalDateTime, toLocalDateTime } from "./localDateTime";
import type { ProcessedMessage } from "./messages";
import type { MessageStorageMode } from "./settings";

export interface MessageStorageOptions {
  messageStorageMode: MessageStorageMode;
  showAuthorNames: boolean;
  showMessageTime: boolean;
  timeZone: string;
}

interface PreparedMessage {
  message: ProcessedMessage;
  localDateTime: LocalDateTime;
}

interface AggregatedLogTarget {
  mode: AggregatedStorageMode;
  period: string;
  path: string;
}

interface AggregatedLogGroup extends AggregatedLogTarget {
  entries: AggregatedLogEntry[];
}

const AGGREGATED_STORAGE_MODES = ["daily", "weekly", "monthly"] as const;
const INDIVIDUAL_MESSAGE_ID_PATTERN = /_(\d+)\.md$/;

export async function saveProcessedMessages(
  vault: Vault,
  messageDirectory: string,
  clippingDirectory: string,
  messages: readonly ProcessedMessage[],
  options: MessageStorageOptions,
): Promise<number> {
  const prepared = messages.map((message) => ({
    message,
    localDateTime: toLocalDateTime(message.timestamp, options.timeZone),
  }));
  const regularMessages = prepared.filter(({ message }) => !message.isClipping);
  const existingIds = await findExistingMessageIds(
    vault,
    messageDirectory,
    regularMessages,
  );
  const groups = new Map<string, AggregatedLogGroup>();
  let savedCount = 0;

  for (const preparedMessage of prepared) {
    const { message, localDateTime } = preparedMessage;
    if (message.isClipping) {
      if (
        (await saveIndividualMessage(vault, clippingDirectory, message)) ===
        "saved"
      ) {
        savedCount++;
      }
      continue;
    }

    if (existingIds.has(message.messageId)) {
      continue;
    }
    existingIds.add(message.messageId);

    if (options.messageStorageMode === "individual") {
      if (
        (await saveIndividualMessage(vault, messageDirectory, message)) ===
        "saved"
      ) {
        savedCount++;
      }
      continue;
    }

    const target = getAggregatedLogTarget(
      messageDirectory,
      options.messageStorageMode,
      localDateTime,
    );
    const group = groups.get(target.path) ?? { ...target, entries: [] };
    group.entries.push({
      messageId: message.messageId,
      date: localDateTime.date,
      time: localDateTime.time,
      authorName: message.authorName,
      markdown: message.markdown,
    });
    groups.set(target.path, group);
  }

  for (const group of groups.values()) {
    savedCount += await saveAggregatedLog(vault, group, options);
  }
  return savedCount;
}

async function findExistingMessageIds(
  vault: Vault,
  messageDirectory: string,
  messages: readonly PreparedMessage[],
): Promise<Set<string>> {
  const messageIds = new Set<string>();
  const folder = vault.getFolderByPath(messageDirectory);
  for (const child of folder?.children ?? []) {
    const messageId = child.name.match(INDIVIDUAL_MESSAGE_ID_PATTERN)?.[1];
    if (messageId) {
      messageIds.add(messageId);
    }
  }

  const paths = new Set<string>();
  for (const { localDateTime } of messages) {
    for (const mode of AGGREGATED_STORAGE_MODES) {
      paths.add(
        getAggregatedLogTarget(messageDirectory, mode, localDateTime).path,
      );
    }
  }

  for (const path of paths) {
    const file = vault.getFileByPath(path);
    if (!file) {
      continue;
    }
    const content = await vault.cachedRead(file);
    if (isManagedAggregatedLog(content)) {
      for (const messageId of getAggregatedMessageIds(content)) {
        messageIds.add(messageId);
      }
    }
  }
  return messageIds;
}

async function saveIndividualMessage(
  vault: Vault,
  directory: string,
  message: ProcessedMessage,
): Promise<"saved" | "duplicate"> {
  await ensureDir(vault, directory);

  const path = `${directory}/${message.fileName}.md`;
  if (vault.getAbstractFileByPath(path)) {
    return "duplicate";
  }

  await vault.create(path, message.markdown);
  return "saved";
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
    throw new Error(
      `Cannot update aggregated log "${group.path}": a folder exists at this path.`,
    );
  }

  let addedCount = 0;
  await vault.process(file, (content) => {
    if (!content.startsWith(getAggregatedLogMarker(group.mode))) {
      throw new Error(
        `Cannot update aggregated log "${group.path}": the existing file is not managed by Discord Message Sender.`,
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

function getAggregatedLogTarget(
  directory: string,
  mode: AggregatedStorageMode,
  localDateTime: LocalDateTime,
): AggregatedLogTarget {
  const period =
    mode === "daily"
      ? localDateTime.date
      : mode === "weekly"
        ? localDateTime.week
        : localDateTime.month;
  return { mode, period, path: `${directory}/${period}.md` };
}

async function ensureDir(vault: Vault, path: string): Promise<void> {
  if (vault.getFolderByPath(path)) {
    return;
  }

  if (vault.getAbstractFileByPath(path)) {
    throw new Error(
      `Cannot create directory "${path}": a file with the same name already exists`,
    );
  }

  const parent = path.split("/").slice(0, -1).join("/");
  if (parent) {
    await ensureDir(vault, parent);
  }
  await vault.createFolder(path);
}
