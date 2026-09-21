# Focus Garden

Focus Garden is a Windows-only focus application planned around a simple loop: a focus session drives real-time growth, produces a deterministic plant, and adds it to a persistent garden.

The project is currently at **M0: Repository Foundation & Windows Native Baseline**. This milestone provides only a runnable foundation app; it does not implement the timer, plant generation, persistence, or garden features.

## Technology baseline

- Tauri 2 and Rust for the native Windows shell and future authoritative domain
- Vite, React, and strict TypeScript for presentation and interaction
- Tailwind CSS for styling
- Vitest and React Testing Library for frontend tests
- Cargo formatting, Clippy, and Rust tests for the native layer
- npm for JavaScript dependency management

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
