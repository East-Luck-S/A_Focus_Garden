import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  getCurrentFocusSession,
  getAppSettings,
  updateAppSettings,
  getPersistenceStatus,
  getGardenOverview,
  abandonFocusSession,
  finalizeFocusSession,
  listGardenPlants,
  pauseFocusSession,
  resolveFocusSessionRecovery,
  resumeFocusSession,
  startFocusSession,
  type FocusSessionSnapshot,
} from "../shared/native";

import { App } from "./App";
import { growthFor, projectedRemaining } from "./growth";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, reject, resolve };
}

vi.mock("../shared/native", () => ({
  getPersistenceStatus: vi.fn(), getCurrentFocusSession: vi.fn(), startFocusSession: vi.fn(),
  getAppSettings: vi.fn(), updateAppSettings: vi.fn(),
  pauseFocusSession: vi.fn(), resumeFocusSession: vi.fn(), abandonFocusSession: vi.fn(),
  resolveFocusSessionRecovery: vi.fn(),
  listGardenPlants: vi.fn(), getGardenOverview: vi.fn(), getGardenPlant: vi.fn(), finalizeFocusSession: vi.fn(),
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
    vi.mocked(getPersistenceStatus).mockResolvedValue({ initialized: true, schemaVersion: 5, databaseFileName: "focus-garden.sqlite3" });
    vi.mocked(getAppSettings).mockResolvedValue({ defaultFocusDurationMs: 1_500_000 });
    vi.mocked(updateAppSettings).mockResolvedValue({ defaultFocusDurationMs: 1_500_000 });
    vi.mocked(getCurrentFocusSession).mockResolvedValue(null);
    vi.mocked(listGardenPlants).mockResolvedValue({ items: [], nextCursor: null });
    vi.mocked(getGardenOverview).mockResolvedValue({ plantCount: 0, totalPlannedDurationMs: 0 });
    vi.mocked(startFocusSession).mockResolvedValue(session());
  });
  afterEach(() => { cleanup(); vi.useRealTimers(); });

  it("renders the Chinese product shell and starts a chosen duration", async () => {
    render(<App />);
    expect(screen.getByText("正在安全读取本机数据…")).toBeInTheDocument();
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("heading", { level: 1, name: "专注花园" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "25分钟" })).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(screen.getByRole("button", { name: "45分钟" }));
    await act(async () => { await Promise.resolve(); });
    expect(updateAppSettings).toHaveBeenCalledWith(2_700_000);
  });

  it("opens an accessible Chinese About dialog and restores focus after Escape", async () => {
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });

    const trigger = screen.getByRole("button", { name: "关于" });
    fireEvent.click(trigger);

    const dialog = screen.getByRole("dialog", { name: "关于专注花园" });
    expect(within(dialog).getByText("1.0.0-rc.2")).toBeInTheDocument();
    expect(within(dialog).getByText(/本机 SQLite 数据库/)).toBeInTheDocument();
    expect(within(dialog).getByText(/THIRD_PARTY_NOTICES\.txt/)).toBeInTheDocument();
    const close = within(dialog).getByRole("button", { name: "关闭" });
    expect(close).toHaveFocus();

    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(close).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Tab", shiftKey: true });
    expect(close).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "关于专注花园" })).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("does not expose legacy English controls or raw session enums", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.queryByRole("button", { name: "Pause" })).not.toBeInTheDocument();
    expect(screen.queryByText("ACTIVE")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "暂停" })).toBeInTheDocument();
    expect(screen.getByRole("timer", { name: /剩余专注时间/ })).toHaveAttribute("aria-live", "off");
  });

  it("uses an interval only to refresh the active projection", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("timer", { name: /剩余专注时间/ })).toHaveTextContent("01:00");
    await act(async () => { vi.advanceTimersByTime(1_000); });
    expect(screen.getByRole("timer", { name: /剩余专注时间/ })).toHaveTextContent("00:59");
    expect(pauseFocusSession).not.toHaveBeenCalled();
  });

  it("stops the projection timer at zero without polling native state", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });

    const callsBefore = {
      current: vi.mocked(getCurrentFocusSession).mock.calls.length,
      persistence: vi.mocked(getPersistenceStatus).mock.calls.length,
      settings: vi.mocked(getAppSettings).mock.calls.length,
    };
    expect(vi.getTimerCount()).toBe(1);
    await act(async () => { vi.advanceTimersByTime(60_000); });

    expect(screen.getByRole("timer", { name: "剩余专注时间 0 分 0 秒" })).toHaveTextContent("00:00");
    expect(screen.getByRole("button", { name: "完成专注并种下" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("专注时间已结束，可以完成并种下植物。");
    expect(vi.getTimerCount()).toBe(0);
    await act(async () => { vi.advanceTimersByTime(10_000); });
    expect(vi.mocked(getCurrentFocusSession)).toHaveBeenCalledTimes(callsBefore.current);
    expect(vi.mocked(getPersistenceStatus)).toHaveBeenCalledTimes(callsBefore.persistence);
    expect(vi.mocked(getAppSettings)).toHaveBeenCalledTimes(callsBefore.settings);
    expect(pauseFocusSession).not.toHaveBeenCalled();
    expect(finalizeFocusSession).not.toHaveBeenCalled();
  });

  it("moves focus to the next primary action after pause and resume", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    vi.mocked(pauseFocusSession).mockResolvedValue(session("PAUSED"));
    vi.mocked(resumeFocusSession).mockResolvedValue(session("ACTIVE"));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });

    fireEvent.click(screen.getByRole("button", { name: "暂停" }));
    await act(async () => { await Promise.resolve(); });
    expect(screen.getByRole("button", { name: "继续专注" })).toHaveFocus();
    expect(screen.getByRole("status")).toHaveTextContent("专注已暂停。");

    fireEvent.click(screen.getByRole("button", { name: "继续专注" }));
    await act(async () => { await Promise.resolve(); });
    expect(screen.getByRole("button", { name: "暂停" })).toHaveFocus();
    expect(screen.getByRole("status")).toHaveTextContent("专注已继续。");
  });

  it("guards a pending pause and keeps the active session retryable after failure", async () => {
    const pendingPause = deferred<FocusSessionSnapshot>();
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    vi.mocked(pauseFocusSession).mockReturnValueOnce(pendingPause.promise);
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });

    const pause = screen.getByRole("button", { name: "暂停" });
    fireEvent.click(pause);
    fireEvent.click(pause);
    expect(pauseFocusSession).toHaveBeenCalledTimes(1);
    expect(pause).toBeDisabled();

    await act(async () => {
      pendingPause.reject({ code: "DATABASE_OPERATION_FAILURE" });
      await Promise.resolve();
    });
    expect(screen.getByRole("button", { name: "暂停" })).toBeEnabled();

    vi.mocked(pauseFocusSession).mockResolvedValueOnce(session("PAUSED"));
    fireEvent.click(screen.getByRole("button", { name: "暂停" }));
    await act(async () => { await Promise.resolve(); });
    expect(pauseFocusSession).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("button", { name: "继续专注" })).toBeInTheDocument();
  });

  it("freezes paused and recovery projections while active time follows the wall clock", () => {
    const active = session();
    expect(projectedRemaining(active, 61_000)).toBe(0);
    expect(growthFor(active, 61_000)).toBe(1);
    expect(projectedRemaining(session("PAUSED"), 99_000)).toBe(50_000);
    expect(projectedRemaining(session("RECOVERY_REQUIRED"), 99_000)).toBe(60_000);
  });

  it.each(["PAUSED", "RECOVERY_REQUIRED"] as const)("does not animate or schedule projection ticks while %s", async (status) => {
    const intervalSpy = vi.spyOn(window, "setInterval");
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session(status));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(intervalSpy).not.toHaveBeenCalled();
    expect(screen.getByRole("img", { name: /正在生长的植物/ })).not.toHaveClass("live-plant-active");
    intervalSpy.mockRestore();
  });

  it("returns to duration selection after abandoning an active session", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    vi.mocked(abandonFocusSession).mockResolvedValue(session("ABANDONED"));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    fireEvent.click(screen.getByRole("button", { name: "放弃本次专注" }));
    expect(abandonFocusSession).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "确认放弃" }));
    await act(async () => { await Promise.resolve(); });
    expect(abandonFocusSession).toHaveBeenCalledWith("session-id");
    expect(screen.getByRole("button", { name: "25分钟" })).toBeInTheDocument();
    expect(screen.getByLabelText("自定义时长")).toHaveFocus();
  });

  it("returns to duration selection after abandoning a recovery session", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session("RECOVERY_REQUIRED"));
    vi.mocked(resolveFocusSessionRecovery).mockResolvedValue(session("ABANDONED"));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    fireEvent.click(screen.getByRole("button", { name: "放弃本次专注" }));
    fireEvent.click(screen.getByRole("button", { name: "确认放弃" }));
    await act(async () => { await Promise.resolve(); });
    expect(resolveFocusSessionRecovery).toHaveBeenCalledWith("session-id", "ABANDON");
    expect(screen.getByRole("button", { name: "15分钟" })).toBeInTheDocument();
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
    expect(screen.getByRole("heading", { name: "今天的专注，已经长成一株植物" })).toHaveFocus();
    expect(screen.getByText("计划时长")).toBeInTheDocument();
    expect(screen.getByText("暂停次数")).toBeInTheDocument();
    expect(screen.getByText("种下时间")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "再专注一次" })).toBeInTheDocument();
    expect(screen.queryByText("session-id")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "查看花园" }));
    expect(screen.getByLabelText("花园")).toBeInTheDocument();
  });

  it("shows a Chinese fatal page and retries core startup failures", async () => {
    vi.mocked(getAppSettings)
      .mockRejectedValueOnce({ code: "DATA_SAFETY_BLOCKER", message: "internal" })
      .mockResolvedValueOnce({ defaultFocusDurationMs: 1_500_000 });
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByText("已停止以保护本机数据，不会自动重置。请保留当前数据并联系支持人员。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("button", { name: "25分钟" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "专注花园" })).toHaveFocus();
  });

  it("loads Garden only after navigation, without blocking Focus when the request fails", async () => {
    vi.mocked(listGardenPlants)
      .mockRejectedValueOnce("failed")
      .mockResolvedValueOnce({ items: [], nextCursor: null });
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("button", { name: "15分钟" })).toBeInTheDocument();
    expect(listGardenPlants).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "花园" }));
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByRole("button", { name: "重试读取花园" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重试读取花园" }));
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByText("你的花园正在等待第一株完成专注后种下的植物。")).toBeInTheDocument();
  });

  it("rejects invalid custom minutes without native calls and saves 30 minutes before starting", async () => {
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    const input = screen.getByLabelText("自定义时长");
    for (const value of ["0", "181", "1.5"]) {
      fireEvent.change(input, { target: { value } });
      fireEvent.click(screen.getByRole("button", { name: "开始专注" }));
      expect(updateAppSettings).not.toHaveBeenCalled();
      expect(startFocusSession).not.toHaveBeenCalled();
    }
    fireEvent.change(input, { target: { value: "30" } });
    fireEvent.click(screen.getByRole("button", { name: "开始专注" }));
    await act(async () => { await Promise.resolve(); });
    expect(updateAppSettings).toHaveBeenCalledWith(1_800_000);
    expect(startFocusSession).toHaveBeenCalledWith(1_800_000);
  });

  it("guards a pending start against repeated activation", async () => {
    const pendingStart = deferred<FocusSessionSnapshot>();
    vi.mocked(startFocusSession).mockReturnValueOnce(pendingStart.promise);
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });

    const start = screen.getByRole("button", { name: "开始专注" });
    fireEvent.click(start);
    fireEvent.click(start);
    expect(startFocusSession).toHaveBeenCalledTimes(1);
    expect(start).toBeDisabled();

    await act(async () => {
      pendingStart.resolve(session());
      await Promise.resolve();
    });
    expect(screen.getByRole("button", { name: "暂停" })).toBeInTheDocument();
  });

  it("opens abandon confirmation with focus on return and Escape closes without a native call", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    fireEvent.click(screen.getByRole("button", { name: "放弃本次专注" }));
    const dialog = screen.getByRole("dialog");
    expect(screen.getByRole("button", { name: "返回，不放弃" })).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(screen.getByRole("button", { name: "确认放弃" })).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(screen.getByRole("button", { name: "返回，不放弃" })).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Tab", shiftKey: true });
    expect(screen.getByRole("button", { name: "确认放弃" })).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(abandonFocusSession).not.toHaveBeenCalled();
  });

  it("restores the recovery abandon trigger and traps focus while abandonment is pending", async () => {
    const pendingAbandon = deferred<FocusSessionSnapshot>();
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session("RECOVERY_REQUIRED"));
    vi.mocked(resolveFocusSessionRecovery).mockReturnValueOnce(pendingAbandon.promise);
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });

    const trigger = screen.getByRole("button", { name: "放弃本次专注" });
    fireEvent.click(trigger);
    fireEvent.click(screen.getByRole("button", { name: "返回，不放弃" }));
    expect(trigger).toHaveFocus();

    fireEvent.click(trigger);
    const dialog = screen.getByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "确认放弃" }));
    await act(async () => { await Promise.resolve(); });
    expect(dialog).toHaveAttribute("aria-busy", "true");
    expect(dialog).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(dialog).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Tab", shiftKey: true });
    expect(dialog).toHaveFocus();

    await act(async () => {
      pendingAbandon.resolve(session("ABANDONED"));
      await Promise.resolve();
    });
    expect(screen.getByLabelText("自定义时长")).toHaveFocus();
  });

  it("keeps abandonment failures visible and actionable inside the dialog", async () => {
    const pendingAbandon = deferred<FocusSessionSnapshot>();
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session());
    vi.mocked(abandonFocusSession).mockReturnValueOnce(pendingAbandon.promise);
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });

    fireEvent.click(screen.getByRole("button", { name: "放弃本次专注" }));
    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "确认放弃" }));
    expect(within(dialog).getByRole("status")).toHaveTextContent("正在放弃，请稍候…");

    await act(async () => {
      pendingAbandon.reject({ code: "DATABASE_OPERATION_FAILURE" });
      await Promise.resolve();
    });
    expect(within(dialog).getByRole("alert")).toHaveTextContent("操作暂时无法完成");
    expect(within(dialog).getByRole("button", { name: "返回，不放弃" })).toHaveFocus();
    expect(screen.queryByRole("alert", { hidden: false })).toBe(within(dialog).getByRole("alert"));
  });

  it("associates duration validation with the input and clears it while editing", async () => {
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    const input = screen.getByLabelText("自定义时长");

    fireEvent.change(input, { target: { value: "181" } });
    fireEvent.click(screen.getByRole("button", { name: "开始专注" }));
    await act(async () => { vi.runOnlyPendingTimers(); });
    expect(input).toHaveFocus();
    expect(input).toHaveAttribute("aria-invalid", "true");
    expect(input).toHaveAttribute("aria-errormessage", "duration-error");
    expect(input).toHaveAccessibleDescription(/1–180|1 到 180/);

    fireEvent.change(input, { target: { value: "30" } });
    expect(input).not.toHaveAttribute("aria-invalid");
    expect(screen.queryByText("请输入 1 到 180 的整数分钟。")).not.toBeInTheDocument();
  });

  it("explains recovery consequences before confirmation", async () => {
    vi.mocked(getCurrentFocusSession).mockResolvedValue(session("RECOVERY_REQUIRED"));
    render(<App />);
    await act(async () => { await Promise.resolve(); await Promise.resolve(); });
    expect(screen.getByText(/不会自动奖励植物/)).toBeInTheDocument();
    expect(screen.getByText(/放弃则不会获得植物/)).toBeInTheDocument();
  });
});
