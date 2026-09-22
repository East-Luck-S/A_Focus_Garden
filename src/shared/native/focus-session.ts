import { invoke } from "@tauri-apps/api/core";

import type { FocusSessionSnapshot, GardenPlantDetail, GardenPlantSummary, RecoveryResolution } from "./types";

export function getCurrentFocusSession(): Promise<FocusSessionSnapshot | null> { return invoke<FocusSessionSnapshot | null>("get_current_focus_session"); }
export function startFocusSession(plannedDurationMs: number): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("start_focus_session", { request: { plannedDurationMs } }); }
export function pauseFocusSession(id: string): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("pause_focus_session", { request: { id } }); }
export function resumeFocusSession(id: string): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("resume_focus_session", { request: { id } }); }
export function abandonFocusSession(id: string): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("abandon_focus_session", { request: { id } }); }
export function resolveFocusSessionRecovery(id: string, resolution: RecoveryResolution): Promise<FocusSessionSnapshot> { return invoke<FocusSessionSnapshot>("resolve_focus_session_recovery", { request: { id, resolution } }); }
export function finalizeFocusSession(id: string): Promise<GardenPlantDetail> { return invoke<GardenPlantDetail>("finalize_focus_session", { request: { id } }); }
export function listGardenPlants(): Promise<GardenPlantSummary[]> { return invoke<GardenPlantSummary[]>("list_garden_plants"); }
export function getGardenPlant(id: string): Promise<GardenPlantDetail> { return invoke<GardenPlantDetail>("get_garden_plant", { request: { id } }); }
