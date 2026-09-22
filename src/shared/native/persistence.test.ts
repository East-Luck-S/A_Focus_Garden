import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { getPersistenceStatus } from "./persistence";

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
