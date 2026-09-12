import { homedir } from "node:os";
import { resolve } from "node:path";
import { $ } from "bun";
import { resolveWasmBindgenCli } from "./wasm-bindgen-cli";

const root = resolve(import.meta.dir, "..");
const cargoHome = resolve(
  process.env.CARGO_HOME ?? resolve(homedir(), ".cargo"),
);
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? "target");
const shell = $.cwd(root);
const wasmBindgen = await resolveWasmBindgenCli(root, target);

// Keep panic locations identical across checkout and Cargo registry paths.
await shell`cargo wasm-build`.env({
  ...process.env,
  CARGO_ENCODED_RUSTFLAGS: [
    `--remap-path-prefix=${cargoHome}=/cargo`,
    `--remap-path-prefix=${root}=/source`,
  ].join("\x1f"),
});
await shell`${wasmBindgen} ${resolve(target, "wasm32-unknown-unknown/release/parse_message.wasm")} --target web --out-dir pkg`;
await shell`wasm-opt pkg/parse_message_bg.wasm -Oz -o pkg/parse_message_bg.wasm`;
