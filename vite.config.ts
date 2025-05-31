import { defineConfig } from "vite";
import copy from "rollup-plugin-copy";
import fs from "node:fs/promises";
import { resolve } from "node:path";

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
      await fs
        .copyFile(
          resolve("dev/parse_message_bg.wasm"),
          resolve("parse_message_bg.wasm")
        )
        .catch(() => {});
    },
  });

  return {
    build: {
      lib: {
        entry: "src/main.ts",
        formats: ["cjs"], // obsidian requires CommonJS
        fileName: () => "main.js",
      },
      outDir: prod ? "dist" : "dev",
      emptyOutDir: true,
      sourcemap: !prod,
      assetsInlineLimit: 0,
      rollupOptions: {
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
        output: {
          inlineDynamicImports: true,
        },
        plugins: [
          // copy parse_message_bg.wasm to dist or dev
          copy({
            targets: [
              { src: "pkg/parse_message_bg.wasm", dest: prod ? "dist" : "dev" },
            ],
            hook: "writeBundle",
            verbose: !prod,
          }),
        ],
      },
    },
    plugins: [!prod && copyMainToRoot()],
    optimizeDeps: {
      exclude: ["node:fs/promises", "node:path"],
    },
  };
});
