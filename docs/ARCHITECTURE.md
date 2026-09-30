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

Only explicit Tauri commands are registered. M6 adds `finalize_focus_session`, `list_garden_plants`, and `get_garden_plant`; no generic query or execute-SQL command exists. `finalize_focus_session` alone transitions an eligible active session and inserts its exact immutable plant in one `BEGIN IMMEDIATE` transaction. A completed retry only reads and verifies its matching plant, and missing/mismatched transactional data is a `DATA_SAFETY_BLOCKER`, never an auto-repair.

The M6 native acceptance exercised this boundary through the release executable: start a legal one-minute session, fully exit, restart after the target time, explicitly continue recovery, finalize, then fully exit and restart into the same persisted Garden plant. A read-only SQLite post-check confirmed migration history, exact session/plant identity, foreign keys, and integrity. This is runtime evidence, not an alternate application path.

## Schema evolution and timestamps

`schema_migrations` is bootstrapped as infrastructure and every migration is an ordered registry entry executed within its own transaction. Recorded versions must form a continuous prefix beginning at 1 (for example, `[]`, `[1]`, or `[1,2]`); a gap is rejected with `MIGRATION_HISTORY_INVALID` before any pending migration runs. Version 1 creates only `app_metadata`. A database newer than `CURRENT_SCHEMA_VERSION` is rejected separately with `UNSUPPORTED_FUTURE_SCHEMA`; migrations never downgrade, reset, or alter historical migration definitions. Persisted timestamps use UTC Unix epoch milliseconds.

## Future plant renderer boundary

The planned SVG renderer is a React/TypeScript projection of an authoritative plant genome produced by Rust. Rendering may interpret the description visually, but it must not generate or mutate plant identity, genome, rarity, or growth truth. M3's generator is a pure Rust-domain function: stable generation input resolves to a saveable seed, and the V1 generator replayably maps that seed to an ordered, integer-only genome. It has no React, SVG, Tauri, SQLite, clock, or uncontrolled-randomness dependency.

## Focus-session timer and recovery

The Rust domain owns the session state machine and timer. Timer truth is `started_at`, `planned_duration_ms`, `target_end_at`, `paused_at`, and `total_paused_ms`, all in UTC Unix epoch milliseconds. Active remaining time is `max(0, target_end_at - now)`; paused remaining time is `max(0, target_end_at - paused_at)`. Resume shifts `target_end_at` by the exact recorded pause duration. Browser intervals refresh only a React projection and never transition a session.

Only one non-terminal session is allowed. Application writes begin in SQLite `BEGIN IMMEDIATE` transactions and the schema has a partial unique index over `ACTIVE`, `PAUSED`, and `RECOVERY_REQUIRED`, so the invariant is not UI-dependent. On launch, persisted `ACTIVE` sessions become `RECOVERY_REQUIRED`; they cannot auto-complete or create rewards. Explicit recovery `CONTINUE` preserves target end time and pause total, while `ABANDON` ends the session. Paused sessions survive launch as paused.

## M5 preview identity and growth projection

Every newly started session resolves its Rust V1 seed and genome inside the same `BEGIN IMMEDIATE` transaction that inserts the session. The database stores a generation bundle: a 16-character lowercase hexadecimal seed string, integer generator version, and exact serde genome JSON. The string representation is mandatory because JavaScript numbers cannot safely carry all `u64` values. Existing M2 open sessions with an all-null bundle are deterministically backfilled before launch recovery proceeds. A partial bundle, malformed seed, unsupported/mismatched version, or invalid genome JSON stops the operation with `SESSION_GENERATION_DATA_INVALID`; the application never silently regenerates, deletes, or repairs it. A complete persisted genome is deserialized and rendered as-is, even if a newer generator would now make a different result.

Rust snapshots include the immutable preview and a remaining-time observation. React derives `growth = clamp(1 - remaining / plannedDuration)` only for display. For `ACTIVE`, remaining is `max(0, targetEndAt - now)` and an interval refreshes `now`; the interval never calls a state-changing command. For `PAUSED`, it is frozen at `pausedAt`; for `RECOVERY_REQUIRED`, it remains the native snapshot until an explicit resolution. At zero time the active session remains active and no reward or plant record is created—M6 owns completion finalization.

## Native command boundary

Command payloads form an explicit boundary between frontend intent/projections and Rust application services. M2 adds typed focus-session commands while keeping completion internal until its M6 transactional finalizer exists.

## M8 settings and formal focus-flow boundary

M8 adds typed `get_app_settings` and `update_app_settings` commands. SQLite stores the canonical default duration; Rust validates every settings update and every session start at 1–180 minutes, independent of React. React keeps only transient custom-input state. Its UI serializes settings, start, session transitions, finalization, and abandonment through a single pending-action projection. Abandonment requires explicit accessible confirmation in React, while Rust remains authoritative for whether the transition is legal. Completion remains an explicit native transaction and is shown as a client-side result projection rather than an automatic Garden navigation.

## M7 product shell and native error boundary

M7 keeps Rust commands and the data model unchanged. The React shell has explicit loading, ready, and fatal startup states. Persistence or current-session failures stop the shell and offer retry without automatic data reset. Garden-list failure is isolated to the Garden projection and never blocks Focus. Typed native wrappers normalize rejected object, string, and unknown values to a stable frontend error shape; localized Chinese copy is selected by Rust's stable error code, while raw native messages and internal identifiers remain private.

## Runtime constraints

The formal runtime has no AI API, cloud backend, account, cloud sync, social system, payment, advertising, or telemetry. These are constraints, not merely missing M0 tasks.
