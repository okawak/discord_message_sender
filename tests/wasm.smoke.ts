import { mock } from "bun:test";
import assert from "node:assert/strict";
import initWasm, {
  type DiscordMessage,
  message_instruction as parseMessage,
} from "../pkg/parse_message.js";

await initWasm();

const message = parseMessage("hello", "!", false);
if (message.kind !== "message" || message.markdown !== "hello") {
  throw new Error("WASM message processing smoke test failed.");
}

const url = parseMessage("!url https://example.com", "!", false);
if (url.kind !== "url" || url.url !== "https://example.com") {
  throw new Error("WASM URL command smoke test failed.");
}

for (const input of ["!url", "!unknown"]) {
  let failed = false;
  try {
    parseMessage(input, "!", false);
  } catch {
    failed = true;
  }
  if (!failed) {
    throw new Error(`WASM command error did not propagate for "${input}".`);
  }
}

// Keep the real Rust boundary and mock only Obsidian's I/O/HTML APIs.
const requests: string[] = [];
let failFetch = false;
mock.module("obsidian", () => ({
  Notice: class {
    constructor(message: string) {
      throw new Error(`Unexpected notice: ${message}`);
    }
  },
  async requestUrl(request: { url: string }) {
    requests.push(request.url);
    if (failFetch) throw new Error("Test network failure");
    return { text: "<main><p>Clipped content</p></main>" };
  },
  sanitizeHTMLToDom: () => ({ querySelector: () => null }),
  htmlToMarkdown: () => "Clipped content",
  stringifyYaml: () => "source: https://example.com/article",
}));
const { parseMessageWasm } = await import("../src/wasmBridge");
const source: DiscordMessage = {
  id: "123",
  content: "ordinary message",
  timestamp: "2026-07-01T00:00:00Z",
};
const regular = await parseMessageWasm(source, "!", "UTC");
assert.equal(regular.length, 1);
assert.equal(regular[0]?.markdown, "ordinary message");
assert.equal(regular[0]?.isClipping, false);
assert.deepEqual(
  await parseMessageWasm({ ...source, content: "" }, "!", "UTC"),
  [],
);

const clipping = { ...source, content: "!url https://example.com/article" };
assert.deepEqual(
  await parseMessageWasm(
    { ...clipping, timestamp: "invalid" },
    "!",
    "Invalid/Zone",
    new Set([source.id]),
  ),
  [],
);
await assert.rejects(
  parseMessageWasm({ ...source, content: "!unknown" }, "!", "UTC"),
  /Failed to parse Discord message/,
);
await assert.rejects(
  parseMessageWasm(
    { ...source, content: "!url http://example.com" },
    "!",
    "UTC",
  ),
  /Only HTTPS URLs are supported/,
);
assert.deepEqual(requests, []);

const clipped = await parseMessageWasm(clipping, "!", "UTC");
assert.equal(clipped.length, 1);
assert.equal(clipped[0]?.isClipping, true);
assert.equal(
  clipped[0]?.markdown,
  "---\nsource: https://example.com/article\n---\n\nClipped content",
);
assert.deepEqual(requests, ["https://example.com/article"]);

failFetch = true;
await assert.rejects(
  parseMessageWasm(clipping, "!", "UTC"),
  /Failed to fetch URL content/,
);
console.log(
  "WASM/host boundary: message lists, skipped clippings, HTTPS fetch, and error propagation passed.",
);
