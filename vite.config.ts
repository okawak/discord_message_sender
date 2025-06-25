import fs from "node:fs/promises";
import { resolve } from "node:path";
import { wasm } from "@rollup/plugin-wasm";
import { defineConfig } from "vite";

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
      lib: {
        entry: "src/main.ts",
        formats: ["cjs"], // obsidian requires CommonJS
        fileName: () => "main.js",
      },
      outDir: prod ? "dist" : "dev",
      emptyOutDir: true,
      sourcemap: !prod,
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
          wasm({
            targetEnv: "auto-inline",
            maxFileSize: Infinity,
          }),
        ],
      },
    },
    plugins: [!prod && copyMainToRoot()].filter(Boolean),
    optimizeDeps: {
      exclude: ["node:fs/promises", "node:path"],
    },
  };
});
