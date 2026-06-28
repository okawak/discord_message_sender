import { Notice } from "obsidian";
import initWasm, {
  type InitOutput,
  process_message as processMessage,
} from "../pkg/parse_message.js";
import {
  type DiscordMessage,
  type ProcessedMessage,
  parseWasmMessageResult,
} from "./messages";

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
  try {
    await initWasmBridge();
    const result: unknown = await processMessage(message.content, prefix);
    return parseWasmMessageResult(result, message.timestamp, message.id);
  } catch (error) {
    console.error(
      "Failed to parse message; saving the original content:",
      error,
    );
    return parseWasmMessageResult(
      [message.content, false],
      message.timestamp,
      message.id,
    );
  }
}
