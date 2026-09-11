import initWasm, {
  convert_html as convertHtml,
  message_instruction as parseMessage,
} from "../pkg/parse_message.js";

const wasm = await initWasm();

const message = parseMessage("hello", "!");
if (message.kind !== "message" || message.markdown !== "hello") {
  throw new Error("WASM message processing smoke test failed.");
}

const url = parseMessage("!url https://example.com", "!");
if (url.kind !== "url" || url.url !== "https://example.com") {
  throw new Error("WASM URL command smoke test failed.");
}

for (const input of ["!url", "!unknown"]) {
  let failed = false;
  try {
    parseMessage(input, "!");
  } catch {
    failed = true;
  }
  if (!failed) {
    throw new Error(`WASM command error did not propagate for "${input}".`);
  }
}

const markdown = convertHtml(
  "https://example.com",
  "<html><head><title>Example</title></head><body><p>Content</p></body></html>",
);
if (!markdown.includes("Example") || !markdown.includes("Content")) {
  throw new Error("WASM HTML conversion smoke test failed.");
}

const repeatedHtml = `<html><body>${"<p>Content</p>".repeat(200)}</body></html>`;
for (let index = 0; index < 100; index += 1) {
  convertHtml("https://example.com", repeatedHtml);
}
const memoryAfterWarmup = wasm.memory.buffer.byteLength;

for (let index = 0; index < 100; index += 1) {
  convertHtml("https://example.com", repeatedHtml);
}
if (wasm.memory.buffer.byteLength > memoryAfterWarmup + 65_536) {
  throw new Error("WASM memory continued to grow after warmup.");
}
