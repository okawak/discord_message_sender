export interface DiscordMessage {
  id: string;
  content: string;
  timestamp: string;
  author?: { bot?: boolean };
}

export interface ProcessedMessage {
  messageId: string;
  timestamp: string;
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
  timestamp: string,
  messageId: string,
): ProcessedMessage {
  return {
    messageId,
    timestamp,
    markdown,
    isClipping,
    fileName: `${formatMessageFileName(timestamp)}_${messageId}`,
  };
}

function formatMessageFileName(timestamp: string): string {
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return timestamp;
  }

  const jst = new Date(date.getTime() + 9 * 60 * 60 * 1000).toISOString();
  return `${jst.slice(0, 10).replaceAll("-", "")}_${jst.slice(11, 19).replaceAll(":", "")}`;
}
