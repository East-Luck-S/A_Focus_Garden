import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";

import { version as appVersion } from "../../package.json";

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
  const [aboutOpen, setAboutOpen] = useState(false);
  const focusAfterRetryRef = useRef(false);
  const appTitleRef = useRef<HTMLHeadingElement>(null);
  const aboutTriggerRef = useRef<HTMLButtonElement>(null);
  const aboutCloseRef = useRef<HTMLButtonElement>(null);
  const restoreAboutFocusRef = useRef(false);

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

  useEffect(() => {
    if (aboutOpen) {
      aboutCloseRef.current?.focus();
    } else if (restoreAboutFocusRef.current) {
      restoreAboutFocusRef.current = false;
      aboutTriggerRef.current?.focus();
    }
  }, [aboutOpen]);

  const handlePlanted = () => {
    setGardenRevision((current) => current + 1);
  };

  const saveSettings = async (durationMs: number) => {
    const next = await updateAppSettings(durationMs);
    setSettings(next);
  };

  const closeAbout = () => {
    restoreAboutFocusRef.current = true;
    setAboutOpen(false);
  };

  const handleAboutKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Tab") {
      event.preventDefault();
      aboutCloseRef.current?.focus();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeAbout();
    }
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
            <button
              ref={aboutTriggerRef}
              className="quiet"
              type="button"
              aria-haspopup="dialog"
              onClick={() => setAboutOpen(true)}
            >
              关于
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

      {aboutOpen ? (
        <div className="dialog-backdrop">
          <section
            className="confirm-dialog about-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="about-title"
            aria-describedby="about-summary"
            onKeyDown={handleAboutKeyDown}
          >
            <p className="eyebrow">应用信息</p>
            <h2 id="about-title">关于专注花园</h2>
            <p id="about-summary" className="about-summary">
              一座只属于这台电脑的安静花园。
            </p>
            <dl className="about-facts">
              <div>
                <dt>版本</dt>
                <dd>{appVersion}</dd>
              </div>
              <div>
                <dt>数据</dt>
                <dd>仅保存在本机</dd>
              </div>
              <div>
                <dt>联网</dt>
                <dd>无需账户与网络</dd>
              </div>
            </dl>
            <p>
              专注记录与植物数据保存在本机 SQLite 数据库中；当前版本不包含账户、云同步、遥测或广告。
            </p>
            <p>
              若检测到数据库或迁移异常，应用会停止相关操作并保留原数据，不会自动重置数据库。
            </p>
            <p>
              第三方开源许可全文随应用安装，文件名为 <code>THIRD_PARTY_NOTICES.txt</code>。
            </p>
            <div className="actions">
              <button
                ref={aboutCloseRef}
                className="primary"
                type="button"
                onClick={closeAbout}
              >
                关闭
              </button>
            </div>
          </section>
        </div>
      ) : null}
    </main>
  );
}
