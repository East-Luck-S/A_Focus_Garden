import "./app.css";

export function App() {
  return (
    <main className="grid min-h-screen place-items-center bg-stone-950 px-6 text-stone-100">
      <section className="text-center" aria-labelledby="app-title">
        <p className="mb-3 text-xs font-semibold uppercase tracking-[0.28em] text-emerald-400">
          Windows desktop application
        </p>
        <h1 id="app-title" className="text-4xl font-semibold tracking-tight">
          Focus Garden
        </h1>
        <p className="mt-4 text-base text-stone-400">
          Repository foundation ready.
        </p>
      </section>
    </main>
  );
}
