import { Notice, requestUrl } from "obsidian";
import {
  processed_message as createProcessedMessage,
  type DiscordMessage,
  type InitOutput,
  type MessageInstruction,
  type ProcessedMessage,
  message_instruction as parseMessage,
} from "../pkg/parse_message.js";
import { convertHtml } from "./htmlConversion";
import { initWasmCore } from "./wasmCore";

export async function initWasmBridge(): Promise<InitOutput> {
  try {
    return await initWasmCore();
  } catch (error) {
    new Notice("WASM initialization failed.");
    throw error;
  }
}

export async function parseMessageWasm(
  message: DiscordMessage,
  prefix: string,
  timeZone: string,
  existingClippingIds: ReadonlySet<string> = new Set(),
): Promise<ProcessedMessage | undefined> {
  await initWasmBridge();

  let instruction: MessageInstruction;
  try {
    instruction = parseMessage(
      message.content,
      prefix,
      existingClippingIds.has(message.id),
    );
  } catch (error) {
    throw new Error("Failed to parse Discord message.", { cause: error });
  }

  if (instruction.kind === "message") {
    return createProcessedMessage(
      instruction.markdown,
      false,
      message,
      timeZone,
    );
  }
  if (instruction.kind === "skip") return undefined;

  const html = await fetchUrlContent(instruction.url);
  let markdown: string;
  try {
    markdown = convertHtml(instruction.url, html);
  } catch (error) {
    throw new Error("Failed to convert URL content to Markdown.", {
      cause: error,
    });
  }

  return createProcessedMessage(markdown, true, message, timeZone);
}

async function fetchUrlContent(value: string): Promise<string> {
  let url: URL;
  try {
    url = new URL(value);
  } catch (error) {
    throw new Error("URL command requires a valid absolute URL.", {
      cause: error,
    });
  }

  if (url.protocol !== "https:") {
    throw new Error("Only HTTPS URLs are supported.");
  }

  try {
    const response = await requestUrl({
      url: url.toString(),
      method: "GET",
      headers: { "User-Agent": "Obsidian Discord Sender" },
    });
    return response.text;
  } catch (error) {
    throw new Error("Failed to fetch URL content.", { cause: error });
  }
}
