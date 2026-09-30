import type { PersistenceStatus } from "./types";
import { invokeNative } from "./invoke";

/** The explicit frontend boundary for persistence-native operations. */
export function getPersistenceStatus(): Promise<PersistenceStatus> {
  return invokeNative<PersistenceStatus>("get_persistence_status");
}
