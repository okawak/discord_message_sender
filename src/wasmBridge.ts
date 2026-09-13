import { Notice, requestUrl } from "obsidian";
import {
  complete_message_instruction,
  type DiscordMessage,
  type MessageInstruction,
  type ProcessedMessageList,
  message_instruction as parseMessage,
} from "../pkg/parse_message.js";
import { convertHtml } from "./htmlConversion";
import { initWasmCore } from "./wasmCore";

export async function initWasmBridge(): Promise<void> {
  try {
    await initWasmCore();
  } catch (error) {
    new Notice("Plugin initialization failed.");
    throw error;
  }
}

export async function parseMessageWasm(
  message: DiscordMessage,
  prefix: string,
  timeZone: string,
  existingClippingIds: ReadonlySet<string> = new Set(),
): Promise<ProcessedMessageList> {
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

  let clippingMarkdown: string | undefined;
  if (instruction.kind === "url") {
    const html = await fetchUrlContent(instruction.url);
    try {
      clippingMarkdown = convertHtml(instruction.url, html);
    } catch (error) {
      throw new Error("Failed to convert URL content to Markdown.", {
        cause: error,
      });
    }
  }

  return complete_message_instruction(
    instruction,
    message,
    clippingMarkdown,
    timeZone,
  );
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
