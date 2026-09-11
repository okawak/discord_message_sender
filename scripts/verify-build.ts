import { cp, mkdtemp, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const temporaryRoot = await mkdtemp(join(tmpdir(), "discord-build-"));

try {
  for (const name of [
    ".cargo",
    "src",
    "crates",
    "scripts",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "package.json",
    "bun.lock",
    "manifest.json",
    "vite.config.ts",
    "tsconfig.json",
    ".bun-version",
    ".node-version",
    "LICENSE",
  ]) {
    await cp(join(root, name), join(temporaryRoot, name), { recursive: true });
  }
  // Reuse the frozen JS installation, but rebuild Rust and WASM without target/pkg caches.
  await symlink(
    join(root, "node_modules"),
    join(temporaryRoot, "node_modules"),
    "dir",
  );
  const build = Bun.spawn(["bun", "run", "build"], {
    cwd: temporaryRoot,
    env: { ...process.env, CARGO_TARGET_DIR: join(temporaryRoot, "target") },
    stdout: "inherit",
    stderr: "pipe",
  });
  const [exitCode, stderr] = await Promise.all([
    build.exited,
    new Response(build.stderr).text(),
  ]);
  process.stderr.write(stderr);
  if (exitCode !== 0) {
    throw new Error("The independent release build failed.");
  }
  if (stderr.includes("failed to connect to jobserver")) {
    throw new Error(
      "The Rust build reported a Cargo jobserver connection failure.",
    );
  }
  for (const name of ["main.js", "manifest.json"]) {
    const original = await Bun.file(join(root, "dist", name)).bytes();
    const rebuilt = await Bun.file(join(temporaryRoot, "dist", name)).bytes();
    if (!Buffer.from(original).equals(rebuilt)) {
      throw new Error(`Release artifact is not reproducible: ${name}`);
    }
    console.log(
      `Reproducible: ${name} (SHA-256 ${new Bun.CryptoHasher("sha256").update(original).digest("hex")})`,
    );
  }
} finally {
  await rm(temporaryRoot, { recursive: true, force: true });
}
