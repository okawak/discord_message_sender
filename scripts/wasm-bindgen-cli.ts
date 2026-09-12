import { chmod, mkdir, mkdtemp, rename, rm } from "node:fs/promises";
import { join } from "node:path";

export const WASM_BINDGEN_CLI_VERSION = "0.2.128";

interface WasmBindgenRelease {
  archiveName: string;
  sha256: string;
  targetTriple: string;
}

const RELEASES: Readonly<
  Record<string, Omit<WasmBindgenRelease, "archiveName">>
> = {
  "darwin-arm64": {
    targetTriple: "aarch64-apple-darwin",
    sha256: "67ba17f260977725c0b541b516dbb5153538140f079a900329fb6077661b47ab",
  },
  "darwin-x64": {
    targetTriple: "x86_64-apple-darwin",
    sha256: "59d9af11d0a61b8019898d555de31153c3a50e7f1797e9849fb38589d16add43",
  },
  "linux-arm64": {
    targetTriple: "aarch64-unknown-linux-musl",
    sha256: "079731dd1bc7798c1efa4f08fcc45130827cbcc9ff60a0b4c6047d64fc6fd25c",
  },
  "linux-x64": {
    targetTriple: "x86_64-unknown-linux-musl",
    sha256: "b51f0208fdff83515a787bd8ab9ac5865ed84dabb66d0c709957bb59793c645f",
  },
  "win32-x64": {
    targetTriple: "x86_64-pc-windows-msvc",
    sha256: "8fd8e2165da16b21ee3f5efd19e7f97d8d27cb7832f54edaef4b18e830283ec0",
  },
};

export function getWasmBindgenRelease(
  platform: string,
  arch: string,
): WasmBindgenRelease {
  const release = RELEASES[`${platform}-${arch}`];
  if (!release) {
    throw new Error(
      `No prebuilt wasm-bindgen CLI is configured for ${platform}-${arch}.`,
    );
  }
  return {
    ...release,
    archiveName: `wasm-bindgen-${WASM_BINDGEN_CLI_VERSION}-${release.targetTriple}.tar.gz`,
  };
}

export function getLockedWasmBindgenVersion(lockFile: string): string {
  const match = lockFile.match(
    /\[\[package\]\]\s+name = "wasm-bindgen"\s+version = "([^"]+)"/,
  );
  if (!match?.[1]) {
    throw new Error("Cargo.lock does not contain the wasm-bindgen package.");
  }
  return match[1];
}

async function hasExpectedVersion(executable: string): Promise<boolean> {
  try {
    const version = Bun.spawn([executable, "--version"], {
      stdout: "pipe",
      stderr: "ignore",
    });
    const [exitCode, output] = await Promise.all([
      version.exited,
      new Response(version.stdout).text(),
    ]);
    return (
      exitCode === 0 &&
      output.trim() === `wasm-bindgen ${WASM_BINDGEN_CLI_VERSION}`
    );
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") {
      return false;
    }
    throw error;
  }
}

async function installWasmBindgenCli(
  toolsRoot: string,
  release: WasmBindgenRelease,
): Promise<string> {
  await mkdir(toolsRoot, { recursive: true });
  const installation = await mkdtemp(
    join(toolsRoot, `.wasm-bindgen-${WASM_BINDGEN_CLI_VERSION}-`),
  );
  const archivePath = join(installation, release.archiveName);
  const executableName =
    process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen";
  const executable = join(installation, executableName);
  const url = `https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_CLI_VERSION}/${release.archiveName}`;

  try {
    console.log(`Downloading ${release.archiveName}...`);
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(
        `Failed to download wasm-bindgen CLI: ${response.status} ${response.statusText}`,
      );
    }
    const archive = await response.bytes();
    const digest = new Bun.CryptoHasher("sha256").update(archive).digest("hex");
    if (digest !== release.sha256) {
      throw new Error(
        `Downloaded wasm-bindgen CLI checksum mismatch: expected ${release.sha256}, received ${digest}.`,
      );
    }
    await Bun.write(archivePath, archive);

    const extraction = Bun.spawn(
      ["tar", "-xzf", archivePath, "--strip-components=1", "-C", installation],
      { stdout: "inherit", stderr: "inherit" },
    );
    if ((await extraction.exited) !== 0) {
      throw new Error("Failed to extract the wasm-bindgen CLI archive.");
    }
    await rm(archivePath);
    if (process.platform !== "win32") {
      await chmod(executable, 0o755);
    }
    if (!(await hasExpectedVersion(executable))) {
      throw new Error(
        "The downloaded wasm-bindgen CLI has an unexpected version.",
      );
    }
    return installation;
  } catch (error) {
    await rm(installation, { recursive: true, force: true });
    throw error;
  }
}

export async function resolveWasmBindgenCli(
  root: string,
  targetDirectory: string,
): Promise<string> {
  const lockFile = await Bun.file(join(root, "Cargo.lock")).text();
  const lockedVersion = getLockedWasmBindgenVersion(lockFile);
  if (lockedVersion !== WASM_BINDGEN_CLI_VERSION) {
    throw new Error(
      `Cargo.lock uses wasm-bindgen ${lockedVersion}, but the build CLI is pinned to ${WASM_BINDGEN_CLI_VERSION}.`,
    );
  }

  // Use one verified release binary everywhere. Locally compiled CLIs with the
  // same version can still produce byte-different WASM glue.
  const release = getWasmBindgenRelease(process.platform, process.arch);
  const toolsRoot = join(targetDirectory, "build-tools");
  const installation = join(
    toolsRoot,
    `wasm-bindgen-${WASM_BINDGEN_CLI_VERSION}-${release.targetTriple}`,
  );
  const executable = join(
    installation,
    process.platform === "win32" ? "wasm-bindgen.exe" : "wasm-bindgen",
  );
  if (await hasExpectedVersion(executable)) {
    return executable;
  }

  await rm(installation, { recursive: true, force: true });
  const temporaryInstallation = await installWasmBindgenCli(toolsRoot, release);
  try {
    await rename(temporaryInstallation, installation);
  } catch (error) {
    if (await hasExpectedVersion(executable)) {
      await rm(temporaryInstallation, { recursive: true, force: true });
    } else {
      throw error;
    }
  }
  return executable;
}
