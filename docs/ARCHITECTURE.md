# Architecture

## Platform boundary

Focus Garden is a Windows 10/11 desktop application. Tauri 2 supplies the native shell and Windows integration. The architecture does not include a web backend, Electron, Next.js, or a cross-platform product commitment.

## Responsibility boundary

```text
React / TypeScript                 Rust                         SQLite
------------------                 ----                         ------
Presentation             ->        Authoritative domain  <->   Canonical source
Interaction                        Application services         of persistent truth
Read projections                   Transactions
SVG rendering                      Migrations
Transient UI state                 Backup / restore
                                   Windows integration
```

React components may render state, collect user intent, and manage transient interface state. They must not become the authoritative owner of focus-session or plant-domain state, and they must not access SQLite directly. They call only explicit typed native wrappers; React has no raw SQL, database-path, or transaction capability.

Rust will own authoritative business rules, session transitions, deterministic plant generation, transactions, persistence coordination, and native Windows integration as those milestones are approved.

SQLite is the canonical persistent source of truth and is owned exclusively by Rust. The application layer calls the infrastructure persistence boundary, which configures SQLite connections and runs versioned migrations. Browser `localStorage` and Zustand persistence must not be used for business persistence.

## Persistence boundary

```text
React component -> typed native wrapper -> Tauri command -> application service
    -> Database / migration / transaction infrastructure -> SQLite
```

The M1 `Database` abstraction owns only a database `PathBuf`. Operations use short-lived configured connections so future repositories share one connection policy: foreign keys enabled and a five-second busy timeout. M1 deliberately keeps SQLite's default journal mode; WAL is deferred because it adds backup/restore coordination considerations without serving a current workload.

At application setup, Rust resolves Tauri's application-local data directory, creates it if necessary, opens `focus-garden.sqlite3`, applies migrations, and registers the persistence service. Startup failures surface as stable command error codes and never trigger database deletion, reset, or recreation.

Only explicit Tauri commands are registered. M1 exposes `get_persistence_status`; no generic query, execute-SQL, or database command exists. The command delegates to the application service, and command errors are serialized as stable `{ code, message }` values rather than raw SQLite details or user paths.

## Schema evolution and timestamps

`schema_migrations` is bootstrapped as infrastructure and every migration is an ordered registry entry executed within its own transaction. Recorded versions must form a continuous prefix beginning at 1 (for example, `[]`, `[1]`, or `[1,2]`); a gap is rejected with `MIGRATION_HISTORY_INVALID` before any pending migration runs. Version 1 creates only `app_metadata`. A database newer than `CURRENT_SCHEMA_VERSION` is rejected separately with `UNSUPPORTED_FUTURE_SCHEMA`; migrations never downgrade, reset, or alter historical migration definitions. Persisted timestamps use UTC Unix epoch milliseconds.

## Future plant renderer boundary

The planned SVG renderer is a React/TypeScript projection of an authoritative plant description produced by Rust. Rendering may interpret the description visually, but it must not generate or mutate plant identity, genome, rarity, or growth truth.

## Native command boundary

Command payloads form an explicit boundary between frontend intent/projections and Rust application services. The M1 shell exposes only the typed persistence status command.

## Runtime constraints

The formal runtime has no AI API, cloud backend, account, cloud sync, social system, payment, advertising, or telemetry. These are constraints, not merely missing M0 tasks.
