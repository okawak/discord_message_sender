import { defineConfig } from "rolldown";
import wasm from "@rollup/plugin-wasm";
import terser from "@rollup/plugin-terser";

const isProduction = process.env.NODE_ENV === "production";
const isDevelopment = !isProduction;

export default defineConfig({
  input: "src/main.ts",
  output: {
    dir: isProduction ? "dist" : ".",
    format: "cjs", // Obsidian requires CommonJS
    inlineDynamicImports: true,
    sourcemap: isDevelopment,
    entryFilenames: "main.js",
  },
  external: ["obsidian", "fs", "path", "crypto", "util", "stream", "events"],
  plugins: [
    wasm({
      // Inline WASM files in development for easier testing
      targetEnv: isDevelopment ? "auto-inline" : "auto",
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

  // Resolve options
  resolve: {
    preferBuiltins: true,
  },

  // Performance optimizations
  treeshake: {
    moduleSideEffects: false,
    propertyReadSideEffects: false,
  },
});
