import "./app.css";

import { useEffect, useMemo, useState } from "react";

import {
  abandonFocusSession,
  finalizeFocusSession,
  getCurrentFocusSession,
  getGardenPlant,
  getPersistenceStatus,
  listGardenPlants,
  pauseFocusSession,
  resolveFocusSessionRecovery,
  resumeFocusSession,
  startFocusSession,
  type FocusSessionSnapshot,
  type GardenPlantDetail,
  type GardenPlantSummary,
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
  const [view, setView] = useState<"FOCUS" | "GARDEN">("FOCUS");
  const [plants, setPlants] = useState<GardenPlantSummary[]>([]);
  const [selectedPlant, setSelectedPlant] = useState<GardenPlantDetail | null>(null);
  const [isFinalizing, setIsFinalizing] = useState(false);

  useEffect(() => {
    let isMounted = true;

    void Promise.all([getPersistenceStatus(), getCurrentFocusSession(), listGardenPlants()])
      .then(([nextStatus, nextSession, nextPlants]) => {
        if (isMounted) {
          setStatus(nextStatus);
          setSession(nextSession);
          setPlants(nextPlants);
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
  const finishAndPlant = () => {
    if (!session || isFinalizing) return;
    setIsFinalizing(true); setError(null);
    void finalizeFocusSession(session.id).then((plant) => {
      setSession(null); setSelectedPlant(plant); setPlants((current) => [plant, ...current]); setView("GARDEN");
    }).catch((reason: unknown) => setError(reason instanceof Error ? reason.message : "Your completed session could not be planted."))
      .finally(() => setIsFinalizing(false));
  };
  const openPlant = (id: string) => {
    setError(null);
    void getGardenPlant(id).then(setSelectedPlant).catch(() => setError("That garden plant could not be opened."));
  };

  return (
    <main className="focus-shell">
      <section className="focus-card" aria-labelledby="app-title">
        <p className="eyebrow">A quiet place to grow</p>
        <h1 id="app-title" className="text-4xl font-semibold tracking-tight">
          Focus Garden
        </h1>
        <nav className="actions" aria-label="Main navigation"><button className="quiet" onClick={() => setView("FOCUS")}>Focus</button><button className="quiet" onClick={() => setView("GARDEN")}>Garden</button></nav>
        {isUnavailable && <p className="notice" role="alert">Focus Garden cannot reach its local journal.</p>}
        {error && <p className="notice" role="alert">{error}</p>}
        {view === "FOCUS" && !session && status?.initialized && (
          <div className="starter">
            <p>Choose a little protected time.</p>
            <div className="duration-options" aria-label="Focus duration">
              {[1, 10, 25, 45].map((minutes) => <button key={minutes} className="duration" onClick={() => apply(startFocusSession(minutes * 60 * 1_000))}>{minutes} min</button>)}
            </div>
          </div>
        )}
        {view === "FOCUS" && session && (
          <div className="session-view">
            <p className="eyebrow">{session.status.replaceAll("_", " ")}</p>
            <PlantSvg genome={session.plantGenome} growth={growth} className="live-plant" label={`Your growing plant, ${Math.round(growth * 100)} percent grown`} />
            <output className="timer" aria-label="Remaining focus time">{formatRemaining(remainingMs)}</output>
            {session.status === "ACTIVE" && remainingMs > 0 && <div className="actions"><button className="primary" onClick={() => apply(pauseFocusSession(session.id))}>Pause</button><button className="quiet" onClick={() => apply(abandonFocusSession(session.id))}>Abandon</button></div>}
            {session.status === "ACTIVE" && remainingMs === 0 && <div className="actions"><button className="primary" disabled={isFinalizing} onClick={finishAndPlant}>{isFinalizing ? "Planting…" : "Finish & plant"}</button><button className="quiet" disabled={isFinalizing} onClick={() => apply(abandonFocusSession(session.id))}>Abandon</button></div>}
            {session.status === "PAUSED" && <div className="actions"><button className="primary" onClick={() => apply(resumeFocusSession(session.id))}>Resume</button><button className="quiet" onClick={() => apply(abandonFocusSession(session.id))}>Abandon</button></div>}
            {session.status === "RECOVERY_REQUIRED" && <div className="recovery"><p>Your session was interrupted. Choose how to continue.</p><div className="actions"><button className="primary" onClick={() => apply(resolveFocusSessionRecovery(session.id, "CONTINUE"))}>Continue</button><button className="quiet" onClick={() => apply(resolveFocusSessionRecovery(session.id, "ABANDON"))}>Abandon</button></div></div>}
          </div>
        )}
        {view === "GARDEN" && <section className="session-view" aria-label="Garden">
          <p className="eyebrow">Garden</p>
          {plants.length === 0 && <p>Your garden is waiting for its first finished focus session.</p>}
          <div className="duration-options">{plants.map((plant) => <button className="duration" key={plant.id} onClick={() => openPlant(plant.id)}><PlantSvg genome={plant.plantGenome} growth={1} label="Finished focus plant" /><span>{new Date(plant.plantedAt).toLocaleDateString()}</span></button>)}</div>
          {selectedPlant && <article className="recovery"><PlantSvg genome={selectedPlant.plantGenome} growth={1} label="Finished focus plant" /><p>Planted {new Date(selectedPlant.plantedAt).toLocaleString()}</p><p>Source session: {selectedPlant.sourceSession.id}</p><p>{Math.round(selectedPlant.sourceSession.plannedDurationMs / 60_000)} min · {selectedPlant.sourceSession.pauseCount} pauses</p></article>}
        </section>}
      </section>
    </main>
  );
}
