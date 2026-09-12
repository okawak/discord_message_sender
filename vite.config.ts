import fs from "node:fs/promises";
import { resolve } from "node:path";
import { defineConfig, type Plugin } from "vite";

// Embed the original WASM before Vite rewrites wasm-bindgen's default URL.
// No compression, decompression dependency, or asset fetch is needed.
const inlineWasm = (): Plugin => ({
  name: "inline-wasm",
  enforce: "pre",
  async transform(code, id) {
    if (!id.endsWith("/pkg/parse_message.js")) return null;

    const wasmUrl =
      /new URL\(['"]parse_message_bg\.wasm['"], import\.meta\.url\)/g;
    if (Array.from(code.matchAll(wasmUrl)).length !== 1) {
      throw new Error("Expected one wasm-bindgen default WASM URL.");
    }
    const wasmPath = resolve("pkg/parse_message_bg.wasm");
    this.addWatchFile(wasmPath);
    const binary = await fs.readFile(wasmPath);
    const base64 = binary.toString("base64");
    return {
      code: code.replace(
        wasmUrl,
        `Uint8Array.from(atob(${JSON.stringify(base64)}), c => c.charCodeAt(0))`,
      ),
      map: null,
    };
  },
});

export default defineConfig(({ mode }) => {
  // define mode by `vite build --mode production`
  const prod = mode === "production";

  // for dev mode, copy files to root
  const copyMainToRoot = () => ({
    name: "copy-main-to-root",
    closeBundle: async () => {
      await fs.copyFile(resolve("dev/main.js"), resolve("main.js"));
      await fs
        .copyFile(resolve("dev/main.js.map"), resolve("main.js.map"))
        .catch(() => {});
    },
  });

  return {
    build: {
      target: "es2022",
      lib: {
        entry: "src/main.ts",
        formats: ["cjs"], // obsidian requires CommonJS
        fileName: () => "main.js",
      },
      outDir: prod ? "dist" : "dev",
      emptyOutDir: true,
      sourcemap: !prod,
      codeSplitting: false,
      rolldownOptions: {
        external: [
          "obsidian",
          "fs",
          "path",
          "crypto",
          "util",
          "stream",
          "events",
          "node:fs/promises",
          "node:path",
        ],
      },
    },
    plugins: [inlineWasm(), !prod && copyMainToRoot()].filter(Boolean),
    optimizeDeps: {
      exclude: ["node:fs/promises", "node:path"],
    },
  };
});
