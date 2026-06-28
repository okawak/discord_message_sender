import { resolve } from "node:path";

const VERSION_PATTERN = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/;
const CARGO_VERSION_PATTERN =
  /^(\[workspace\.package\][ \t]*\n(?:^(?!\[).*\n)*?^version[ \t]*=[ \t]*")([^"]+)(".*)$/m;

type VersionTuple = readonly [number, number, number];
type JsonObject = Record<string, unknown>;

export interface ReleaseFileContents {
  packageJson: string;
  manifestJson: string;
  cargoToml: string;
  versionsJson: string;
}

export function parseReleaseVersion(version: string): VersionTuple {
  const match = VERSION_PATTERN.exec(version);
  if (!match) {
    throw new Error(
      `Invalid release version "${version}". Use x.y.z without a v prefix.`,
    );
  }
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

export function prepareReleaseFileContents(
  files: ReleaseFileContents,
  version: string,
): ReleaseFileContents {
  const packageJson = parseJson(files.packageJson, "package.json");
  const manifestJson = parseJson(files.manifestJson, "manifest.json");
  const versionsJson = parseJson(files.versionsJson, "versions.json");
  const cargoMatch = CARGO_VERSION_PATTERN.exec(files.cargoToml);
  if (!cargoMatch) {
    throw new Error("Cargo.toml is missing [workspace.package] version.");
  }

  const packageVersion = readString(packageJson, "version", "package.json");
  const manifestVersion = readString(manifestJson, "version", "manifest.json");
  const cargoVersion = cargoMatch[2] ?? "";
  if (packageVersion !== manifestVersion || packageVersion !== cargoVersion) {
    throw new Error(
      `Source versions do not match: package=${packageVersion}, manifest=${manifestVersion}, cargo=${cargoVersion}.`,
    );
  }

  if (
    compareVersions(
      parseReleaseVersion(version),
      parseReleaseVersion(packageVersion),
    ) < 0
  ) {
    throw new Error(
      `Release version ${version} is older than the current version ${packageVersion}.`,
    );
  }

  packageJson.version = version;
  manifestJson.version = version;
  versionsJson[version] = readString(
    manifestJson,
    "minAppVersion",
    "manifest.json",
  );

  return {
    packageJson: stringify(packageJson),
    manifestJson: stringify(manifestJson),
    cargoToml: files.cargoToml.replace(CARGO_VERSION_PATTERN, `$1${version}$3`),
    versionsJson: stringify(versionsJson),
  };
}

function compareVersions(left: VersionTuple, right: VersionTuple): number {
  return left[0] - right[0] || left[1] - right[1] || left[2] - right[2];
}

function parseJson(contents: string, fileName: string): JsonObject {
  const value: unknown = JSON.parse(contents);
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${fileName} must contain a JSON object.`);
  }
  return value as JsonObject;
}

function readString(object: JsonObject, key: string, fileName: string): string {
  const value = object[key];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${fileName} must contain a non-empty "${key}" string.`);
  }
  return value;
}

function stringify(value: JsonObject): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

async function main(): Promise<void> {
  const version = Bun.argv[2];
  if (!version) {
    throw new Error("Usage: bun run release:prepare <version>");
  }

  const root = resolve(import.meta.dir, "..");
  const path = (name: string) => resolve(root, name);
  const updated = prepareReleaseFileContents(
    {
      packageJson: await Bun.file(path("package.json")).text(),
      manifestJson: await Bun.file(path("manifest.json")).text(),
      cargoToml: await Bun.file(path("Cargo.toml")).text(),
      versionsJson: await Bun.file(path("versions.json")).text(),
    },
    version,
  );
  await Promise.all([
    Bun.write(path("package.json"), updated.packageJson),
    Bun.write(path("manifest.json"), updated.manifestJson),
    Bun.write(path("Cargo.toml"), updated.cargoToml),
    Bun.write(path("versions.json"), updated.versionsJson),
  ]);

  const cargo = Bun.spawn(
    ["cargo", "update", "-p", "html_to_markdown", "-p", "parse_message"],
    { cwd: root, stdout: "inherit", stderr: "inherit" },
  );
  const exitCode = await cargo.exited;
  if (exitCode !== 0) {
    throw new Error(`cargo update failed with exit code ${exitCode}.`);
  }
  console.log(`Prepared release version ${version}.`);
}

if (import.meta.main) {
  try {
    await main();
  } catch (error: unknown) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
