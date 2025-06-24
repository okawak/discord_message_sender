import { type App, Notice } from "obsidian";
import initWasm, {
  process_message as processMessage,
} from "../pkg/parse_message.js";

const WASM_FILE_NAME = "parse_message_bg.wasm";

// flag to indicate if the WASM module is ready
let wasmReady: Promise<void> | null = null;

export async function initWasmBridge(
  app: App,
  manifestDir: string,
): Promise<void> {
  if (wasmReady) return wasmReady;

  wasmReady = (async () => {
    const adapter = app.vault.adapter;
    let bytes: Uint8Array;

    try {
      const buf = await adapter.readBinary(`${manifestDir}/${WASM_FILE_NAME}`);
      bytes = new Uint8Array(buf);
      await initWasm({ module: bytes });
    } catch (error) {
      wasmReady = null; // reset on error
      new Notice("WASM initialization failed.");
      throw new Error(`WASM initialization failed: ${error}`);
    }
  })();

  return wasmReady;
}

export async function parseMessageWasm(
  app: App,
  manifestDir: string,
  content: string,
  prefix: string,
  timestamp: string,
) {
  try {
    await initWasmBridge(app, manifestDir);
    return await processMessage(content, prefix, timestamp);
  } catch (error) {
    throw new Error(`Failed to parse message: ${error}`);
  }
}
