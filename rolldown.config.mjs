import { defineConfig } from "rolldown";
import terser from "@rollup/plugin-terser";

const prod = process.env.NODE_ENV === "production";

export default defineConfig({
  input: "src/main.ts",
  output: {
    dir: "dist",
    format: "cjs", // Obsidian は CommonJS 読み込み
    inlineDynamicImports: true, // 1ファイル配布
  },
  // Obsidian 自体をバンドルしない
  external: ["obsidian"],
  plugins: [prod && terser({ format: { comments: false } })].filter(Boolean), // false を除外
});
