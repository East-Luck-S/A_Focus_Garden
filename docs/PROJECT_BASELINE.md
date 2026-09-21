# Focus Garden Project Baseline

## Status

This document defines the accepted M0 repository baseline. The repository contains a minimal Windows-native Tauri application with a React frontend and no product-domain implementation.

## Supported platform

Focus Garden targets Windows 10 and Windows 11 only. Other desktop and mobile platforms are outside the product baseline.

## Foundation toolchain

- Tauri 2, Rust, and the Windows MSVC target
- Vite, React, and TypeScript with `strict: true`
- Tailwind CSS through its Vite plugin
- ESLint using flat configuration
- Vitest, jsdom, React Testing Library, and jest-dom matchers
- npm with a committed `package-lock.json`

Dependencies are added just in time. Zustand, Framer Motion, date-fns, SQLite libraries, and Tauri plugins are deferred until a milestone has an implemented need for them.

## Repository layout

```text
src/
  app/             application shell and composition
  components/      reusable presentation components
  features/        frontend feature projections and interactions
  shared/          shared frontend utilities and types
  test/            frontend test setup
src-tauri/src/
  application/     native application services
  domain/          authoritative business domain
  infrastructure/  persistence and Windows integrations
docs/               baseline project documentation
```

The M0 boundary folders contain documentation or empty Rust modules only. They reserve explicit ownership boundaries without implementing future behavior.

## M0 verification contract

M0 is acceptable only when dependency installation, strict type checking, linting, the frontend smoke test, the frontend production build, Rust formatting/check/tests/Clippy, and a Tauri Windows native release build all pass.

## Explicit exclusions

M0 contains no focus timer, session state machine, persistence schema, plant generator, renderer, garden, statistics, tags, collection, backup/restore, desktop integration feature, AI, cloud service, or account system.
