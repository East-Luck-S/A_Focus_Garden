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
import { PlantSvg } from "../components/plant";
import { growthFor, projectedRemaining } from "./growth";

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
  const growth = useMemo(() => session ? growthFor(session, now) : 0, [now, session]);
  const apply = (operation: Promise<FocusSessionSnapshot>) => {
    setError(null);
    void operation.then((next) => {
      // An abandoned session is terminal and no longer belongs on the Focus
      // Screen. Returning to the selector lets the user begin a new intent.
      setSession(next.status === "ABANDONED" ? null : next);
      setNow(next.observedAt);
    }).catch((reason: unknown) => {
      const message = typeof reason === "object" && reason !== null && "message" in reason
        ? String(reason.message)
        : "That focus session action could not be completed.";
      setError(message);
    });
  };

  return (
    <main className="focus-shell">
      <section className="focus-card" aria-labelledby="app-title">
        <p className="eyebrow">A quiet place to grow</p>
        <h1 id="app-title" className="text-4xl font-semibold tracking-tight">
          Focus Garden
        </h1>
        {isUnavailable && <p className="notice" role="alert">Focus Garden cannot reach its local journal.</p>}
        {error && <p className="notice" role="alert">{error}</p>}
        {!session && status?.initialized && (
          <div className="starter">
            <p>Choose a little protected time.</p>
            <div className="duration-options" aria-label="Focus duration">
              {[10, 25, 45].map((minutes) => <button key={minutes} className="duration" onClick={() => apply(startFocusSession(minutes * 60 * 1_000))}>{minutes} min</button>)}
            </div>
          </div>
        )}
        {session && (
          <div className="session-view">
            <p className="eyebrow">{session.status.replaceAll("_", " ")}</p>
            <PlantSvg genome={session.plantGenome} growth={growth} className="live-plant" label={`Your growing plant, ${Math.round(growth * 100)} percent grown`} />
            <output className="timer" aria-label="Remaining focus time">{formatRemaining(remainingMs)}</output>
            {session.status === "ACTIVE" && <div className="actions"><button className="primary" onClick={() => apply(pauseFocusSession(session.id))}>Pause</button><button className="quiet" onClick={() => apply(abandonFocusSession(session.id))}>Abandon</button></div>}
            {session.status === "PAUSED" && <div className="actions"><button className="primary" onClick={() => apply(resumeFocusSession(session.id))}>Resume</button><button className="quiet" onClick={() => apply(abandonFocusSession(session.id))}>Abandon</button></div>}
            {session.status === "RECOVERY_REQUIRED" && <div className="recovery"><p>Your session was interrupted. Choose how to continue.</p><div className="actions"><button className="primary" onClick={() => apply(resolveFocusSessionRecovery(session.id, "CONTINUE"))}>Continue</button><button className="quiet" onClick={() => apply(resolveFocusSessionRecovery(session.id, "ABANDON"))}>Abandon</button></div></div>}
          </div>
        )}
      </section>
    </main>
  );
}
