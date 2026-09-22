import { invoke } from "@tauri-apps/api/core";

import type { PersistenceStatus } from "./types";

/** The explicit frontend boundary for persistence-native operations. */
export function getPersistenceStatus(): Promise<PersistenceStatus> {
  return invoke<PersistenceStatus>("get_persistence_status");
}
