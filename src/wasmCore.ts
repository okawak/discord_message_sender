import initWasm, {
  discord_error_message,
  discord_failure_notice,
} from "../pkg/parse_message.js";

let ready: Promise<void> | undefined;

/** Initialize before calling any domain function; a failed initialization can be retried. */
export function initWasmCore(): Promise<void> {
  ready ??= initWasm().then(
    () => {},
    (error: unknown) => {
      ready = undefined;
      throw new Error("WASM initialization failed.", { cause: error });
    },
  );
  return ready;
}

/** Preserve HTTP status across the Rust-generated error message and JS catch handlers. */
export type DiscordRequestMethod = "GET" | "POST";
export class DiscordApiError extends Error {
  constructor(
    readonly status: number,
    readonly method: DiscordRequestMethod,
    path: string,
    responseText: string,
  ) {
    super(discord_error_message(status, method, path, responseText));
    this.name = "DiscordApiError";
  }
}

export function getDiscordApiFailureNotice(error: DiscordApiError): string {
  return discord_failure_notice(error.status, error.method);
}
