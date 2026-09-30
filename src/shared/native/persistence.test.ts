import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { getAppSettings, getPersistenceStatus, updateAppSettings } from "./persistence";

describe("getPersistenceStatus", () => {
  it("invokes the explicit persistence command and returns its typed result", async () => {
    const expected = {
      initialized: true,
      schemaVersion: 1,
      databaseFileName: "focus-garden.sqlite3",
    };
    invoke.mockResolvedValueOnce(expected);

    await expect(getPersistenceStatus()).resolves.toEqual(expected);
    expect(invoke).toHaveBeenCalledWith("get_persistence_status");
  });
});

it("uses explicit app-settings commands and camel-case payload", async () => {
  const expected = { defaultFocusDurationMs: 1_500_000 };
  invoke.mockResolvedValueOnce(expected).mockResolvedValueOnce(expected);
  await expect(getAppSettings()).resolves.toEqual(expected);
  await expect(updateAppSettings(2_700_000)).resolves.toEqual(expected);
  expect(invoke).toHaveBeenNthCalledWith(1, "get_app_settings");
  expect(invoke).toHaveBeenNthCalledWith(2, "update_app_settings", {
    request: { defaultFocusDurationMs: 2_700_000 },
  });
});
