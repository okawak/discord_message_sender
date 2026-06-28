import { describe, expect, test } from "bun:test";
import {
  parseReleaseVersion,
  prepareReleaseFileContents,
  type ReleaseFileContents,
} from "../scripts/prepare-release";

const FILES: ReleaseFileContents = {
  packageJson: '{\n  "name": "plugin",\n  "version": "0.2.8"\n}\n',
  manifestJson:
    '{\n  "id": "plugin",\n  "version": "0.2.8",\n  "minAppVersion": "1.8.10"\n}\n',
  cargoToml:
    '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "0.2.8"\nedition = "2024"\n\n[profile.release]\nlto = true\n',
  versionsJson: '{\n  "0.2.0": "1.8.10"\n}\n',
};

describe("parseReleaseVersion", () => {
  test("accepts stable semantic versions without a prefix", () => {
    expect(parseReleaseVersion("0.3.0")).toEqual([0, 3, 0]);
    expect(parseReleaseVersion("10.20.30")).toEqual([10, 20, 30]);
  });

  test("rejects prefixed, incomplete, and prerelease versions", () => {
    expect(() => parseReleaseVersion("v0.3.0")).toThrow();
    expect(() => parseReleaseVersion("0.3")).toThrow();
    expect(() => parseReleaseVersion("0.3.0-beta.1")).toThrow();
  });
});

describe("prepareReleaseFileContents", () => {
  test("updates every version source and adds the Obsidian compatibility entry", () => {
    const updated = prepareReleaseFileContents(FILES, "0.3.0");

    expect(JSON.parse(updated.packageJson)).toMatchObject({ version: "0.3.0" });
    expect(JSON.parse(updated.manifestJson)).toMatchObject({
      version: "0.3.0",
      minAppVersion: "1.8.10",
    });
    expect(updated.cargoToml).toContain(
      '[workspace.package]\nversion = "0.3.0"',
    );
    expect(JSON.parse(updated.versionsJson)).toMatchObject({
      "0.2.0": "1.8.10",
      "0.3.0": "1.8.10",
    });
  });

  test("allows an idempotent retry of the current version", () => {
    const first = prepareReleaseFileContents(FILES, "0.3.0");
    expect(prepareReleaseFileContents(first, "0.3.0")).toEqual(first);
  });

  test("rejects a version older than the current source version", () => {
    expect(() => prepareReleaseFileContents(FILES, "0.2.7")).toThrow(
      "older than the current version",
    );
  });

  test("rejects inconsistent source versions", () => {
    expect(() =>
      prepareReleaseFileContents(
        {
          ...FILES,
          manifestJson: FILES.manifestJson.replace("0.2.8", "0.2.7"),
        },
        "0.3.0",
      ),
    ).toThrow("Source versions do not match");
  });
});
