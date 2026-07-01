import { toLocalDateTime } from "./localDateTime";

export interface DiscordMessage {
  id: string;
  content: string;
  timestamp: string;
  author?: {
    id?: string;
    username?: string;
    global_name?: string | null;
    bot?: boolean;
  };
  member?: { nick?: string | null };
}

export interface ProcessedMessage {
  messageId: string;
  timestamp: string;
  authorId: string;
  authorName: string;
  markdown: string;
  isClipping: boolean;
  fileName: string;
}

export type MessageInstruction =
  | { kind: "message"; markdown: string }
  | { kind: "url"; url: string };

export function parseWasmMessageInstruction(
  value: unknown,
): MessageInstruction {
  if (
    !Array.isArray(value) ||
    typeof value[0] !== "string" ||
    typeof value[1] !== "string"
  ) {
    throw new TypeError("WASM returned an invalid message instruction.");
  }

  switch (value[0]) {
    case "message":
      return { kind: "message", markdown: value[1] };
    case "url":
      return { kind: "url", url: value[1] };
    default:
      throw new TypeError(`WASM returned unknown message kind "${value[0]}".`);
  }
}

export function createProcessedMessage(
  markdown: string,
  isClipping: boolean,
  message: DiscordMessage,
  timeZone: string,
): ProcessedMessage {
  return {
    messageId: message.id,
    timestamp: message.timestamp,
    authorId: message.author?.id ?? "",
    authorName:
      message.member?.nick?.trim() ||
      message.author?.global_name?.trim() ||
      message.author?.username?.trim() ||
      message.author?.id ||
      "Unknown",
    markdown,
    isClipping,
    fileName: `${formatMessageFileName(message.timestamp, timeZone)}_${message.id}`,
  };
}

function formatMessageFileName(timestamp: string, timeZone: string): string {
  return toLocalDateTime(timestamp, timeZone).fileTimestamp;
}
