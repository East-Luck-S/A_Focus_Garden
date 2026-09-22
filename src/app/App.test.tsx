import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  getCurrentFocusSession,
  getPersistenceStatus,
  abandonFocusSession,
  finalizeFocusSession,
  listGardenPlants,
  pauseFocusSession,
  resolveFocusSessionRecovery,
  startFocusSession,
  type FocusSessionSnapshot,
} from "../shared/native";

import { App } from "./App";
import { growthFor, projectedRemaining } from "./growth";

vi.mock("../shared/native", () => ({
  getPersistenceStatus: vi.fn(), getCurrentFocusSession: vi.fn(), startFocusSession: vi.fn(),
  pauseFocusSession: vi.fn(), resumeFocusSession: vi.fn(), abandonFocusSession: vi.fn(),
  resolveFocusSessionRecovery: vi.fn(),
  listGardenPlants: vi.fn(), getGardenPlant: vi.fn(), finalizeFocusSession: vi.fn(),
}));

const session = (status: FocusSessionSnapshot["status"] = "ACTIVE"): FocusSessionSnapshot => ({
  id: "session-id", plannedDurationMs: 60_000, startedAt: 1_000, targetEndAt: 61_000,
  pausedAt: status === "PAUSED" ? 11_000 : null, totalPausedMs: 0, completedAt: null, status,
  pauseCount: 0, createdAt: 1_000, updatedAt: 1_000, remainingMs: status === "PAUSED" ? 50_000 : 60_000,
  observedAt: 1_000, plantSeed: "ffffffffffffffff", generatorVersion: 1,
  plantGenome: {
    generator_version: 1, morphology: "FERN", stem: { height: 70, thickness: 4, curvature: 0, branch_count: 2 }, leaves: [],
    flower: { kind: "BLOOM", petal_count: 5, size: 20, tilt_degrees: 0 },
    palette: { stem: { red: 70, green: 90, blue: 65 }, leaf_primary: { red: 90, green: 140, blue: 80 }, leaf_secondary: { red: 150, green: 180, blue: 100 }, flower_primary: { red: 240, green: 150, blue: 160 }, flower_secondary: { red: 255, green: 220, blue: 140 } },
    effects: { glow: 0, dew_drops: 0, pollen: 0, breeze_offset: 0 },
  },
});

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(1_000);
    vi.mocked(getPersistenceStatus).mockResolvedValue({ initialized: true, schemaVersion: 4, databaseFileName: "focus-garden.sqlite3" });
    vi.mocked(getCurrentFocusSession).mockResolvedValue(null);
    vi.mocked(listGardenPlants).mockResolvedValue([]);
    vi.mocked(startFocusSession).mockResolvedValue(session());
  });
  afterEach(() => { cleanup(); vi.useRealTimers(); });

  it("renders Focus Garden and starts a chosen duration", async () => {
    render(<App />);
    expect(screen.getByRole("heading", { level: 1, name: "Focus Garden" })).toBeInTheDocument();
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    const choice = screen.getByRole("button", { name: "25 min" });
    fireEvent.click(choice);
    expect(startFocusSession).toHaveBeenCalledWith(1_500_000);
  });

  it("uses an interval only to refresh the active projection", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByLabelText("Remaining focus time")).toHaveTextContent("01:00");
    await act(async () => { vi.advanceTimersByTime(1_000); });
    expect(screen.getByLabelText("Remaining focus time")).toHaveTextContent("00:59");
    expect(pauseFocusSession).not.toHaveBeenCalled();
  });

  it("freezes paused and recovery projections while active time follows the wall clock", () => {
    const active = session();
    expect(projectedRemaining(active, 61_000)).toBe(0);
    expect(growthFor(active, 61_000)).toBe(1);
    expect(projectedRemaining(session("PAUSED"), 99_000)).toBe(50_000);
    expect(projectedRemaining(session("RECOVERY_REQUIRED"), 99_000)).toBe(60_000);
  });

  it("returns to duration selection after abandoning an active session", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    vi.mocked(abandonFocusSession).mockResolvedValue(session("ABANDONED"));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    fireEvent.click(screen.getByRole("button", { name: "Abandon" }));
    await act(async () => { await Promise.resolve(); });
    expect(abandonFocusSession).toHaveBeenCalledWith("session-id");
    expect(screen.getByRole("button", { name: "25 min" })).toBeInTheDocument();
  });

  it("returns to duration selection after abandoning a recovery session", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session("RECOVERY_REQUIRED"));
    vi.mocked(resolveFocusSessionRecovery).mockResolvedValue(session("ABANDONED"));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    fireEvent.click(screen.getByRole("button", { name: "Abandon" }));
    await act(async () => { await Promise.resolve(); });
    expect(resolveFocusSessionRecovery).toHaveBeenCalledWith("session-id", "ABANDON");
    expect(screen.getByRole("button", { name: "10 min" })).toBeInTheDocument();
  });

  it("shows an explicit finish action at zero and invokes it only once while pending", async () => {
    const finished = { ...session(), targetEndAt: 1_000, remainingMs: 0 };
    vi.mocked(getCurrentFocusSession).mockResolvedValue(finished);
    vi.mocked(finalizeFocusSession).mockResolvedValue({ id: "session-id", plantedAt: 1_000, plantSeed: "ffffffffffffffff", generatorVersion: 1, plantGenome: finished.plantGenome, sourceSession: { id: "session-id", plannedDurationMs: 60_000, startedAt: 1_000, targetEndAt: 1_000, completedAt: 1_000, totalPausedMs: 0, pauseCount: 0 } });
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    const finish = screen.getByRole("button", { name: "Finish & plant" });
    expect(finalizeFocusSession).not.toHaveBeenCalled();
    fireEvent.click(finish); fireEvent.click(finish);
    await act(async () => { await Promise.resolve(); });
    expect(finalizeFocusSession).toHaveBeenCalledTimes(1);
    expect(screen.getByLabelText("Garden")).toBeInTheDocument();
    expect(screen.getByText("Source session: session-id")).toBeInTheDocument();
  });
});
