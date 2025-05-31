import type { App, DataAdapter } from "obsidian";
import initWasm, {
  process_message as processMessage,
} from "../pkg/parse_message.js";

const WASM_FILE_NAME = "parse_message_bg.wasm";

// flag to indicate if the WASM module is ready
let wasmReady: Promise<void> | null = null;

export async function initWasmBridge(
  app: App,
  manifestDir: string
): Promise<void> {
  if (wasmReady) return wasmReady;

  wasmReady = (async () => {
    const adapter = app.vault.adapter;
    let bytes: Uint8Array;

    if (isDataAdapter(adapter)) {
      const buf = await adapter.readBinary(`${manifestDir}/${WASM_FILE_NAME}`);
      bytes = new Uint8Array(buf);
    } else {
      const res = await fetch(`${manifestDir}/${WASM_FILE_NAME}`);
      bytes = new Uint8Array(await res.arrayBuffer());
    }
    await initWasm({ module: bytes });
  })();

  return wasmReady;
}

export async function parseMessageWasm(
  app: App,
  manifestDir: string,
  content: string,
  prefix: string,
  timestamp: string
) {
  await initWasmBridge(app, manifestDir);
  return processMessage(content, prefix, timestamp);
}

function isDataAdapter(a: unknown): a is DataAdapter {
  return !!a && typeof (a as DataAdapter).readBinary === "function";
}
