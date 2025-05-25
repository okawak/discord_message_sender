import { defineConfig } from "rolldown";
import wasm from "@rollup/plugin-wasm";
import terser from "@rollup/plugin-terser";

const isProduction = process.env.NODE_ENV === "production";

export default defineConfig({
  input: "src/main.ts",
  output: {
    dir: isProduction ? "dist" : ".",
    format: "cjs", // Obsidian requires CommonJS
    inlineDynamicImports: true,
    sourcemap: !isProduction,
    entryFileNames: "main.js",
  },
  external: ["obsidian", "fs", "path", "crypto", "util", "stream", "events"],
  plugins: [
    {
      name: "replace-import-meta",
      transform(code, _id) {
        if (code.includes("import.meta.url")) {
          // replace import.meta.url with document.baseURI for browser compatibility
          return code.replace(
            /new URL\(([^,]+),\s*import\.meta\.url\)/g,
            'new URL($1, typeof document !== "undefined" ? document.baseURI : "file:///")'
          );
        }
        return null;
      },
    },
    wasm({
      targetEnv: "auto-inline",
    }),
    // Minify only in production
    isProduction &&
      terser({
        format: {
          comments: false,
        },
        compress: {
          drop_console: false, // Keep console logs for debugging
          drop_debugger: true,
        },
        mangle: {
          keep_classnames: true, // Preserve class names for Obsidian
          keep_fnames: true, // Preserve function names for debugging
        },
      }),
  ].filter(Boolean),

  // Performance optimizations
  treeshake: {
    moduleSideEffects: false,
    propertyReadSideEffects: false,
  },
});
