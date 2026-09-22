import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { finalizeFocusSession, listGardenPlants, pauseFocusSession, startFocusSession } from "./focus-session";

describe("focus session native boundary", () => {
  it("uses explicit command names and typed request payloads", async () => {
    invoke.mockResolvedValue({ id: "session-id" });
    await startFocusSession(1_500_000);
    await pauseFocusSession("session-id");
    await finalizeFocusSession("session-id");
    await listGardenPlants();
    expect(invoke).toHaveBeenNthCalledWith(1, "start_focus_session", { request: { plannedDurationMs: 1_500_000 } });
    expect(invoke).toHaveBeenNthCalledWith(2, "pause_focus_session", { request: { id: "session-id" } });
    expect(invoke).toHaveBeenNthCalledWith(3, "finalize_focus_session", { request: { id: "session-id" } });
    expect(invoke).toHaveBeenNthCalledWith(4, "list_garden_plants");
  });
});
