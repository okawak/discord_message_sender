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

export function parseWasmMessageResult(value: unknown): ProcessedMessage {
  if (
    !isRecord(value) ||
    typeof value.md !== "string" ||
    typeof value.is_clip !== "boolean" ||
    typeof value.name !== "string"
  ) {
    throw new TypeError("WASM returned an invalid processed message.");
  }

  return {
    markdown: value.md,
    isClipping: value.is_clip,
    fileName: value.name,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}
