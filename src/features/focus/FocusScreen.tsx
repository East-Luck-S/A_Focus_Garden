import { useEffect, useMemo, useState } from "react";
import { PlantSvg } from "../../components/plant";
import { growthFor, projectedRemaining } from "../../app/growth";
import { abandonFocusSession, finalizeFocusSession, nativeErrorMessage, pauseFocusSession, resolveFocusSessionRecovery, resumeFocusSession, startFocusSession, type FocusSessionSnapshot, type GardenPlantDetail } from "../../shared/native";

type Props = { session: FocusSessionSnapshot | null; onSessionChange: (session: FocusSessionSnapshot | null) => void; onPlanted: (plant: GardenPlantDetail) => void };
const statusLabel = { ACTIVE: "专注中", PAUSED: "已暂停", RECOVERY_REQUIRED: "需要恢复", COMPLETED: "已完成", ABANDONED: "已放弃" } as const;
const formatRemaining = (milliseconds: number) => { const seconds = Math.ceil(milliseconds / 1_000); return `${Math.floor(seconds / 60).toString().padStart(2, "0")}:${(seconds % 60).toString().padStart(2, "0")}`; };

export function FocusScreen({ session, onSessionChange, onPlanted }: Props) {
  const [now, setNow] = useState(() => Date.now()); const [error, setError] = useState<string | null>(null); const [isFinalizing, setIsFinalizing] = useState(false);
  useEffect(() => { if (session?.status !== "ACTIVE") return; const interval = window.setInterval(() => setNow(Date.now()), 1_000); return () => window.clearInterval(interval); }, [session?.status]);
  const remainingMs = useMemo(() => session ? projectedRemaining(session, now) : 0, [now, session]); const growth = useMemo(() => session ? growthFor(session, now) : 0, [now, session]);
  const apply = (operation: Promise<FocusSessionSnapshot>) => { setError(null); void operation.then((next) => { onSessionChange(next.status === "ABANDONED" ? null : next); setNow(next.observedAt); }).catch((reason: unknown) => setError(nativeErrorMessage(reason))); };
  const finish = () => { if (!session || isFinalizing) return; setIsFinalizing(true); setError(null); void finalizeFocusSession(session.id).then((plant) => { onSessionChange(null); onPlanted(plant); }).catch((reason: unknown) => setError(nativeErrorMessage(reason))).finally(() => setIsFinalizing(false)); };
  return <section className="session-view" aria-label="专注">{error && <p className="notice" role="alert">{error}</p>}
    {!session && <div className="starter"><p>选择一段安静、不被打扰的专注时间。</p><div className="duration-options" aria-label="专注时长">{[1, 10, 25, 45].map((minutes) => <button key={minutes} className="duration" onClick={() => apply(startFocusSession(minutes * 60_000))}>{minutes} 分钟</button>)}</div></div>}
    {session && <div className="session-view"><p className="eyebrow">{statusLabel[session.status]}</p><PlantSvg genome={session.plantGenome} growth={growth} className="live-plant" label={`正在生长的植物，已生长 ${Math.round(growth * 100)}%`} /><output className="timer" aria-label="剩余专注时间">{formatRemaining(remainingMs)}</output>
      {session.status === "ACTIVE" && remainingMs > 0 && <div className="actions"><button className="primary" onClick={() => apply(pauseFocusSession(session.id))}>暂停</button><button className="quiet" onClick={() => apply(abandonFocusSession(session.id))}>放弃本次专注</button></div>}
      {session.status === "ACTIVE" && remainingMs === 0 && <div className="actions"><button className="primary" disabled={isFinalizing} onClick={finish}>{isFinalizing ? "正在种下…" : "完成专注并种下"}</button><button className="quiet" disabled={isFinalizing} onClick={() => apply(abandonFocusSession(session.id))}>放弃本次专注</button></div>}
      {session.status === "PAUSED" && <div className="actions"><button className="primary" onClick={() => apply(resumeFocusSession(session.id))}>继续专注</button><button className="quiet" onClick={() => apply(abandonFocusSession(session.id))}>放弃本次专注</button></div>}
      {session.status === "RECOVERY_REQUIRED" && <div className="recovery"><p>上次专注被中断。离开期间仍会按原计划计时，请选择如何继续。</p><div className="actions"><button className="primary" onClick={() => apply(resolveFocusSessionRecovery(session.id, "CONTINUE"))}>继续专注</button><button className="quiet" onClick={() => apply(resolveFocusSessionRecovery(session.id, "ABANDON"))}>放弃本次专注</button></div></div>}</div>}
  </section>;
}
