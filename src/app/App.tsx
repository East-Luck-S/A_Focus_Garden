import "./app.css";

import { useEffect, useMemo, useState } from "react";

import {
  abandonFocusSession,
  getCurrentFocusSession,
  getPersistenceStatus,
  pauseFocusSession,
  resolveFocusSessionRecovery,
  resumeFocusSession,
  startFocusSession,
  type FocusSessionSnapshot,
  type PersistenceStatus,
} from "../shared/native";

function projectedRemaining(session: FocusSessionSnapshot, now: number): number {
  if (session.status === "ACTIVE") return Math.max(0, session.targetEndAt - now);
  if (session.status === "PAUSED" && session.pausedAt !== null) return Math.max(0, session.targetEndAt - session.pausedAt);
  return session.remainingMs;
}

function formatRemaining(remainingMs: number): string {
  const seconds = Math.ceil(remainingMs / 1_000);
  return `${Math.floor(seconds / 60).toString().padStart(2, "0")}:${(seconds % 60).toString().padStart(2, "0")}`;
}

export function App() {
  const [status, setStatus] = useState<PersistenceStatus | null>(null);
  const [session, setSession] = useState<FocusSessionSnapshot | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [isUnavailable, setIsUnavailable] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    void Promise.all([getPersistenceStatus(), getCurrentFocusSession()])
      .then(([nextStatus, nextSession]) => {
        if (isMounted) {
          setStatus(nextStatus);
          setSession(nextSession);
        }
      })
      .catch(() => {
        if (isMounted) {
          setIsUnavailable(true);
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    if (session?.status !== "ACTIVE") return;
    const interval = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(interval);
  }, [session?.status]);

  const remainingMs = useMemo(() => session ? projectedRemaining(session, now) : 0, [now, session]);
  const apply = (operation: Promise<FocusSessionSnapshot>) => {
    setError(null);
    void operation.then(setSession).catch(() => setError("That focus session action could not be completed."));
  };

  return (
    <main className="grid min-h-screen place-items-center bg-stone-950 px-6 text-stone-100">
      <section className="w-full max-w-md rounded-3xl border border-stone-800 bg-stone-900/70 p-8 text-center shadow-2xl" aria-labelledby="app-title">
        <p className="mb-3 text-xs font-semibold uppercase tracking-[0.28em] text-emerald-400">
          Windows desktop application
        </p>
        <h1 id="app-title" className="text-4xl font-semibold tracking-tight">
          Focus Garden
        </h1>
        <p className="mt-4 text-base text-stone-400" role="status">
          {status?.initialized
            ? `Persistence ready · Schema v${status.schemaVersion}`
            : isUnavailable
              ? "Persistence unavailable."
              : "Initializing persistence…"}
        </p>
        {error && <p className="mt-4 text-sm text-rose-300" role="alert">{error}</p>}
        {!session && status?.initialized && (
          <div className="mt-10 space-y-4">
            <p className="text-stone-300">Choose a calm stretch of time.</p>
            <button className="rounded-full bg-emerald-500 px-6 py-3 font-semibold text-stone-950" onClick={() => apply(startFocusSession(25 * 60 * 1_000))}>Start 25 minute focus</button>
          </div>
        )}
        {session && (
          <div className="mt-10 space-y-6">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-emerald-300">{session.status.replaceAll("_", " ")}</p>
            <output className="block text-6xl font-light tabular-nums tracking-tight" aria-label="Remaining focus time">{formatRemaining(remainingMs)}</output>
            {session.status === "ACTIVE" && <div className="flex justify-center gap-3"><button className="rounded-full bg-amber-200 px-5 py-2 font-semibold text-stone-950" onClick={() => apply(pauseFocusSession(session.id))}>Pause</button><button className="rounded-full border border-stone-600 px-5 py-2 text-stone-200" onClick={() => apply(abandonFocusSession(session.id))}>Abandon</button></div>}
            {session.status === "PAUSED" && <div className="flex justify-center gap-3"><button className="rounded-full bg-emerald-500 px-5 py-2 font-semibold text-stone-950" onClick={() => apply(resumeFocusSession(session.id))}>Resume</button><button className="rounded-full border border-stone-600 px-5 py-2 text-stone-200" onClick={() => apply(abandonFocusSession(session.id))}>Abandon</button></div>}
            {session.status === "RECOVERY_REQUIRED" && <div className="space-y-3"><p className="text-sm text-stone-300">This focus session needs your decision after restart.</p><div className="flex justify-center gap-3"><button className="rounded-full bg-emerald-500 px-5 py-2 font-semibold text-stone-950" onClick={() => apply(resolveFocusSessionRecovery(session.id, "CONTINUE"))}>Continue</button><button className="rounded-full border border-stone-600 px-5 py-2 text-stone-200" onClick={() => apply(resolveFocusSessionRecovery(session.id, "ABANDON"))}>Abandon</button></div></div>}
          </div>
        )}
      </section>
    </main>
  );
}
