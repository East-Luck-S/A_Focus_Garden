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

## ADR-005: Just-in-time dependencies

**Status:** Accepted for M0

M0 installs only packages used by the foundation application, linting, testing, styling, and native build. Planned libraries are deferred until their owning milestone.

## ADR-006: Minimal native capability surface

**Status:** Accepted for M0

The foundation window uses only Tauri core defaults. The template greeting command and opener plugin were removed because M0 has no use case requiring them.

## ADR-007: NSIS is the M0 Windows packaging target

**Status:** Accepted for M0

The native acceptance build produces a Windows executable and NSIS installer. Limiting the target avoids adding the MSI-specific VBSCRIPT/WiX gate while still validating the Windows packaging pipeline.
