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

const MESSAGE_ID_PATTERN = /^<!-- discord-message-id: (\d+) -->\r?$/gm;
const DATE_SECTION_PATTERN =
  /^<!-- discord-message-date: (\d{4}-\d{2}-\d{2}) -->\r?\n## \1\r?$/gm;
const RESERVED_MARKER_PATTERN = /<!-- discord-message-(id|date):/g;

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
    hasAggregatedLogMarker(content, mode),
  );
}

export function hasAggregatedLogMarker(
  content: string,
  mode: AggregatedStorageMode,
): boolean {
  return content.startsWith(
    getAggregatedLogMarker(mode),
    getMarkerOffset(content),
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
  const pendingEntries = entries.filter((entry) => {
    if (messageIds.has(entry.messageId)) {
      return false;
    }
    messageIds.add(entry.messageId);
    return true;
  });
  if (pendingEntries.length === 0) {
    return { content: existingContent, addedCount: 0 };
  }

  if (options.mode === "daily") {
    return {
      content: appendBlock(
        existingContent,
        pendingEntries.map((entry) => formatEntry(entry, options)).join("\n\n"),
      ),
      addedCount: pendingEntries.length,
    };
  }

  const entriesByDate = new Map<string, AggregatedLogEntry[]>();
  for (const entry of pendingEntries) {
    const dateEntries = entriesByDate.get(entry.date) ?? [];
    dateEntries.push(entry);
    entriesByDate.set(entry.date, dateEntries);
  }
  let content = existingContent;
  for (const [date, dateEntries] of entriesByDate) {
    const block = dateEntries
      .map((entry) => formatEntry(entry, options))
      .join("\n\n");
    content = appendToDateSection(content, date, block);
  }
  return { content, addedCount: pendingEntries.length };
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
  const markdown = entry.markdown.replace(
    RESERVED_MARKER_PATTERN,
    "<!-- discord-message-$1 :",
  );
  return details
    ? `${marker}\n${details}\n\n${markdown}`
    : `${marker}\n${markdown}`;
}

function appendToDateSection(
  content: string,
  date: string,
  block: string,
): string {
  const sections = Array.from(content.matchAll(DATE_SECTION_PATTERN));
  const sectionIndex = sections.findIndex((match) => match[1] === date);
  if (sectionIndex < 0) {
    return appendBlock(
      content,
      `<!-- discord-message-date: ${date} -->\n## ${date}\n\n${block}`,
    );
  }

  const nextSection = sections[sectionIndex + 1];
  if (nextSection?.index === undefined) {
    return appendBlock(content, block);
  }

  const before = appendBlock(content.slice(0, nextSection.index), block);
  const after = content.slice(nextSection.index);
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

function getMarkerOffset(content: string): number {
  const bomOffset = content.startsWith("\uFEFF") ? 1 : 0;
  const frontmatter = content
    .slice(bomOffset)
    .match(/^---\r?\n(?:[\s\S]*?\r?\n)?---\r?(?:\n|$)/);
  const contentOffset = bomOffset + (frontmatter?.[0].length ?? 0);
  const blankLines =
    content.slice(contentOffset).match(/^(?:[ \t]*\r?\n)*/)?.[0].length ?? 0;
  return contentOffset + blankLines;
}
