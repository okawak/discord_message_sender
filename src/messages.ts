export interface DiscordMessage {
  id: string;
  content: string;
  timestamp: string;
  author?: { bot?: boolean };
}

export interface ProcessedMessage {
  markdown: string;
  isClipping: boolean;
  fileName: string;
}

export function parseWasmMessageResult(
  value: unknown,
  timestamp: string,
): ProcessedMessage {
  if (
    !Array.isArray(value) ||
    typeof value[0] !== "string" ||
    typeof value[1] !== "boolean"
  ) {
    throw new TypeError("WASM returned an invalid processed message.");
  }

  return {
    markdown: value[0],
    isClipping: value[1],
    fileName: formatMessageFileName(timestamp),
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
