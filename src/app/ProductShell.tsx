import { useCallback, useEffect, useRef, useState } from "react";

import { FocusScreen } from "../features/focus/FocusScreen";
import { GardenScreen } from "../features/garden/GardenScreen";
import {
  getAppSettings,
  getCurrentFocusSession,
  getPersistenceStatus,
  nativeErrorMessage,
  updateAppSettings,
  type AppSettings,
  type FocusSessionSnapshot,
} from "../shared/native";

type BootState = "LOADING" | "READY" | "FATAL";
type View = "FOCUS" | "GARDEN";

export function ProductShell() {
  const [boot, setBoot] = useState<BootState>("LOADING");
  const [fatal, setFatal] = useState<unknown>(null);
  const [session, setSession] = useState<FocusSessionSnapshot | null>(null);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [gardenRevision, setGardenRevision] = useState(0);
  const [view, setView] = useState<View>("FOCUS");
  const focusAfterRetryRef = useRef(false);
  const appTitleRef = useRef<HTMLHeadingElement>(null);

  const loadCore = useCallback(
    async () =>
      Promise.all([
        getPersistenceStatus(),
        getCurrentFocusSession(),
        getAppSettings(),
      ]),
    [],
  );

  const retryBoot = () => {
    setBoot("LOADING");
    setFatal(null);
    focusAfterRetryRef.current = true;
    void loadCore()
      .then(([, nextSession, nextSettings]) => {
        setSession(nextSession);
        setSettings(nextSettings);
        setBoot("READY");
      })
      .catch((reason: unknown) => {
        setFatal(reason);
        setBoot("FATAL");
      });
  };

  useEffect(() => {
    void loadCore()
      .then(([, nextSession, nextSettings]) => {
        setSession(nextSession);
        setSettings(nextSettings);
        setBoot("READY");
      })
      .catch((reason: unknown) => {
        setFatal(reason);
        setBoot("FATAL");
      });
  }, [loadCore]);

  useEffect(() => {
    if (boot === "READY" && focusAfterRetryRef.current) {
      focusAfterRetryRef.current = false;
      appTitleRef.current?.focus();
    }
  }, [boot]);

  const handlePlanted = () => {
    setGardenRevision((current) => current + 1);
  };

  const saveSettings = async (durationMs: number) => {
    const next = await updateAppSettings(durationMs);
    setSettings(next);
  };

  if (boot === "LOADING") {
    return (
      <main className="focus-shell">
        <section className="focus-card boot-card" aria-live="polite">
          <p className="eyebrow">专注花园</p>
          <h1>正在准备你的花园</h1>
          <p>正在安全读取本机数据…</p>
        </section>
      </main>
    );
  }

  if (boot === "FATAL" || !settings) {
    return (
      <main className="focus-shell">
        <section className="focus-card boot-card">
          <p className="eyebrow">启动受阻</p>
          <h1>暂时无法打开专注花园</h1>
          <p className="notice" role="alert">
            {nativeErrorMessage(fatal)}
          </p>
          <p>为了保护本机数据，应用不会自动重置任何内容。</p>
          <button className="primary" type="button" onClick={retryBoot}>
            重试
          </button>
        </section>
      </main>
    );
  }

  return (
    <main className="focus-shell">
      <section
        className={view === "GARDEN" ? "focus-card garden-shell-card" : "focus-card"}
        aria-labelledby="app-title"
      >
        <header className="app-header">
          <div>
            <p className="eyebrow">专注花园 / Focus Garden</p>
            <h1 ref={appTitleRef} id="app-title" tabIndex={-1}>专注花园</h1>
          </div>
          <nav className="main-nav" aria-label="主导航">
            <button
              className="quiet"
              type="button"
              aria-current={view === "FOCUS" ? "page" : undefined}
              onClick={() => setView("FOCUS")}
            >
              专注
            </button>
            <button
              className="quiet"
              type="button"
              aria-current={view === "GARDEN" ? "page" : undefined}
              onClick={() => setView("GARDEN")}
            >
              花园
            </button>
          </nav>
        </header>

        {view === "FOCUS" ? (
          <FocusScreen
            session={session}
            settings={settings}
            onSettingsChange={saveSettings}
            onSessionChange={setSession}
            onPlanted={handlePlanted}
            onOpenGarden={() => setView("GARDEN")}
          />
        ) : (
          <GardenScreen refreshKey={gardenRevision} />
        )}
      </section>
    </main>
  );
}
