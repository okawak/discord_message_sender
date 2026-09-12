import { describe, expect, test } from "bun:test";
import {
  getLockedWasmBindgenVersion,
  getWasmBindgenRelease,
  WASM_BINDGEN_CLI_VERSION,
} from "../scripts/wasm-bindgen-cli";

describe("wasm-bindgen CLI resolution", () => {
  test("uses the official portable archive for Linux x64", () => {
    expect(getWasmBindgenRelease("linux", "x64")).toEqual({
      archiveName: `wasm-bindgen-${WASM_BINDGEN_CLI_VERSION}-x86_64-unknown-linux-musl.tar.gz`,
      targetTriple: "x86_64-unknown-linux-musl",
      sha256:
        "b51f0208fdff83515a787bd8ab9ac5865ed84dabb66d0c709957bb59793c645f",
    });
  });

  test("supports developer platforms with official release archives", () => {
    expect(getWasmBindgenRelease("darwin", "arm64").targetTriple).toBe(
      "aarch64-apple-darwin",
    );
    expect(getWasmBindgenRelease("darwin", "x64").targetTriple).toBe(
      "x86_64-apple-darwin",
    );
    expect(getWasmBindgenRelease("linux", "arm64").targetTriple).toBe(
      "aarch64-unknown-linux-musl",
    );
    expect(getWasmBindgenRelease("win32", "x64").targetTriple).toBe(
      "x86_64-pc-windows-msvc",
    );
  });

  test("rejects platforms without an official configured archive", () => {
    expect(() => getWasmBindgenRelease("win32", "arm64")).toThrow(
      "No prebuilt wasm-bindgen CLI is configured",
    );
  });

  test("reads the library version without matching related packages", () => {
    expect(
      getLockedWasmBindgenVersion(`
[[package]]
name = "wasm-bindgen-macro"
version = "9.9.9"

[[package]]
name = "wasm-bindgen"
version = "${WASM_BINDGEN_CLI_VERSION}"
`),
    ).toBe(WASM_BINDGEN_CLI_VERSION);
  });

  test("rejects a lock file without wasm-bindgen", () => {
    expect(() =>
      getLockedWasmBindgenVersion('[[package]]\nname = "serde"'),
    ).toThrow("Cargo.lock does not contain the wasm-bindgen package");
  });
});
