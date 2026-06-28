import { Notice } from "obsidian";
import initWasm, {
  type InitOutput,
  process_message as processMessage,
} from "../pkg/parse_message.js";
import { type ProcessedMessage, parseWasmMessageResult } from "./messages";

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
  content: string,
  prefix: string,
  timestamp: string,
): Promise<ProcessedMessage> {
  try {
    await initWasmBridge();
    const result: unknown = await processMessage(content, prefix);
    return parseWasmMessageResult(result, timestamp);
  } catch (error) {
    throw new Error("Failed to parse message.", { cause: error });
  }
}
