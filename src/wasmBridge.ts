import { Notice } from "obsidian";
import initWasm, {
  type InitOutput,
  process_message as processMessage,
} from "../pkg/parse_message.js";
import { type ProcessedMessage, parseWasmMessageResult } from "./messages";

// flag to indicate if the WASM module is ready
let wasmReady: Promise<InitOutput> | null = null;

export async function initWasmBridge(): Promise<InitOutput> {
  if (!wasmReady) {
    wasmReady = (async () => {
      try {
        return await initWasm();
      } catch (error: unknown) {
        wasmReady = null; // reset on error
        new Notice("WASM initialization failed.");
        throw new Error("WASM initialization failed.", { cause: error });
      }
    })();
  }
  return wasmReady;
}

export async function parseMessageWasm(
  content: string,
  prefix: string,
  timestamp: string,
): Promise<ProcessedMessage> {
  try {
    await initWasmBridge();
    const result: unknown = await processMessage(content, prefix, timestamp);
    return parseWasmMessageResult(result);
  } catch (error) {
    throw new Error("Failed to parse message.", { cause: error });
  }
}
