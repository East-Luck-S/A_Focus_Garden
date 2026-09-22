import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { getCurrentFocusSession, getPersistenceStatus } from "../shared/native";

import { App } from "./App";

vi.mock("../shared/native", () => ({
  getPersistenceStatus: vi.fn(),
  getCurrentFocusSession: vi.fn(),
}));

describe("App", () => {
  it("renders the Focus Garden title", () => {
    vi.mocked(getPersistenceStatus).mockResolvedValue({
      initialized: true,
      schemaVersion: 1,
      databaseFileName: "focus-garden.sqlite3",
    });
    vi.mocked(getCurrentFocusSession).mockResolvedValue(null);
    render(<App />);

    expect(
      screen.getByRole("heading", { level: 1, name: "Focus Garden" }),
    ).toBeInTheDocument();
  });

  it("shows the initialized persistence version", async () => {
    vi.mocked(getPersistenceStatus).mockResolvedValue({
      initialized: true,
      schemaVersion: 1,
      databaseFileName: "focus-garden.sqlite3",
    });
    vi.mocked(getCurrentFocusSession).mockResolvedValue(null);
    render(<App />);

    expect(await screen.findByText("Persistence ready · Schema v1")).toBeInTheDocument();
  });
});
