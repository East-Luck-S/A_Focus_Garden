import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  getGardenOverview,
  getGardenPlant,
  listGardenPlants,
  type GardenPlantDetail,
  type GardenPlantSummary,
} from "../../shared/native";

import { GardenScreen } from "./GardenScreen";

vi.mock("../../shared/native", () => ({
  getGardenOverview: vi.fn(),
  getGardenPlant: vi.fn(),
  listGardenPlants: vi.fn(),
  nativeErrorMessage: () => "请稍后重试。",
}));

const genome: GardenPlantSummary["plantGenome"] = {
  generator_version: 1,
  morphology: "FERN",
  stem: { height: 70, thickness: 4, curvature: 0, branch_count: 2 },
  leaves: [],
  flower: { kind: "BLOOM", petal_count: 5, size: 20, tilt_degrees: 0 },
  palette: {
    stem: { red: 70, green: 90, blue: 65 },
    leaf_primary: { red: 90, green: 140, blue: 80 },
    leaf_secondary: { red: 150, green: 180, blue: 100 },
    flower_primary: { red: 240, green: 150, blue: 160 },
    flower_secondary: { red: 255, green: 220, blue: 140 },
  },
  effects: { glow: 0, dew_drops: 0, pollen: 0, breeze_offset: 0 },
};

function plant(id: string): GardenPlantSummary {
  return {
    id,
    plantedAt: 1_700_000_000_000,
    generatorVersion: 1,
    plantGenome: genome,
    sourcePlannedDurationMs: 1_500_000,
    sourcePauseCount: 2,
  };
}

function detail(id: string): GardenPlantDetail {
  return {
    ...plant(id),
    plantSeed: "ffffffffffffffff",
    sourceSession: {
      id: "session-id",
      plannedDurationMs: 1_500_000,
      startedAt: 1_699_999_000_000,
      targetEndAt: 1_700_000_000_000,
      completedAt: 1_700_000_000_000,
      totalPausedMs: 0,
      pauseCount: 2,
    },
  };
}

async function settle() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

describe("GardenScreen", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getGardenOverview).mockResolvedValue({
      plantCount: 0,
      totalPlannedDurationMs: 0,
    });
    vi.mocked(listGardenPlants).mockResolvedValue({ items: [], nextCursor: null });
  });

  afterEach(cleanup);

  it("shows an empty garden and overview after its own initial request", async () => {
    render(<GardenScreen refreshKey={0} />);
    await settle();

    expect(listGardenPlants).toHaveBeenCalledWith(null, 24);
    expect(getGardenOverview).toHaveBeenCalledTimes(1);
    expect(screen.getByText("植物总数")).toBeInTheDocument();
    expect(screen.getByText("你的花园正在等待第一株完成专注后种下的植物。")).toBeInTheDocument();
  });

  it("retries an initial error without discarding the ability to view the garden", async () => {
    vi.mocked(listGardenPlants)
      .mockRejectedValueOnce({ code: "DATABASE_OPERATION_FAILURE" })
      .mockResolvedValueOnce({ items: [plant("plant-a")], nextCursor: null });
    vi.mocked(getGardenOverview).mockResolvedValue({
      plantCount: 1,
      totalPlannedDurationMs: 1_500_000,
    });
    render(<GardenScreen refreshKey={0} />);
    await settle();

    fireEvent.click(screen.getByRole("button", { name: "重试读取花园" }));
    await settle();
    expect(screen.getByRole("button", { name: /计划 25 分钟/ })).toBeInTheDocument();
  });

  it("appends later pages without duplicate plant cards", async () => {
    const cursor = { plantedAt: 1_700_000_000_000, id: "plant-a" };
    vi.mocked(listGardenPlants)
      .mockResolvedValueOnce({ items: [plant("plant-a")], nextCursor: cursor })
      .mockResolvedValueOnce({ items: [plant("plant-a"), plant("plant-b")], nextCursor: null });
    vi.mocked(getGardenOverview).mockResolvedValue({
      plantCount: 2,
      totalPlannedDurationMs: 3_000_000,
    });
    render(<GardenScreen refreshKey={0} />);
    await settle();

    fireEvent.click(screen.getByRole("button", { name: "加载更多" }));
    await settle();
    expect(listGardenPlants).toHaveBeenLastCalledWith(cursor, 24);
    expect(screen.getAllByRole("button", { name: /计划 25 分钟/ })).toHaveLength(2);
  });

  it("opens a friendly detail dialog and restores focus after Escape", async () => {
    vi.mocked(listGardenPlants).mockResolvedValue({ items: [plant("plant-a")], nextCursor: null });
    vi.mocked(getGardenOverview).mockResolvedValue({
      plantCount: 1,
      totalPlannedDurationMs: 1_500_000,
    });
    vi.mocked(getGardenPlant).mockResolvedValue(detail("plant-a"));
    render(<GardenScreen refreshKey={0} />);
    await settle();

    const card = screen.getByRole("button", { name: /计划 25 分钟/ });
    fireEvent.click(card);
    await settle();
    const dialog = screen.getByRole("dialog");
    expect(screen.getByRole("button", { name: "关闭详情" })).toHaveFocus();
    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(screen.getByRole("button", { name: "关闭详情" })).toHaveFocus();
    expect(screen.queryByText("session-id")).not.toBeInTheDocument();
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(card).toHaveFocus();
  });
});
