import initWasm, {
  message_instruction as parseMessage,
} from "../pkg/parse_message.js";

await initWasm();

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
