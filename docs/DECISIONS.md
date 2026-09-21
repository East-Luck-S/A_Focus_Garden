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

## ADR-004: SQLite will be the future canonical persistent source of truth

**Status:** Accepted as an architectural boundary; implementation deferred to M1+

React will not access SQLite directly. Browser storage and frontend state persistence are not substitutes for canonical business persistence. No database dependency or schema is added in M0.

## ADR-005: Just-in-time dependencies

**Status:** Accepted for M0

M0 installs only packages used by the foundation application, linting, testing, styling, and native build. Planned libraries are deferred until their owning milestone.

## ADR-006: Minimal native capability surface

**Status:** Accepted for M0

The foundation window uses only Tauri core defaults. The template greeting command and opener plugin were removed because M0 has no use case requiring them.

## ADR-007: NSIS is the M0 Windows packaging target

**Status:** Accepted for M0

The native acceptance build produces a Windows executable and NSIS installer. Limiting the target avoids adding the MSI-specific VBSCRIPT/WiX gate while still validating the Windows packaging pipeline.
