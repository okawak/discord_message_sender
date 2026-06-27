import { resolve } from "node:path";

const VERSION_PATTERN = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/;

type VersionTuple = readonly [number, number, number];

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
  const requestedVersion = parseReleaseVersion(version);
  const packageJson = parseJsonObject(files.packageJson, "package.json");
  const manifestJson = parseJsonObject(files.manifestJson, "manifest.json");
  const versionsJson = parseJsonObject(files.versionsJson, "versions.json");
  const cargoVersion = readCargoWorkspaceVersion(files.cargoToml);
  const currentVersions = [
    readRequiredString(packageJson, "version", "package.json"),
    readRequiredString(manifestJson, "version", "manifest.json"),
    cargoVersion,
  ];

  if (!currentVersions.every((current) => current === currentVersions[0])) {
    throw new Error(
      `Source versions do not match: package=${currentVersions[0]}, manifest=${currentVersions[1]}, cargo=${currentVersions[2]}.`,
    );
  }

  const currentVersion = parseReleaseVersion(currentVersions[0] ?? "");
  if (compareVersions(requestedVersion, currentVersion) < 0) {
    throw new Error(
      `Release version ${version} is older than the current version ${currentVersions[0]}.`,
    );
  }

  const minAppVersion = readRequiredString(
    manifestJson,
    "minAppVersion",
    "manifest.json",
  );
  packageJson.version = version;
  manifestJson.version = version;
  versionsJson[version] = minAppVersion;

  return {
    packageJson: serializeJson(packageJson),
    manifestJson: serializeJson(manifestJson),
    cargoToml: replaceCargoWorkspaceVersion(files.cargoToml, version),
    versionsJson: serializeJson(versionsJson),
  };
}

async function prepareRelease(root: string, version: string): Promise<void> {
  const paths = {
    packageJson: resolve(root, "package.json"),
    manifestJson: resolve(root, "manifest.json"),
    cargoToml: resolve(root, "Cargo.toml"),
    versionsJson: resolve(root, "versions.json"),
  };
  const updated = prepareReleaseFileContents(
    {
      packageJson: await Bun.file(paths.packageJson).text(),
      manifestJson: await Bun.file(paths.manifestJson).text(),
      cargoToml: await Bun.file(paths.cargoToml).text(),
      versionsJson: await Bun.file(paths.versionsJson).text(),
    },
    version,
  );

  await Promise.all([
    Bun.write(paths.packageJson, updated.packageJson),
    Bun.write(paths.manifestJson, updated.manifestJson),
    Bun.write(paths.cargoToml, updated.cargoToml),
    Bun.write(paths.versionsJson, updated.versionsJson),
  ]);

  await updateCargoLock(root);
}

async function updateCargoLock(root: string): Promise<void> {
  const cargoMetadata = Bun.spawn(
    ["cargo", "metadata", "--no-deps", "--format-version", "1"],
    {
      cwd: root,
      stdout: "pipe",
      stderr: "inherit",
    },
  );
  const [metadataText, metadataExitCode] = await Promise.all([
    new Response(cargoMetadata.stdout).text(),
    cargoMetadata.exited,
  ]);
  if (metadataExitCode !== 0) {
    throw new Error(
      `cargo metadata failed with exit code ${metadataExitCode}.`,
    );
  }

  const metadata: unknown = JSON.parse(metadataText);
  const packageNames = readCargoWorkspacePackageNames(metadata);
  const cargoUpdate = Bun.spawn(
    [
      "cargo",
      "update",
      ...packageNames.flatMap((packageName) => ["-p", packageName]),
    ],
    {
      cwd: root,
      stdout: "inherit",
      stderr: "inherit",
    },
  );
  const updateExitCode = await cargoUpdate.exited;
  if (updateExitCode !== 0) {
    throw new Error(`cargo update failed with exit code ${updateExitCode}.`);
  }
}

function compareVersions(left: VersionTuple, right: VersionTuple): number {
  if (left[0] !== right[0]) {
    return left[0] - right[0];
  }
  if (left[1] !== right[1]) {
    return left[1] - right[1];
  }
  return left[2] - right[2];
}

function parseJsonObject(
  contents: string,
  fileName: string,
): Record<string, unknown> {
  const parsed: unknown = JSON.parse(contents);
  if (!isRecord(parsed)) {
    throw new Error(`${fileName} must contain a JSON object.`);
  }
  return parsed;
}

function readRequiredString(
  object: Record<string, unknown>,
  key: string,
  fileName: string,
): string {
  const value = object[key];
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${fileName} must contain a non-empty "${key}" string.`);
  }
  return value;
}

function readCargoWorkspaceVersion(cargoToml: string): string {
  return findCargoWorkspaceVersion(cargoToml).version;
}

function replaceCargoWorkspaceVersion(
  cargoToml: string,
  version: string,
): string {
  const location = findCargoWorkspaceVersion(cargoToml);
  return `${cargoToml.slice(0, location.start)}${location.prefix}${version}${location.suffix}${cargoToml.slice(location.end)}`;
}

function findCargoWorkspaceVersion(cargoToml: string): {
  version: string;
  start: number;
  end: number;
  prefix: string;
  suffix: string;
} {
  const header = /^\[workspace\.package\]\s*$/m.exec(cargoToml);
  if (!header) {
    throw new Error("Cargo.toml is missing [workspace.package].");
  }

  const sectionStart = header.index + header[0].length;
  const remaining = cargoToml.slice(sectionStart);
  const nextHeader = /^\[[^\]]+\]\s*$/m.exec(remaining);
  const sectionEnd =
    nextHeader === null ? cargoToml.length : sectionStart + nextHeader.index;
  const section = cargoToml.slice(sectionStart, sectionEnd);
  const versionLine = /^(\s*version\s*=\s*")([^"]+)(".*)$/m.exec(section);
  if (!versionLine) {
    throw new Error(
      "Cargo.toml [workspace.package] is missing a version string.",
    );
  }

  const prefix = versionLine[1];
  const version = versionLine[2];
  const suffix = versionLine[3];
  if (prefix === undefined || version === undefined || suffix === undefined) {
    throw new Error("Could not parse Cargo.toml workspace version.");
  }

  const start = sectionStart + versionLine.index;
  return {
    version,
    start,
    end: start + versionLine[0].length,
    prefix,
    suffix,
  };
}

function serializeJson(value: Record<string, unknown>): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function readCargoWorkspacePackageNames(metadata: unknown): string[] {
  if (!isRecord(metadata) || !Array.isArray(metadata.packages)) {
    throw new Error("cargo metadata did not return a packages array.");
  }

  const packageNames = metadata.packages.map((packageData) => {
    if (!isRecord(packageData) || typeof packageData.name !== "string") {
      throw new Error("cargo metadata returned a package without a name.");
    }
    return packageData.name;
  });
  if (packageNames.length === 0) {
    throw new Error("cargo metadata returned no workspace packages.");
  }
  return packageNames;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

async function main(): Promise<void> {
  const version = Bun.argv[2];
  if (!version) {
    throw new Error("Usage: bun run release:prepare <version>");
  }

  const root = resolve(import.meta.dir, "..");
  await prepareRelease(root, version);
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
