import type { FocusSessionSnapshot } from "../shared/native";

/** Pure display projection; it never changes the authoritative session. */
export function projectedRemaining(session: FocusSessionSnapshot, now: number): number {
  if (session.status === "ACTIVE") return Math.max(0, session.targetEndAt - now);
  if (session.status === "PAUSED" && session.pausedAt !== null) return Math.max(0, session.targetEndAt - session.pausedAt);
  return session.remainingMs;
}

export function growthFor(session: FocusSessionSnapshot, now: number): number {
  const remaining = projectedRemaining(session, now);
  return Math.min(1, Math.max(0, 1 - remaining / session.plannedDurationMs));
}
