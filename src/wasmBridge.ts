import { Notice } from "obsidian";
import initWasm, {
  convert_html as convertHtml,
  type InitOutput,
  parse_message as parseMessage,
} from "../pkg/parse_message.js";
import {
  createProcessedMessage,
  type DiscordMessage,
  type MessageInstruction,
  type ProcessedMessage,
  parseWasmMessageInstruction,
} from "./messages";
import { fetchUrlContent } from "./urlFetcher";

// flag to indicate if the WASM module is ready
let wasmReady: Promise<InitOutput> | null = null;

export async function initWasmBridge(): Promise<InitOutput> {
  wasmReady ??= initWasm().catch((error: unknown) => {
    wasmReady = null;
    new Notice("WASM initialization failed.");
    throw new Error("WASM initialization failed.", { cause: error });
  });
  return wasmReady;
}

export async function parseMessageWasm(
  message: DiscordMessage,
  prefix: string,
): Promise<ProcessedMessage> {
  await initWasmBridge();

  let instruction: MessageInstruction;
  try {
    const result: unknown = parseMessage(message.content, prefix);
    instruction = parseWasmMessageInstruction(result);
  } catch (error) {
    throw new Error("Failed to parse Discord message.", { cause: error });
  }

  if (instruction.kind === "message") {
    return createProcessedMessage(
      instruction.markdown,
      false,
      message.timestamp,
      message.id,
    );
  }

  const html = await fetchUrlContent(instruction.url);
  let markdown: string;
  try {
    markdown = convertHtml(instruction.url, html);
  } catch (error) {
    throw new Error("Failed to convert URL content to Markdown.", {
      cause: error,
    });
  }

  return createProcessedMessage(markdown, true, message.timestamp, message.id);
}
