import { describe, expect, it } from "vitest";
import { nativeErrorMessage, normalizeNativeError } from "./errors";

describe("native error normalization", () => {
  it("normalizes object, string, and unknown rejection values", () => {
    const normalized = normalizeNativeError({ code: "SESSION_INVALID_TRANSITION", message: "C:\\private\\raw.sqlite SELECT secret" });
    expect(normalized.code).toBe("SESSION_INVALID_TRANSITION");
    expect(normalized.message).toBe("Native command failed.");
    expect(normalized.message).not.toContain("private");
    expect(normalizeNativeError("native failure").code).toBe("UNKNOWN_NATIVE_ERROR");
    expect(normalizeNativeError(null).code).toBe("UNKNOWN_NATIVE_ERROR");
  });
  it("uses safe Chinese copy for data blockers and unknown failures", () => {
    expect(nativeErrorMessage({ code: "DATA_SAFETY_BLOCKER", message: "unsafe English" })).toContain("已停止以保护本机数据，不会自动重置");
    expect(nativeErrorMessage({ code: "OTHER", message: "unsafe English" })).toBe("操作暂时无法完成。请检查应用状态后重试。");
  });
  it("maps real Rust stable codes instead of enum names", () => {
    expect(nativeErrorMessage({ code: "SESSION_NOT_READY_TO_COMPLETE" })).toContain("尚未结束");
    expect(nativeErrorMessage({ code: "INVALID_FOCUS_DURATION" })).toContain("有效的专注时长");
    expect(nativeErrorMessage({ code: "OPEN_SESSION_EXISTS" })).toContain("未结束的专注");
    expect(nativeErrorMessage({ code: "SESSION_DATA_INVALID" })).toContain("不会自动重置");
    expect(nativeErrorMessage({ code: "PLANT_NOT_FOUND" })).toContain("未找到这株植物");
  });
});
