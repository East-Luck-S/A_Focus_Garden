import type { FocusSessionSnapshot, GardenCursor, GardenOverview, GardenPlantDetail, GardenPlantPage, RecoveryResolution } from "./types";
import { invokeNative } from "./invoke";

export function getCurrentFocusSession(): Promise<FocusSessionSnapshot | null> { return invokeNative("get_current_focus_session"); }
export function startFocusSession(plannedDurationMs: number): Promise<FocusSessionSnapshot> { return invokeNative("start_focus_session", { request: { plannedDurationMs } }); }
export function pauseFocusSession(id: string): Promise<FocusSessionSnapshot> { return invokeNative("pause_focus_session", { request: { id } }); }
export function resumeFocusSession(id: string): Promise<FocusSessionSnapshot> { return invokeNative("resume_focus_session", { request: { id } }); }
export function abandonFocusSession(id: string): Promise<FocusSessionSnapshot> { return invokeNative("abandon_focus_session", { request: { id } }); }
export function resolveFocusSessionRecovery(id: string, resolution: RecoveryResolution): Promise<FocusSessionSnapshot> { return invokeNative("resolve_focus_session_recovery", { request: { id, resolution } }); }
export function finalizeFocusSession(id: string): Promise<GardenPlantDetail> { return invokeNative("finalize_focus_session", { request: { id } }); }
export function listGardenPlants(cursor: GardenCursor | null, limit = 24): Promise<GardenPlantPage> { return invokeNative("list_garden_plants", { request: { cursor, limit } }); }
export function getGardenOverview(): Promise<GardenOverview> { return invokeNative("get_garden_overview"); }
export function getGardenPlant(id: string): Promise<GardenPlantDetail> { return invokeNative("get_garden_plant", { request: { id } }); }
