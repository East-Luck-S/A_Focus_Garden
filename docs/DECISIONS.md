# Architecture Decisions

## ADR-001: Windows-only product baseline

**Status:** Accepted for M0

Focus Garden targets Windows 10 and Windows 11. Native validation and documentation optimize for the Windows MSVC toolchain rather than claiming untested cross-platform support.

## ADR-002: Tauri 2 with a React/TypeScript frontend

**Status:** Accepted for M0

Tauri 2 and Rust provide the native application boundary. Vite, React, and strict TypeScript provide the presentation layer. Electron, Next.js, and a web backend are excluded.

## ADR-003: Rust will own authoritative business behavior

**Status:** Accepted as an architectural boundary; implementation deferred

Future session, plant, persistence, transaction, migration, backup, and Windows-integration behavior belongs behind the Rust application/domain boundary. React owns presentation, interaction, projections, SVG rendering, and transient UI state.

## ADR-004: SQLite is the canonical persistent source of truth

**Status:** Accepted and implemented for M1

SQLite access exists only in Rust through `rusqlite` with its bundled SQLite feature. React never receives generic SQL access; it calls specific typed Tauri command wrappers. Browser storage and frontend state persistence are not substitutes for canonical business persistence.

## ADR-008: Versioned migrations begin with the first schema

**Status:** Accepted and implemented for M1

All schema evolution uses ordered, versioned migrations recorded in `schema_migrations`. Migration 001 creates only the infrastructure `app_metadata` table. Each migration is atomic, failures roll back, and databases newer than the application schema are rejected rather than downgraded or reset.

## ADR-009: Persist timestamps as UTC Unix epoch milliseconds

**Status:** Accepted and implemented for M1

Persistence timestamps use UTC Unix epoch milliseconds. This is timezone-independent, sortable, compact, and consistent across migration tracking and future persistence records.

## ADR-010: Keep SQLite default journal mode in M1

**Status:** Accepted for M1

M1 intentionally leaves SQLite's journal mode at its default. WAL is not needed for the single-user foundation workload and would require explicit backup/restore coordination for sidecar files. The connection policy enables foreign keys and a bounded busy timeout only.

## ADR-011: Rust owns focus-session timer truth

**Status:** Accepted and implemented for M2

The authoritative timer is a persisted wall-clock model in Rust. React may tick for display but cannot progress, pause, resume, complete, or repair a session by itself. This remains correct across rendering delays, window blur/minimize, and application restart.

## ADR-012: Explicit recovery for interrupted active sessions

**Status:** Accepted and implemented for M2

An active session discovered on cold launch changes to `RECOVERY_REQUIRED`; it is never silently completed, abandoned, or rewarded. The user must explicitly continue without inventing paused time or abandon it. Paused sessions remain paused.

## ADR-013: Versioned deterministic plant-generation contract

**Status:** Accepted and implemented for M3

Plant generation is a pure Rust-domain contract. V1 resolves a seed from validated stable input with documented FNV-1a 64-bit hashing and generates its genome with a fixed integer SplitMix64 PRNG. It neither reads a clock nor uses uncontrolled randomness. The public genome uses ordered structs, enums, vectors, and integer fields so serialized output remains stable. Generator versions are explicit and unsupported versions fail rather than silently changing a saved plant's meaning.

## ADR-014: Persist immutable session preview bundles

**Status:** Accepted and implemented for M5

Focus-session preview identity is persisted at start, not recreated in React or recalculated during recovery. A transaction writes the session, hexadecimal `u64` seed, generator version, and exact serialized genome together. A complete stored bundle is deserialized and used unchanged; legacy all-null open sessions are deterministically backfilled once, while partial or invalid bundles are treated as integrity failures. This prevents generator upgrades, JavaScript integer precision, and restart paths from changing an in-progress plant.

## ADR-015: Growth is a read projection, never a completion signal

**Status:** Accepted and implemented for M5

Growth derives from persisted timer truth and an observed wall clock. Browser intervals refresh the display only. A zero remaining value renders fully grown but does not transition a session, create a plant, or issue a reward. Those atomic completion responsibilities remain exclusively in M6.

## ADR-016: Completion creates an immutable transactional plant receipt

**Status:** Accepted and implemented for M6

An elapsed timer is not a reward signal. The only completion use case uses one immediate SQLite transaction: validate the persisted generation bundle, complete the active session, insert the exact plant receipt, read the joined result, and commit. SQLite triggers make receipts and their source completion fields immutable. Historical data that cannot satisfy the exact one-plant relationship is blocked rather than regenerated or repaired.

## ADR-005: Just-in-time dependencies

**Status:** Accepted for M0

M0 installs only packages used by the foundation application, linting, testing, styling, and native build. Planned libraries are deferred until their owning milestone.

## ADR-006: Minimal native capability surface

**Status:** Accepted for M0

The foundation window uses only Tauri core defaults. The template greeting command and opener plugin were removed because M0 has no use case requiring them.

## ADR-007: NSIS is the M0 Windows packaging target

**Status:** Accepted for M0

The native acceptance build produces a Windows executable and NSIS installer. Limiting the target avoids adding the MSI-specific VBSCRIPT/WiX gate while still validating the Windows packaging pipeline.
