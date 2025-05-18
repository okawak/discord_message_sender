import { defineConfig } from "rolldown";
import wasm from "@rollup/plugin-wasm";
import terser from "@rollup/plugin-terser";

const prod = process.env.NODE_ENV === "production";

export default defineConfig({
  input: "src/main.ts",
  output: {
    dir: prod ? "dist" : ".",
    format: "cjs", // Obsidian は CommonJS 読み込み
    inlineDynamicImports: true, // 1ファイル配布
  },
  external: ["obsidian"],
  plugins: [
    wasm({ maxFileSize: 10_000_000 }),
    prod && terser({ format: { comments: false } }),
  ].filter(Boolean), // false を除外
});
