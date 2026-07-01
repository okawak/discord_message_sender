import type { MessageStorageMode } from "./settings";

export type AggregatedStorageMode = Exclude<MessageStorageMode, "individual">;

export interface AggregatedLogEntry {
  messageId: string;
  date: string;
  time: string;
  authorName: string;
  markdown: string;
}

export interface AggregatedLogOptions {
  mode: AggregatedStorageMode;
  showAuthorNames: boolean;
  showMessageTime: boolean;
}

export interface AggregatedLogMergeResult {
  content: string;
  addedCount: number;
}

const MESSAGE_ID_PATTERN = /<!-- discord-message-id: (\d+) -->/g;
const DATE_HEADING_PATTERN = /^## (\d{4}-\d{2}-\d{2})\r?$/gm;

export function getAggregatedLogMarker(mode: AggregatedStorageMode): string {
  return `<!-- discord-message-sender: ${mode}-log -->`;
}

export function createAggregatedLog(
  mode: AggregatedStorageMode,
  period: string,
): string {
  return `${getAggregatedLogMarker(mode)}\n# ${period}\n`;
}

export function isManagedAggregatedLog(content: string): boolean {
  return (["daily", "weekly", "monthly"] as const).some((mode) =>
    content.startsWith(getAggregatedLogMarker(mode)),
  );
}

export function getAggregatedMessageIds(content: string): string[] {
  return Array.from(
    content.matchAll(MESSAGE_ID_PATTERN),
    (match) => match[1],
  ).filter((id): id is string => !!id);
}

export function mergeAggregatedLog(
  existingContent: string,
  entries: readonly AggregatedLogEntry[],
  options: AggregatedLogOptions,
): AggregatedLogMergeResult {
  const messageIds = new Set(getAggregatedMessageIds(existingContent));
  let content = existingContent;
  let addedCount = 0;

  for (const entry of entries) {
    if (messageIds.has(entry.messageId)) {
      continue;
    }

    const block = formatEntry(entry, options);
    content =
      options.mode === "daily"
        ? appendBlock(content, block)
        : appendToDateSection(content, entry.date, block);
    messageIds.add(entry.messageId);
    addedCount++;
  }

  return { content, addedCount };
}

function formatEntry(
  entry: AggregatedLogEntry,
  options: AggregatedLogOptions,
): string {
  const details = [
    ...(options.showAuthorNames
      ? [`**${escapeMarkdown(entry.authorName)}**`]
      : []),
    ...(options.showMessageTime ? [entry.time] : []),
  ].join(" · ");
  const marker = `<!-- discord-message-id: ${entry.messageId} -->`;
  return details
    ? `${marker}\n${details}\n\n${entry.markdown}`
    : `${marker}\n${entry.markdown}`;
}

function appendToDateSection(
  content: string,
  date: string,
  block: string,
): string {
  const headings = Array.from(content.matchAll(DATE_HEADING_PATTERN));
  const headingIndex = headings.findIndex((match) => match[1] === date);
  if (headingIndex < 0) {
    return appendBlock(content, `## ${date}\n\n${block}`);
  }

  const nextHeading = headings[headingIndex + 1];
  if (nextHeading?.index === undefined) {
    return appendBlock(content, block);
  }

  const before = appendBlock(content.slice(0, nextHeading.index), block);
  const after = content.slice(nextHeading.index);
  return `${before.endsWith("\n\n") ? before : `${before}\n`}${after}`;
}

function appendBlock(content: string, block: string): string {
  const separator = content.endsWith("\n\n")
    ? ""
    : content.endsWith("\n")
      ? "\n"
      : "\n\n";
  return `${content}${separator}${block}\n`;
}

function escapeMarkdown(value: string): string {
  return value.replace(/[\\`*_[\]{}()#+\-.!|<>]/g, "\\$&");
}
