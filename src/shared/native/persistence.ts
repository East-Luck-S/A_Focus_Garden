import type { AppSettings, PersistenceStatus } from "./types";
import { invokeNative } from "./invoke";

/** The explicit frontend boundary for persistence-native operations. */
export function getPersistenceStatus(): Promise<PersistenceStatus> {
  return invokeNative<PersistenceStatus>("get_persistence_status");
}
export function getAppSettings(): Promise<AppSettings> { return invokeNative<AppSettings>("get_app_settings"); }
export function updateAppSettings(defaultFocusDurationMs: number): Promise<AppSettings> { return invokeNative<AppSettings>("update_app_settings", { request: { defaultFocusDurationMs } }); }
