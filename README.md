# 专注花园 / Focus Garden

专注花园是一个 Windows 专注应用：一次专注会驱动植物成长，完成后种下一株可持久保存的确定性植物。当前内部发布候选版本为 **1.0.0-rc.2**，界面与 Windows 安装体验以简体中文为主。

当前本地数据库 schema 为 **v5**：它保留专注会话、确定性植物与种植记录，并保存 1–180 分钟的默认时长设置。花园以本地只读投影视图呈现，不会连接云端或自动重置本机数据。

The project has closed **M7: Chinese Product Shell**. The application is Chinese-first and starts through explicit loading, ready, and data-safe fatal states. A completed focus session is still finalized only by one native SQLite transaction that writes its immutable plant receipt; the Garden remains a read projection of those receipts.

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
npm run dev:native
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

## Third-party notices

The committed `THIRD_PARTY_NOTICES.txt` contains the original license files for the locked JavaScript runtime packages and license text collected from the locked non-development Windows Rust dependency graph, including build dependencies as a conservative disclosure. It is bundled into the Windows installation directory. Verify it with:

```powershell
npm run licenses:verify
```

After an intentional dependency update, install `cargo-about` and regenerate the notice:

```powershell
cargo install --locked cargo-about --features cli
npm run licenses:generate
```

The generated notice covers third-party software only. It does not choose or grant a license for Focus Garden itself.

## Persistence and focus-session foundation

Rust exclusively owns SQLite. React uses a typed native wrapper for the explicit `get_persistence_status` command and has no SQL or database-path access. On native startup, Tauri resolves the application's local data directory, safely creates it when needed, and initializes `focus-garden.sqlite3` there. The precise user-specific directory is intentionally not part of the frontend API or this documentation.

The database has migration tracking (`schema_migrations`), application metadata (`app_metadata`), focus sessions (`sessions`), and immutable completed plants (`plants`). Schema version 4 copies only validated historical completed session bundles into plants atomically; malformed, partial, or absent completed bundles stop upgrade with `DATA_SAFETY_BLOCKER`. Rust owns timer truth and finalization. The Garden receives typed native read projections only; React never accesses SQLite.

## Windows native build

Build the release executable and NSIS installer:

```powershell
npm run release:build
```

The release script uses stable Rust path remapping so packaged binaries do not retain the build machine's workspace or Cargo home paths. It also verifies artifact versions, hashes, signature status, source-map absence, and the absence of build paths, development endpoints, private-key markers, or common key-like tokens. Re-run the artifact checks without rebuilding with:

```powershell
npm run release:verify
```

Pass `-RequireSignature` directly to `scripts/verify-release.ps1` for a public build after the Human-selected Windows signing identity is available. Internal release candidates are allowed to report `NotSigned`; public V1.0 is not.

Generated frontend output, Rust targets, and native bundles are intentionally excluded from version control.

## Architecture and roadmap

- [Project baseline](docs/PROJECT_BASELINE.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Decisions](docs/DECISIONS.md)
- [Data model](docs/DATA_MODEL.md)
