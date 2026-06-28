import fs from "node:fs/promises";
import { resolve } from "node:path";
import { wasm } from "@rollup/plugin-wasm";
import { defineConfig, type Plugin } from "vite";

const fixWasmImportMetaUrlForCommonJs = (): Plugin => ({
  name: "fix-wasm-import-meta-url-for-commonjs",
  enforce: "post" as const,
  transform(code: string, id: string) {
    if (!id.endsWith("/pkg/parse_message.js")) {
      return null;
    }

    if (!code.includes("data:application/wasm;base64,")) {
      throw new Error("Expected Vite to inline the wasm-bindgen binary.");
    }

    // Vite inlines the WASM URL before this hook. CommonJS output has no
    // import.meta.url, but URL still requires a syntactically valid base.
    const importMetaUrl = "import.meta.url";
    const commonJsBaseUrl = JSON.stringify("file:///").padEnd(
      importMetaUrl.length,
    );
    return {
      code: code.replaceAll(importMetaUrl, commonJsBaseUrl),
      map: this.getCombinedSourcemap(),
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
      lib: {
        entry: "src/main.ts",
        formats: ["cjs"], // obsidian requires CommonJS
        fileName: () => "main.js",
      },
      outDir: prod ? "dist" : "dev",
      emptyOutDir: true,
      sourcemap: !prod,
      codeSplitting: false,
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
        plugins: [
          wasm({
            targetEnv: "auto-inline",
            maxFileSize: Infinity,
          }),
        ],
      },
    },
    plugins: [
      fixWasmImportMetaUrlForCommonJs(),
      !prod && copyMainToRoot(),
    ].filter(Boolean),
    optimizeDeps: {
      exclude: ["node:fs/promises", "node:path"],
    },
  };
});
