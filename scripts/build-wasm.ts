import { realpath } from "node:fs/promises";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");

async function output(command: string[]): Promise<string> {
  const child = Bun.spawn(command, {
    cwd: root,
    stdout: "pipe",
    stderr: "inherit",
  });
  const [status, stdout] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
  ]);
  if (status !== 0) throw new Error(`${command.join(" ")} failed (${status}).`);
  return stdout.trim();
}

const [sysroot, rustVersion] = await Promise.all([
  output(["rustc", "--print", "sysroot"]),
  output(["rustc", "--version", "--verbose"]),
]);
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

const build = Bun.spawn(
  [
    "wasm-pack",
    "build",
    "crates/parse_message",
    "--release",
    "--target",
    "web",
    "-d",
    "../../pkg",
    "--",
    "--locked",
  ],
  {
    cwd: root,
    env: {
      ...process.env,
      CARGO_ENCODED_RUSTFLAGS: flags.join("\x1f"),
    },
    stdout: "inherit",
    stderr: "inherit",
  },
);
if ((await build.exited) !== 0) throw new Error("WASM build failed.");
