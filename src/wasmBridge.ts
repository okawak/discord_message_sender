import { Notice } from "obsidian";
import initWasm, {
  type InitOutput,
  process_message as processMessage,
} from "../pkg/parse_message.js";

// flag to indicate if the WASM module is ready
let wasmReady: Promise<InitOutput> | null = null;

export async function initWasmBridge(): Promise<InitOutput> {
  if (!wasmReady) {
    try {
      wasmReady = initWasm();
    } catch (error) {
      wasmReady = null; // reset on error
      new Notice("WASM initialization failed.");
      throw new Error(`WASM initialization failed: ${error}`);
    }
  }
  return wasmReady;
}

export async function parseMessageWasm(
  content: string,
  prefix: string,
  timestamp: string,
) {
  try {
    await initWasmBridge();
    return await processMessage(content, prefix, timestamp);
  } catch (error) {
    throw new Error(`Failed to parse message: ${error}`);
  }
}
