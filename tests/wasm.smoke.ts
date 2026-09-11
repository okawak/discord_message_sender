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

const resolvedUrls = convertHtml(
  "https://example.com/docs/page?old=1",
  [
    '<a href="next">Next</a>',
    '<a href="?q=x">Query</a>',
    '<img src="//cdn.example.com/image.png" alt="CDN">',
    '<a href="HTTPS://other.example/x">Other</a>',
    '<a href="java&#10;script:alert(1)">Unsafe</a>',
  ].join(""),
);
for (const expectedUrl of [
  "https://example.com/docs/next",
  "https://example.com/docs/page?q=x",
  "https://cdn.example.com/image.png",
  "https://other.example/x",
]) {
  if (!resolvedUrls.includes(expectedUrl)) {
    throw new Error(`WASM URL resolution failed for "${expectedUrl}".`);
  }
}
if (resolvedUrls.includes("javascript:")) {
  throw new Error("WASM URL resolution restored an unsafe scheme.");
}

const inlineLink = convertHtml(
  "https://example.com",
  '<a href="/target"><span>Hello</span><span>World</span></a>',
);
if (!inlineLink.includes("[HelloWorld](https://example.com/target)")) {
  throw new Error("WASM inline link destination was not preserved.");
}

const markdownTable = convertHtml(
  "https://example.com",
  "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>",
);
if (
  !markdownTable.includes("| A | B |\n| --- | --- |\n| 1 | 2 |") ||
  !Bun.markdown.html(markdownTable).includes("<table>")
) {
  throw new Error("WASM HTML table did not produce a valid Markdown table.");
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
