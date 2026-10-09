/// <reference types="node" />

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

type TauriConfig = {
  productName: string;
  version: string;
  identifier: string;
  build: {
    beforeDevCommand?: string;
    devUrl?: string;
  };
  app: {
    security: {
      csp: string;
      devCsp?: string;
      freezePrototype: boolean;
      dangerousDisableAssetCspModification: boolean;
      capabilities: string[];
    };
  };
  bundle: {
    resources: Record<string, string>;
    targets: string[];
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
  const developmentConfig = JSON.parse(readFileSync(
    join(process.cwd(), "src-tauri", "tauri.dev.conf.json"),
    "utf8",
  )) as {
    build: { beforeDevCommand: string; devUrl: string };
    app: { security: { devCsp: string } };
  };
  const packageMetadata = JSON.parse(readFileSync(
    join(process.cwd(), "package.json"),
    "utf8",
  )) as {
    name: string;
    version: string;
    scripts: Record<string, string>;
  };
  const cargoManifest = readFileSync(
    join(process.cwd(), "src-tauri", "Cargo.toml"),
    "utf8",
  );

  it("keeps the production CSP local and free of development or inline-script allowances", () => {
    expect(config.app.security.csp).not.toMatch(/http:\/\/localhost:1420|ws:\/\/|unsafe-inline|unsafe-eval|\*/);
    expect(config.app.security.csp).toContain("object-src 'none'");
    expect(config.app.security.csp).toContain("frame-ancestors 'none'");
    expect(config.app.security.devCsp).toBeUndefined();
    expect(config.build.devUrl).toBeUndefined();
    expect(config.build.beforeDevCommand).toBeUndefined();
    expect(developmentConfig.build.devUrl).toBe("http://localhost:1420");
    expect(developmentConfig.app.security.devCsp).toContain("ws://localhost:1420");
  });

  it("enables only the named main-window capability and keeps Tauri CSP hardening active", () => {
    expect(config.app.security.capabilities).toEqual(["default"]);
    expect(config.app.security.freezePrototype).toBe(true);
    expect(config.app.security.dangerousDisableAssetCspModification).toBe(false);
    expect(capability.windows).toEqual(["main"]);
    expect(capability.permissions).toEqual([]);
  });

  it("ships the lockfile-bound third-party notice as a named release resource", () => {
    expect(config.bundle.resources).toEqual({
      "../THIRD_PARTY_NOTICES.txt": "THIRD_PARTY_NOTICES.txt",
    });
  });

  it("keeps product identity and release versions aligned across every package manifest", () => {
    const cargoVersion = cargoManifest.match(
      /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"\s*$/m,
    )?.[1];

    expect(config.productName).toBe("Focus Garden");
    expect(config.identifier).toBe("com.focusgarden.desktop");
    expect(config.bundle.targets).toEqual(["nsis"]);
    expect(config.version).toBe(packageMetadata.version);
    expect(cargoVersion).toBe(packageMetadata.version);
    expect(packageMetadata.scripts["release:build:public"]).toContain("-PublicRelease");
    expect(packageMetadata.scripts["release:build:public"]).toContain("tauri.signing.local.conf.json");
    expect(packageMetadata.scripts["release:verify:public"]).toContain("-PublicRelease");
  });
});
