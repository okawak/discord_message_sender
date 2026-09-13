import { describe, expect, test } from "bun:test";
import manifest from "../manifest.json";

describe("manifest", () => {
  test("uses punctuation required by the Obsidian plugin review", () => {
    expect(manifest.description).toMatch(/[.!?]$/);
  });

  test("requires the declarative settings API", () => {
    expect(manifest.minAppVersion).toBe("1.13.0");
  });
});
