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
  nativeErrorMessage: (reason: unknown) => reason && typeof reason === "object" && "code" in reason && reason.code === "DATA_SAFETY_BLOCKER"
    ? "已停止以保护本机数据，不会自动重置。请保留当前数据并联系支持人员。"
    : "操作暂时无法完成。请检查应用状态后重试。",
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

  it("renders the Chinese product shell and starts a chosen duration", async () => {
    render(<App />);
    expect(screen.getByText("正在准备专注花园…")).toBeInTheDocument();
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("heading", { level: 1, name: "专注花园" })).toBeInTheDocument();
    const choice = screen.getByRole("button", { name: "25 分钟" });
    fireEvent.click(choice);
    expect(startFocusSession).toHaveBeenCalledWith(1_500_000);
  });

  it("does not expose legacy English controls or raw session enums", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.queryByRole("button", { name: "Pause" })).not.toBeInTheDocument();
    expect(screen.queryByText("ACTIVE")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "暂停" })).toBeInTheDocument();
    expect(screen.getByLabelText("剩余专注时间")).toBeInTheDocument();
  });

  it("uses an interval only to refresh the active projection", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByLabelText("剩余专注时间")).toHaveTextContent("01:00");
    await act(async () => { vi.advanceTimersByTime(1_000); });
    expect(screen.getByLabelText("剩余专注时间")).toHaveTextContent("00:59");
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
    fireEvent.click(screen.getByRole("button", { name: "放弃本次专注" }));
    await act(async () => { await Promise.resolve(); });
    expect(abandonFocusSession).toHaveBeenCalledWith("session-id");
    expect(screen.getByRole("button", { name: "25 分钟" })).toBeInTheDocument();
  });

  it("returns to duration selection after abandoning a recovery session", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session("RECOVERY_REQUIRED"));
    vi.mocked(resolveFocusSessionRecovery).mockResolvedValue(session("ABANDONED"));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    fireEvent.click(screen.getByRole("button", { name: "放弃本次专注" }));
    await act(async () => { await Promise.resolve(); });
    expect(resolveFocusSessionRecovery).toHaveBeenCalledWith("session-id", "ABANDON");
    expect(screen.getByRole("button", { name: "10 分钟" })).toBeInTheDocument();
  });

  it("shows an explicit finish action at zero and invokes it only once while pending", async () => {
    const finished = { ...session(), targetEndAt: 1_000, remainingMs: 0 };
    vi.mocked(getCurrentFocusSession).mockResolvedValue(finished);
    vi.mocked(finalizeFocusSession).mockResolvedValue({ id: "session-id", plantedAt: 1_000, plantSeed: "ffffffffffffffff", generatorVersion: 1, plantGenome: finished.plantGenome, sourceSession: { id: "session-id", plannedDurationMs: 60_000, startedAt: 1_000, targetEndAt: 1_000, completedAt: 1_000, totalPausedMs: 0, pauseCount: 0 } });
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    const finish = screen.getByRole("button", { name: "完成专注并种下" });
    expect(finalizeFocusSession).not.toHaveBeenCalled();
    fireEvent.click(finish); fireEvent.click(finish);
    await act(async () => { await Promise.resolve(); });
    expect(finalizeFocusSession).toHaveBeenCalledTimes(1);
    expect(screen.getByLabelText("花园")).toBeInTheDocument();
    expect(screen.queryByText("session-id")).not.toBeInTheDocument();
  });

  it("shows a Chinese fatal page and retries core startup failures", async () => {
    vi.mocked(getPersistenceStatus).mockRejectedValueOnce({ code: "DATA_SAFETY_BLOCKER", message: "internal" }).mockResolvedValueOnce({ initialized: true, schemaVersion: 4, databaseFileName: "focus-garden.sqlite3" });
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByText("已停止以保护本机数据，不会自动重置。请保留当前数据并联系支持人员。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("button", { name: "25 分钟" })).toBeInTheDocument();
  });

  it("keeps Focus available when the Garden list fails and retries Garden", async () => {
    vi.mocked(listGardenPlants).mockRejectedValueOnce("failed").mockResolvedValueOnce([]);
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("button", { name: "10 分钟" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "花园" }));
    expect(screen.getByRole("button", { name: "重试读取花园" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重试读取花园" }));
    await act(async () => { await Promise.resolve(); });
    expect(screen.getByText("你的花园正在等待第一株完成专注后种下的植物。")).toBeInTheDocument();
  });
});
