import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import { growthFor, projectedRemaining } from "../../app/growth";
import { PlantSvg } from "../../components/plant";
import {
  abandonFocusSession,
  finalizeFocusSession,
  nativeErrorMessage,
  pauseFocusSession,
  resolveFocusSessionRecovery,
  resumeFocusSession,
  startFocusSession,
  type AppSettings,
  type FocusSessionSnapshot,
  type GardenPlantDetail,
} from "../../shared/native";

type Props = {
  session: FocusSessionSnapshot | null;
  settings: AppSettings;
  onSettingsChange: (durationMs: number) => Promise<void>;
  onSessionChange: (session: FocusSessionSnapshot | null) => void;
  onPlanted: (plant: GardenPlantDetail) => void;
  onOpenGarden: () => void;
};

type PendingAction =
  | "settings"
  | "start"
  | "pause"
  | "resume"
  | "recover"
  | "abandon"
  | "finalize";

const presets = [15, 25, 45, 60] as const;
const statusLabel = {
  ACTIVE: "专注中",
  PAUSED: "已暂停",
  RECOVERY_REQUIRED: "需要恢复",
  COMPLETED: "已完成",
  ABANDONED: "已放弃",
} as const;

function formatRemaining(milliseconds: number): string {
  const seconds = Math.ceil(milliseconds / 1_000);
  const minutes = Math.floor(seconds / 60).toString().padStart(2, "0");
  const remainder = (seconds % 60).toString().padStart(2, "0");
  return `${minutes}:${remainder}`;
}
function formatRemainingAccessible(milliseconds: number): string {
  const seconds = Math.ceil(milliseconds / 1_000);
  return `剩余专注时间 ${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒`;
}
function formatDateTime(milliseconds: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(milliseconds));
}
export function FocusScreen({
  session,
  settings,
  onSettingsChange,
  onSessionChange,
  onPlanted,
  onOpenGarden,
}: Props) {
  const [now, setNow] = useState(() => Date.now());
  const [error, setError] = useState<string | null>(null);
  const [abandonError, setAbandonError] = useState<string | null>(null);
  const [durationError, setDurationError] = useState<string | null>(null);
  const [statusMessage, setStatusMessage] = useState("");
  const [pendingAction, setPendingAction] = useState<PendingAction | null>(null);
  const [customMinutes, setCustomMinutes] = useState(
    String(settings.defaultFocusDurationMs / 60_000),
  );
  const [completedPlant, setCompletedPlant] = useState<GardenPlantDetail | null>(null);
  const [confirmAbandon, setConfirmAbandon] = useState(false);
  const [focusTarget, setFocusTarget] = useState<"PRIMARY" | "STARTER" | "COMPLETION" | null>(null);
  const cancelAbandonRef = useRef<HTMLButtonElement>(null);
  const abandonTriggerRef = useRef<HTMLButtonElement>(null);
  const dialogRef = useRef<HTMLElement>(null);
  const durationInputRef = useRef<HTMLInputElement>(null);
  const primaryActionRef = useRef<HTMLButtonElement>(null);
  const completionTitleRef = useRef<HTMLHeadingElement>(null);
  const recoveryTitleRef = useRef<HTMLHeadingElement>(null);
  const announcedEndRef = useRef<string | null>(null);

  useEffect(() => {
    if (session?.status !== "ACTIVE" || projectedRemaining(session, Date.now()) === 0) {
      return;
    }

    const interval = window.setInterval(() => {
      const nextNow = Date.now();
      setNow(nextNow);
      if (projectedRemaining(session, nextNow) === 0) {
        window.clearInterval(interval);
      }
    }, 1_000);
    return () => window.clearInterval(interval);
  }, [session]);

  useEffect(() => {
    if (!confirmAbandon) return;
    if (pendingAction === "abandon") dialogRef.current?.focus();
    else cancelAbandonRef.current?.focus();
  }, [confirmAbandon, pendingAction]);

  const remainingMs = useMemo(
    () => (session ? projectedRemaining(session, now) : 0),
    [now, session],
  );
  const growth = useMemo(
    () => (session ? growthFor(session, now) : 0),
    [now, session],
  );
  const isBusy = pendingAction !== null;

  useEffect(() => {
    if (session?.status === "RECOVERY_REQUIRED") recoveryTitleRef.current?.focus();
  }, [session?.status]);

  useEffect(() => {
    if (isBusy || !focusTarget) return;
    const target = focusTarget === "PRIMARY"
      ? primaryActionRef.current
      : focusTarget === "STARTER"
        ? durationInputRef.current
        : completionTitleRef.current;
    if (target) {
      target.focus();
      setFocusTarget(null);
    }
  }, [completedPlant, focusTarget, isBusy, session?.status]);

  useEffect(() => {
    if (session?.status === "ACTIVE" && remainingMs === 0 && announcedEndRef.current !== session.id) {
      announcedEndRef.current = session.id;
      setStatusMessage("专注时间已结束，可以完成并种下植物。");
    }
  }, [remainingMs, session]);

  const runPending = (action: PendingAction, operation: () => Promise<void>) => {
    if (pendingAction) {
      return;
    }

    setPendingAction(action);
    setError(null);
    if (action === "abandon") setAbandonError(null);
    void operation()
      .catch((reason: unknown) => {
        const message = nativeErrorMessage(reason);
        if (action === "abandon") setAbandonError(message);
        else setError(message);
      })
      .finally(() => setPendingAction(null));
  };

  const applyTransition = (
    action: PendingAction,
    operation: () => Promise<FocusSessionSnapshot>,
  ) => {
    runPending(action, async () => {
      const next = await operation();
      onSessionChange(next.status === "ABANDONED" ? null : next);
      setNow(next.observedAt);
      if (next.status === "ABANDONED") {
        setConfirmAbandon(false);
        setStatusMessage("本次专注已放弃，未生成植物。");
        setFocusTarget("STARTER");
      } else {
        setStatusMessage(next.status === "PAUSED" ? "专注已暂停。" : "专注已继续。");
        setFocusTarget("PRIMARY");
      }
    });
  };

  const validateMinutes = (): number | null => {
    const minutes = Number(customMinutes);
    if (!Number.isInteger(minutes) || minutes < 1 || minutes > 180) {
      setDurationError("请输入 1 到 180 的整数分钟。");
      requestAnimationFrame(() => durationInputRef.current?.focus());
      return null;
    }
    setDurationError(null);
    return minutes;
  };

  const choosePreset = (minutes: number) => {
    setCustomMinutes(String(minutes));
    setDurationError(null);
    runPending("settings", () => onSettingsChange(minutes * 60_000));
  };

  const start = () => {
    const minutes = validateMinutes();
    if (minutes === null) {
      return;
    }

    const durationMs = minutes * 60_000;
    runPending("start", async () => {
      if (settings.defaultFocusDurationMs !== durationMs) {
        await onSettingsChange(durationMs);
      }
      const next = await startFocusSession(durationMs);
      onSessionChange(next);
      setNow(next.observedAt);
      setStatusMessage("专注已开始。");
      setFocusTarget("PRIMARY");
    });
  };

  const finish = () => {
    if (!session) {
      return;
    }

    runPending("finalize", async () => {
      const plant = await finalizeFocusSession(session.id);
      onSessionChange(null);
      onPlanted(plant);
      setCompletedPlant(plant);
      setStatusMessage("专注已完成，植物已经种下。");
      setFocusTarget("COMPLETION");
    });
  };

  const confirmAbandonSession = () => {
    if (!session) {
      return;
    }

    const operation =
      session.status === "RECOVERY_REQUIRED"
        ? () => resolveFocusSessionRecovery(session.id, "ABANDON")
        : () => abandonFocusSession(session.id);
    applyTransition("abandon", operation);
  };

  const closeAbandonDialog = () => {
    if (!isBusy) {
      setConfirmAbandon(false);
      setAbandonError(null);
      abandonTriggerRef.current?.focus();
    }
  };

  const handleDialogKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Tab") {
      const focusable = Array.from(
        event.currentTarget.querySelectorAll<HTMLButtonElement>("button:not(:disabled)"),
      );
      const currentIndex = focusable.indexOf(document.activeElement as HTMLButtonElement);
      if (focusable.length === 0) {
        event.preventDefault();
        dialogRef.current?.focus();
        return;
      }
      event.preventDefault();
      if (currentIndex === -1) {
        (event.shiftKey ? focusable.at(-1) : focusable[0])?.focus();
        return;
      }
      const offset = event.shiftKey ? -1 : 1;
      const nextIndex = (currentIndex + offset + focusable.length) % focusable.length;
      focusable[nextIndex]?.focus();
      return;
    }
    if (event.key === "Escape" && !isBusy) {
      event.preventDefault();
      closeAbandonDialog();
    }
  };

  return (
    <section className="session-view" aria-label="专注">
      {error && (
        <p className="notice" role="alert">
          {error}
        </p>
      )}

      {pendingAction && !confirmAbandon && (
        <p className="busy-status" role="status" aria-live="polite">
          正在处理，请稍候…
        </p>
      )}
      <p className="sr-only" role="status" aria-live="polite">
        {statusMessage}
      </p>

      {!session && !completedPlant && (
        <div className="starter">
          <p className="starter-copy">选择一段安静、不被打扰的专注时间。</p>
          <div className="duration-options" aria-label="专注时长">
            {presets.map((minutes) => (
              <button
                key={minutes}
                className="duration"
                type="button"
                disabled={isBusy}
                aria-pressed={settings.defaultFocusDurationMs === minutes * 60_000}
                onClick={() => choosePreset(minutes)}
              >
                <strong>{minutes}</strong>
                <span>分钟</span>
              </button>
            ))}
          </div>

          <div className="custom-duration">
            <label htmlFor="custom-duration">自定义时长</label>
            <div className="custom-duration-row">
              <input
                ref={durationInputRef}
                id="custom-duration"
                type="number"
                inputMode="numeric"
                min="1"
                max="180"
                step="1"
                value={customMinutes}
                disabled={isBusy}
                aria-describedby={durationError ? "duration-help duration-error" : "duration-help"}
                aria-errormessage={durationError ? "duration-error" : undefined}
                aria-invalid={durationError ? "true" : undefined}
                onChange={(event) => {
                  setCustomMinutes(event.target.value);
                  setDurationError(null);
                }}
              />
              <span>分钟</span>
            </div>
            <p id="duration-help">可设置 1–180 分钟，选择会保存为下次默认时长。</p>
            {durationError ? <p id="duration-error" className="field-error" role="alert">{durationError}</p> : null}
          </div>

          <button ref={primaryActionRef} className="primary start-button" type="button" disabled={isBusy} onClick={start}>
            {pendingAction === "start" ? "正在开始…" : "开始专注"}
          </button>
        </div>
      )}

      {completedPlant && (
        <article className="completion-card" aria-labelledby="completion-title">
          <p className="eyebrow">本次专注完成</p>
          <h2 ref={completionTitleRef} id="completion-title" tabIndex={-1}>今天的专注，已经长成一株植物</h2>
          <PlantSvg
            genome={completedPlant.plantGenome}
            growth={1}
            className="completion-plant"
            label="本次专注种下的植物"
          />
          <dl className="completion-facts">
            <div>
              <dt>计划时长</dt>
              <dd>{Math.round(completedPlant.sourceSession.plannedDurationMs / 60_000)} 分钟</dd>
            </div>
            <div>
              <dt>暂停次数</dt>
              <dd>{completedPlant.sourceSession.pauseCount} 次</dd>
            </div>
            <div>
              <dt>种下时间</dt>
              <dd>{formatDateTime(completedPlant.plantedAt)}</dd>
            </div>
          </dl>
          <div className="actions">
            <button className="primary" type="button" onClick={onOpenGarden}>
              查看花园
            </button>
            <button className="quiet" type="button" onClick={() => {
              setCompletedPlant(null);
              setFocusTarget("STARTER");
            }}>
              再专注一次
            </button>
          </div>
        </article>
      )}

      {session && (
        <div className="active-session">
          <p className="eyebrow">{statusLabel[session.status]}</p>
          <PlantSvg
            genome={session.plantGenome}
            growth={growth}
            className={`live-plant ${session.status === "ACTIVE" ? "live-plant-active" : ""}`}
            label={`正在生长的植物，已生长 ${Math.round(growth * 100)}%`}
          />
          <output className="timer" role="timer" aria-live="off" aria-label={formatRemainingAccessible(remainingMs)}>
            {formatRemaining(remainingMs)}
          </output>

          {session.status === "ACTIVE" && remainingMs > 0 && (
            <div className="actions">
              <button
                ref={primaryActionRef}
                className="primary"
                type="button"
                disabled={isBusy}
                onClick={() =>
                  applyTransition("pause", () => pauseFocusSession(session.id))
                }
              >
                暂停
              </button>
              <button
                className="quiet danger-action"
                type="button"
                disabled={isBusy}
                onClick={(event) => {
                  abandonTriggerRef.current = event.currentTarget;
                  setConfirmAbandon(true);
                }}
              >
                放弃本次专注
              </button>
            </div>
          )}

          {session.status === "ACTIVE" && remainingMs === 0 && (
            <div className="actions">
              <button
                ref={primaryActionRef}
                className="primary"
                type="button"
                disabled={isBusy}
                onClick={finish}
              >
                {pendingAction === "finalize" ? "正在种下…" : "完成专注并种下"}
              </button>
              <button
                className="quiet danger-action"
                type="button"
                disabled={isBusy}
                onClick={(event) => {
                  abandonTriggerRef.current = event.currentTarget;
                  setConfirmAbandon(true);
                }}
              >
                放弃本次专注
              </button>
            </div>
          )}

          {session.status === "PAUSED" && (
            <div className="actions">
              <button
                ref={primaryActionRef}
                className="primary"
                type="button"
                disabled={isBusy}
                onClick={() =>
                  applyTransition("resume", () => resumeFocusSession(session.id))
                }
              >
                继续专注
              </button>
              <button
                className="quiet danger-action"
                type="button"
                disabled={isBusy}
                onClick={(event) => {
                  abandonTriggerRef.current = event.currentTarget;
                  setConfirmAbandon(true);
                }}
              >
                放弃本次专注
              </button>
            </div>
          )}

          {session.status === "RECOVERY_REQUIRED" && (
            <div className="recovery">
              <h2 ref={recoveryTitleRef} tabIndex={-1}>上次专注需要你来决定</h2>
              <p>
                继续会沿用原结束时间；离开期间不会自动奖励植物。如果计时已经到点，
                继续后即可由你确认完成。放弃则不会获得植物。
              </p>
              <div className="actions">
                <button
                  ref={primaryActionRef}
                  className="primary"
                  type="button"
                  disabled={isBusy}
                  onClick={() =>
                    applyTransition("recover", () =>
                      resolveFocusSessionRecovery(session.id, "CONTINUE"),
                    )
                  }
                >
                  继续这次专注
                </button>
                <button
                  className="quiet danger-action"
                  type="button"
                  disabled={isBusy}
                  onClick={(event) => {
                    abandonTriggerRef.current = event.currentTarget;
                    setConfirmAbandon(true);
                  }}
                >
                  放弃本次专注
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {confirmAbandon && session && (
        <div className="dialog-backdrop">
          <section
            ref={dialogRef}
            className="confirm-dialog"
            role="dialog"
            tabIndex={-1}
            aria-modal="true"
            aria-busy={pendingAction === "abandon"}
            aria-labelledby="abandon-title"
            aria-describedby="abandon-description"
            onKeyDown={handleDialogKeyDown}
          >
            <p className="eyebrow">确认放弃</p>
            <h2 id="abandon-title">要放弃这次专注吗？</h2>
            <p id="abandon-description">
              当前进度会保存为“已放弃”，但不会获得植物。这个操作不能恢复。
            </p>
            {pendingAction === "abandon" ? (
              <p className="busy-status" role="status" aria-live="polite">正在放弃，请稍候…</p>
            ) : null}
            {abandonError ? <p className="notice" role="alert">{abandonError}</p> : null}
            <div className="actions">
              <button
                ref={cancelAbandonRef}
                className="primary"
                type="button"
                disabled={isBusy}
                onClick={closeAbandonDialog}
              >
                返回，不放弃
              </button>
              <button
                className="quiet danger-action"
                type="button"
                disabled={isBusy}
                onClick={confirmAbandonSession}
              >
                {pendingAction === "abandon" ? "正在放弃…" : "确认放弃"}
              </button>
            </div>
          </section>
        </div>
      )}
    </section>
  );
}
