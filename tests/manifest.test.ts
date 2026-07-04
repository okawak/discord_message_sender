import { describe, expect, test } from "bun:test";

interface Manifest {
  description: string;
  minAppVersion: string;
}

const manifest = (await Bun.file(
  new URL("../manifest.json", import.meta.url),
).json()) as Manifest;

describe("manifest", () => {
  test("uses punctuation required by the Obsidian plugin review", () => {
    expect(manifest.description).toMatch(/[.!?]$/);
  });

  test("keeps compatibility with the legacy settings API", () => {
    expect(manifest.minAppVersion).toBe("1.8.10");
  });
});
