import { invoke } from "@tauri-apps/api/core";

import type { FocusSessionSnapshot, RecoveryResolution } from "./types";

export function getCurrentFocusSession(): Promise<FocusSessionSnapshot | null> { return invoke<FocusSessionSnapshot | null>("get_current_focus_session"); }
export function startFocusSession(plannedDurationMs: number): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("start_focus_session", { request: { plannedDurationMs } }); }
export function pauseFocusSession(id: string): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("pause_focus_session", { request: { id } }); }
export function resumeFocusSession(id: string): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("resume_focus_session", { request: { id } }); }
export function abandonFocusSession(id: string): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("abandon_focus_session", { request: { id } }); }
export function resolveFocusSessionRecovery(id: string, resolution: RecoveryResolution): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("resolve_focus_session_recovery", { request: { id, resolution } }); }
