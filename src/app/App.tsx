import "./app.css";

import { useEffect, useState } from "react";

import { getPersistenceStatus, type PersistenceStatus } from "../shared/native";

export function App() {
  const [status, setStatus] = useState<PersistenceStatus | null>(null);
  const [isUnavailable, setIsUnavailable] = useState(false);

  useEffect(() => {
    let isMounted = true;

    void getPersistenceStatus()
      .then((nextStatus) => {
        if (isMounted) {
          setStatus(nextStatus);
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

  return (
    <main className="grid min-h-screen place-items-center bg-stone-950 px-6 text-stone-100">
      <section className="text-center" aria-labelledby="app-title">
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
      </section>
    </main>
  );
}
