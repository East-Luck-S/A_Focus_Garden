import { invoke } from "@tauri-apps/api/core";

import { normalizeNativeError } from "./errors";

export async function invokeNative<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  try {
    return payload === undefined ? await invoke<T>(command) : await invoke<T>(command, payload);
  } catch (reason: unknown) {
    throw normalizeNativeError(reason);
  }
}
