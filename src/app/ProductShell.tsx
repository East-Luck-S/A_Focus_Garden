import { useCallback, useEffect, useState } from "react";

import { FocusScreen } from "../features/focus/FocusScreen";
import { GardenScreen } from "../features/garden/GardenScreen";
import {
  getCurrentFocusSession,
  getPersistenceStatus,
  listGardenPlants,
  nativeErrorMessage,
  type FocusSessionSnapshot,
  type GardenPlantDetail,
  type GardenPlantSummary,
} from "../shared/native";

type BootState = "LOADING" | "READY" | "FATAL";
type View = "FOCUS" | "GARDEN";

export function ProductShell() {
  const [boot, setBoot] = useState<BootState>("LOADING");
  const [fatal, setFatal] = useState<unknown>(null);
  const [session, setSession] = useState<FocusSessionSnapshot | null>(null);
  const [plants, setPlants] = useState<GardenPlantSummary[]>([]);
  const [gardenError, setGardenError] = useState<unknown>(null);
  const [view, setView] = useState<View>("FOCUS");

  const loadGarden = useCallback(() => {
    setGardenError(null);
    void listGardenPlants().then(setPlants).catch(setGardenError);
  }, []);
  const loadCore = useCallback(
    async () => Promise.all([getPersistenceStatus(), getCurrentFocusSession()]),
    [],
  );
  const bootApp = () => {
    setBoot("LOADING");
    setFatal(null);
    void loadCore()
      .then(([, nextSession]) => {
        setSession(nextSession);
        setBoot("READY");
        loadGarden();
      })
      .catch((reason: unknown) => {
        setFatal(reason);
        setBoot("FATAL");
      });
  };

  useEffect(() => {
    void loadCore()
      .then(([, nextSession]) => {
        setSession(nextSession);
        setBoot("READY");
        loadGarden();
      })
      .catch((reason: unknown) => {
        setFatal(reason);
        setBoot("FATAL");
      });
  }, [loadCore, loadGarden]);

  if (boot === "LOADING") {
    return <main className="focus-shell"><section className="focus-card" aria-live="polite"><h1>专注花园</h1><p>正在准备专注花园…</p></section></main>;
  }
  if (boot === "FATAL") {
    return <main className="focus-shell"><section className="focus-card"><h1>暂时无法打开专注花园</h1><p className="notice" role="alert">{nativeErrorMessage(fatal)}</p><p>为了保护本机数据，应用不会自动重置任何内容。</p><button className="primary" onClick={bootApp}>重试</button></section></main>;
  }
  const planted = (plant: GardenPlantDetail) => { setPlants((current) => [plant, ...current]); setView("GARDEN"); };
  return <main className="focus-shell"><section className="focus-card" aria-labelledby="app-title"><p className="eyebrow">专注花园 / Focus Garden</p><h1 id="app-title">专注花园</h1><nav className="actions" aria-label="主导航"><button className="quiet" aria-current={view === "FOCUS" ? "page" : undefined} onClick={() => setView("FOCUS")}>专注</button><button className="quiet" aria-current={view === "GARDEN" ? "page" : undefined} onClick={() => setView("GARDEN")}>花园</button></nav>{view === "FOCUS" ? <FocusScreen session={session} onSessionChange={setSession} onPlanted={planted} /> : <GardenScreen plants={plants} loadError={gardenError} onRetry={loadGarden} />}</section></main>;
}
