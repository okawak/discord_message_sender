import initWasm, {
  convert_html as convertHtml,
  parse_message as parseMessage,
} from "../pkg/parse_message.js";

const wasm = await initWasm();

const message: unknown = parseMessage("hello", "!");
if (
  !Array.isArray(message) ||
  message[0] !== "message" ||
  message[1] !== "hello"
) {
  throw new Error("WASM message processing smoke test failed.");
}

const url: unknown = parseMessage("!url https://example.com", "!");
if (
  !Array.isArray(url) ||
  url[0] !== "url" ||
  url[1] !== "https://example.com"
) {
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
