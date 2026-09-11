import { runInNewContext } from "node:vm";
import type { DiscordPluginSettings } from "../pkg/parse_message.js";

interface TestSettingTab {
  getSettingDefinitions(): unknown[];
  setControlValue(key: string, value: unknown): Promise<void>;
}
const tabs: TestSettingTab[] = [];
const saved: unknown[] = [];
const notices: string[] = [];
class HostPlugin {
  manifest = { dir: "test-plugin" };
  app = {};
  async loadData() {
    return {
      channelId: "123",
      lastProcessedMessageId: "456",
      enableAutoSyncOnStartup: false,
    };
  }
  async saveData(value: unknown) {
    saved.push(structuredClone(value));
  }
  addCommand() {}
  addSettingTab(tab: TestSettingTab) {
    tabs.push(tab);
  }
}
interface LoadedPlugin {
  onload(): Promise<void>;
  settings: DiscordPluginSettings;
}
type PluginConstructor = new () => LoadedPlugin;
const module = {
  exports: {} as { default?: PluginConstructor } | PluginConstructor,
};
const obsidian = {
  Plugin: HostPlugin,
  PluginSettingTab: class {
    update() {}
  },
  Notice: class {
    constructor(message: string) {
      notices.push(message);
    }
  },
  requestUrl() {
    throw new Error("Network calls are forbidden in the bundle smoke test.");
  },
};
const bundle = Bun.file(new URL("../dist/main.js", import.meta.url));
if (bundle.size >= 1_000_000)
  throw new Error(`main.js exceeds the size budget: ${bundle.size} bytes.`);
const expectedWasm = await Bun.file(
  new URL("../pkg/parse_message_bg.wasm", import.meta.url),
).bytes();
let instantiations = 0;
const checkedWebAssembly = Object.create(WebAssembly) as typeof WebAssembly;
checkedWebAssembly.instantiate = (async (
  bytes: BufferSource,
  imports: WebAssembly.Imports,
) => {
  const actual = ArrayBuffer.isView(bytes)
    ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    : new Uint8Array(bytes);
  if (!Buffer.from(actual).equals(Buffer.from(expectedWasm)))
    throw new Error("Embedded WASM does not decode to the original binary.");
  instantiations++;
  return WebAssembly.instantiate(bytes, imports);
}) as typeof WebAssembly.instantiate;
runInNewContext(await bundle.text(), {
  module,
  exports: module.exports,
  console,
  TextEncoder,
  TextDecoder,
  WebAssembly: checkedWebAssembly,
  atob(value: string) {
    if (!Buffer.from(value, "base64").equals(Buffer.from(expectedWasm)))
      throw new Error("WASM must be embedded directly, without compression.");
    return atob(value);
  },
  // Loading needs neither Node APIs nor Compression Streams.
  DecompressionStream: undefined,
  URL,
  Request,
  Response,
  require(name: string) {
    if (name !== "obsidian")
      throw new Error(`Unexpected bundle dependency: ${name}`);
    return obsidian;
  },
  fetch() {
    throw new Error("The bundle must initialize without fetching any assets.");
  },
});
const Plugin =
  typeof module.exports === "function"
    ? module.exports
    : module.exports.default;
if (!Plugin) throw new Error("CommonJS plugin export is missing.");
// Constructor must work before the async WASM initialization.
const plugin = new Plugin();
await plugin.onload();
const tab = tabs[0];
if (!tab || tab.getSettingDefinitions().length === 0 || saved.length !== 1)
  throw new Error("Bundle initialization/settings migration failed.");
const channel = plugin.settings.channels[0];
if (channel?.lastProcessedMessageId !== "456")
  throw new Error("Bundle lost the legacy sync cursor.");
await tab.setControlValue("savedNotificationTemplate", "  Saved {count}  ");
if (
  plugin.settings.notificationTemplates.saved !== "Saved {count}" ||
  plugin.settings.channels[0] !== channel
)
  throw new Error(
    "Bundle settings edit broke persistence or channel identity.",
  );
if (instantiations !== 1)
  throw new Error(`Expected one WASM initialization; got ${instantiations}.`);
if (notices.length > 0)
  throw new Error(`Unexpected bundle notice: ${notices.join(", ")}`);
console.log(
  `Production CommonJS bundle: ${bundle.size} bytes; uncompressed offline WASM initialization, settings migration, and persistence passed.`,
);
