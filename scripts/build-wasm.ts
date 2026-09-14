import { realpath } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { $ } from "bun";

const root = resolve(import.meta.dir, "..");
const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");

// The first rustc invocation may install the pinned toolchain through rustup.
// Concurrent invocations can race over the same component downloads.
const sysroot = (
  await $`rustc --print sysroot 2>${Bun.stderr}`.cwd(root).text()
).trim();
const rustVersion = await $`rustc --version --verbose 2>${Bun.stderr}`
  .cwd(root)
  .text();
const commit = /^commit-hash: (\w+)$/m.exec(rustVersion)?.[1];
if (!commit) throw new Error("Cannot determine the Rust compiler commit.");

// Panic locations and inlined standard-library code otherwise embed the host's
// Cargo/rustup paths in WASM, even when every tool and dependency is locked.
const mappings = [
  [root, "/workspace"],
  [cargoHome, "/cargo"],
  [join(sysroot, "lib/rustlib/src/rust"), `/rustc/${commit}`],
] as const;
const flags =
  process.env.CARGO_ENCODED_RUSTFLAGS?.split("\x1f") ??
  process.env.RUSTFLAGS?.split(/\s+/).filter(Boolean) ??
  [];
for (const [source, destination] of mappings) {
  for (const path of new Set([
    source,
    await realpath(source).catch(() => source),
  ])) {
    flags.push(`--remap-path-prefix=${path}=${destination}`);
  }
}

await $`wasm-pack build crates/parse_message --release --target web -d ../../pkg -- --locked`
  .cwd(root)
  .env({ ...process.env, CARGO_ENCODED_RUSTFLAGS: flags.join("\x1f") });
