import { expectTypeOf, test } from "bun:test";
import type * as wasm from "../pkg/parse_message.js";
import type { initWasmCore } from "../src/wasmCore";

// These assertions are checked by tsc against the declarations generated from
// Rust. Only unvalidated host data may remain unknown.
test("generated WASM APIs expose concrete validated values", () => {
  expectTypeOf<Parameters<typeof wasm.normalize_settings>[0]>().toBeUnknown();
  expectTypeOf<Parameters<typeof wasm.migrate_settings>[0]>().toBeUnknown();
  expectTypeOf<
    Parameters<typeof wasm.decode_discord_message>[0]
  >().toBeString();
  expectTypeOf<
    Parameters<typeof wasm.decode_discord_messages>[0]
  >().toBeString();
  expectTypeOf<
    Parameters<typeof wasm.discord_rate_limit_delay>[0]
  >().toBeUnknown();
  expectTypeOf<Parameters<typeof wasm.discord_reset_delay>[0]>().toBeUnknown();
  expectTypeOf<
    Parameters<typeof wasm.discord_retry_decision>[2]
  >().toBeUnknown();
  expectTypeOf<
    Parameters<typeof wasm.normalize_setting_control>[1]
  >().toBeUnknown();
  expectTypeOf<
    ReturnType<typeof wasm.read_setting_control>
  >().toEqualTypeOf<wasm.SettingControlValue>();
  expectTypeOf<
    ReturnType<typeof wasm.normalize_setting_control>
  >().toEqualTypeOf<wasm.SettingControlValue>();
  expectTypeOf<wasm.SettingControlValue>().toEqualTypeOf<
    string | boolean | null
  >();
  expectTypeOf<ReturnType<typeof initWasmCore>>().toEqualTypeOf<
    Promise<void>
  >();
});
