import initWasm, {
  process_message as processMessage,
} from "../pkg/parse_message.js";

await initWasm();
const result: unknown = await processMessage("hello", "!");
if (!Array.isArray(result) || result[0] !== "hello" || result[1] !== false) {
  throw new Error("WASM message processing smoke test failed.");
}
