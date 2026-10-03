/// <reference types="node" />

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

type TauriConfig = {
  app: {
    security: {
      csp: string;
      devCsp: string;
      freezePrototype: boolean;
      dangerousDisableAssetCspModification: boolean;
      capabilities: string[];
    };
  };
};

type Capability = {
  windows: string[];
  permissions: string[];
};

describe("release security configuration", () => {
  const config = JSON.parse(readFileSync(
    join(process.cwd(), "src-tauri", "tauri.conf.json"),
    "utf8",
  )) as TauriConfig;
  const capability = JSON.parse(readFileSync(
    join(process.cwd(), "src-tauri", "capabilities", "default.json"),
    "utf8",
  )) as Capability;

  it("keeps the production CSP local and free of development or inline-script allowances", () => {
    expect(config.app.security.csp).not.toMatch(/http:\/\/localhost:1420|ws:\/\/|unsafe-inline|unsafe-eval|\*/);
    expect(config.app.security.csp).toContain("object-src 'none'");
    expect(config.app.security.csp).toContain("frame-ancestors 'none'");
    expect(config.app.security.devCsp).toContain("http://localhost:1420");
  });

  it("enables only the named main-window capability and keeps Tauri CSP hardening active", () => {
    expect(config.app.security.capabilities).toEqual(["default"]);
    expect(config.app.security.freezePrototype).toBe(true);
    expect(config.app.security.dangerousDisableAssetCspModification).toBe(false);
    expect(capability.windows).toEqual(["main"]);
    expect(capability.permissions).toEqual([]);
  });
});
