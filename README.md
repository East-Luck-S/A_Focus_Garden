# Focus Garden

Focus Garden is a Windows-only focus application planned around a simple loop: a focus session drives real-time growth, produces a deterministic plant, and adds it to a persistent garden.

The project has closed **M5: Focus Flow & Real-time Growth** and is now entering M6. M5 wires a calm focus screen to an immutable Rust-authoritative plant preview and real-time display growth. Completion finalization, rewards, persistent plants, and the Garden remain deferred to later milestones.

## Technology baseline

- Tauri 2 and Rust for the native Windows shell and future authoritative domain
- Vite, React, and strict TypeScript for presentation and interaction
- Tailwind CSS for styling
- Vitest and React Testing Library for frontend tests
- Cargo formatting, Clippy, and Rust tests for the native layer
- npm for JavaScript dependency management
- `rusqlite` with bundled SQLite for local persistence; users do not need to install SQLite separately

The formal runtime has no AI API, cloud backend, account, cloud sync, social, payment, ads, or telemetry.

## Development prerequisites

Use Windows 10 or Windows 11 with:

- Node.js and npm
- The Rust MSVC toolchain with the `x86_64-pc-windows-msvc` target
- Microsoft C++ Build Tools with the **Desktop development with C++** workload and a Windows SDK
- Microsoft Edge WebView2 Runtime (included with current Windows 10 and Windows 11 installations)

See the official [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/#windows) if the native build cannot locate MSVC, the Windows SDK, or WebView2.

## Install

```powershell
npm ci
```

Use `npm install` only when intentionally updating dependencies or the lockfile.

## Development run

Run the web frontend alone:

```powershell
npm run dev
```

Run the native Tauri application:

```powershell
npm run tauri dev
```

## Validation

```powershell
npm run typecheck
npm run lint
npm run test:run
npm run build
```

Run all frontend validation in sequence with:

```powershell
npm run check
```

For interactive test watch mode, run `npm test`.

Validate the Rust layer from `src-tauri`:

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy -- -D warnings
```

## Persistence and focus-session foundation

Rust exclusively owns SQLite. React uses a typed native wrapper for the explicit `get_persistence_status` command and has no SQL or database-path access. On native startup, Tauri resolves the application's local data directory, safely creates it when needed, and initializes `focus-garden.sqlite3` there. The precise user-specific directory is intentionally not part of the frontend API or this documentation.

The database has migration tracking (`schema_migrations`), application metadata (`app_metadata`), and persistent focus sessions (`sessions`). Schema changes are atomic and versioned. Schema version 3 stores an immutable session preview bundle: a fixed-width hexadecimal `u64` seed string, generator version, and exact Rust serde genome JSON. Rust owns timer truth as UTC milliseconds: `started_at`, `planned_duration_ms`, `target_end_at`, `paused_at`, and `total_paused_ms`. React's one-second interval refreshes only its display projection. Startup turns an interrupted `ACTIVE` session into `RECOVERY_REQUIRED`; paused sessions remain paused. A partial unique database index plus immediate transactions allow only one open session. M5 does not create a `plants` table, reward a finished timer, or finalize a session.

## Windows native build

Build the release executable and NSIS installer:

```powershell
npm run tauri build
```

Generated frontend output, Rust targets, and native bundles are intentionally excluded from version control.

## Architecture and roadmap

- [Project baseline](docs/PROJECT_BASELINE.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Decisions](docs/DECISIONS.md)
- [Data model](docs/DATA_MODEL.md)
