export type NativeCommandError = Error & { code: string };

const safetyCodes = new Set([
  "DATA_SAFETY_BLOCKER",
  "UNSUPPORTED_FUTURE_SCHEMA",
  "MIGRATION_HISTORY_INVALID",
  "MIGRATION_FAILED",
  "SESSION_GENERATION_DATA_INVALID",
]);

const localizedCopy: Record<string, string> = {
  DATA_SAFETY_BLOCKER: "已停止以保护本机数据，不会自动重置。请保留当前数据并联系支持人员。",
  UNSUPPORTED_FUTURE_SCHEMA: "本机数据由更新版本创建。已停止以保护本机数据，不会自动重置。请使用较新版本打开。",
  MIGRATION_HISTORY_INVALID: "本机数据的迁移记录不完整。已停止以保护本机数据，不会自动重置。",
  MIGRATION_FAILED: "本机数据升级未完成。已停止以保护本机数据，不会自动重置。",
  SESSION_GENERATION_DATA_INVALID: "这次专注的植物数据无法安全读取。已停止以保护本机数据，不会自动重置。",
  SESSION_DATA_INVALID: "这次专注的存储数据无法安全读取。已停止以保护本机数据，不会自动重置。",
  INVALID_FOCUS_DURATION: "请选择有效的专注时长后重试。",
  GARDEN_QUERY_INVALID: "花园分页请求无效，请重新打开花园后重试。",
  OPEN_SESSION_EXISTS: "已有一项未结束的专注，请先继续、完成或放弃它。",
  SESSION_NOT_FOUND: "未找到这次专注记录。请刷新后重试。",
  SESSION_INVALID_TRANSITION: "当前状态不能执行此操作。请刷新后重试。",
  SESSION_NOT_READY_TO_COMPLETE: "专注时间尚未结束，暂时不能种下植物。",
  PLANT_NOT_FOUND: "未找到这株植物。请返回花园后重试。",
  TIME_VALUE_INVALID: "本机时间数据无效，请检查系统时间后重试。",
  DATABASE_OPERATION_FAILURE: "本机数据暂时无法读取或保存。请重试；数据不会被自动重置。",
  TRANSACTION_FAILURE: "保存操作未完成，请重试；数据不会被自动重置。",
};

export function normalizeNativeError(reason: unknown): NativeCommandError {
  const candidate = typeof reason === "object" && reason !== null ? reason as Record<string, unknown> : null;
  const code = typeof candidate?.code === "string" ? candidate.code : "UNKNOWN_NATIVE_ERROR";
  const error = new Error("Native command failed.") as NativeCommandError;
  error.code = code;
  return error;
}

export function nativeErrorMessage(reason: unknown): string {
  const error = normalizeNativeError(reason);
  if (safetyCodes.has(error.code)) return localizedCopy[error.code] ?? localizedCopy.DATA_SAFETY_BLOCKER!;
  return localizedCopy[error.code] ?? "操作暂时无法完成。请检查应用状态后重试。";
}
