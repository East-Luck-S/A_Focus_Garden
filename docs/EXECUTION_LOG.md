# Execution Log

## M13 — Release Hardening — INTERNAL RC VERIFIED / HUMAN GATES OPEN

- Unified source and lockfile root versions at internal candidate `1.0.0-rc.2` while preserving the stable `Focus Garden` package identity, `com.focusgarden.desktop` data identity, schema version 5, and generator version 1.
- Reduced the native WebView attack surface: production CSP has no inline-style, development URL, development command, or `devCsp` allowance; prototype freezing and Tauri CSP asset hardening remain enabled; only the named main-window capability is loaded, with no unused Tauri core commands. `npm run dev:native` explicitly merges a separate development-only config containing the Vite and WebSocket sources.
- Native command normalization discards diagnostic messages before they enter frontend state. Rust serialization tests cover filesystem, SQLite, migration, and repository sentinel diagnostics and confirm that paths, SQL text, and plant seed field names do not enter IPC error payloads.
- Added a reproducible Windows release pipeline. Rust path remapping removes workspace and Cargo-home paths, and every `npm run release:build` now runs a second verifier over both executable artifacts. The verifier locks product/version metadata, reports SHA-256 and Authenticode status, rejects source maps, absolute build/user paths, development endpoints, private-key markers, and common key-like tokens, and can require a valid signature for a public build.
- A clean `npm ci` and subsequent full regression passed: TypeScript, lint, 68/68 frontend tests, production web build, locked Rust formatting/check, 52 library tests, 2 release-shell tests, and clippy with warnings denied. Both the development application and release executable launched, remained responsive, exposed the Chinese title `专注花园`, exited cleanly, and left no residual process.
- Dependency evidence: npm reported zero known vulnerabilities. RustSec reported zero vulnerabilities across 443 locked crates and eight allowed informational warnings; the `glib` advisory is absent from the Windows target tree. The Windows non-development Rust graph contains permissive and five MPL-2.0 transitive crates, with no GPL, AGPL, or SSPL dependency. A 232,472-byte lockfile-bound `THIRD_PARTY_NOTICES.txt` includes original license files for four npm runtime packages and unescaped cargo-about license text covering 244 unique non-development Windows Rust graph packages, conservatively retaining build dependencies. Source and installed copies exactly match SHA-256 `65D2BF685AEDB8802D1B282F63E7D09945AB55B52B27705FCF14568C2E0CEAAF`.
- The Chinese application header now exposes a keyboard-contained About dialog. It reads the version directly from the package manifest, states the local-only/no-account/no-cloud/no-telemetry boundary, repeats the non-destructive data-safety promise, and points to the installed notice filename without exposing a machine-specific path. Tab, Shift+Tab, Escape, initial focus, and trigger focus restoration are covered by the frontend suite.
- Final internal artifacts passed the automated scan. `focus-garden.exe` is 5,884,928 bytes with SHA-256 `DCC4EB89DD06B8AF3393DF4658AD4A446354AEF27F14DF8B8F66CA56808685E1`; `Focus Garden_1.0.0-rc.2_x64-setup.exe` is 2,164,920 bytes with SHA-256 `9ECF39B31BA792B3484CD6E29036B70E2CEC4C659D2DA802A62062D44685464A`. Both carry product version `1.0.0-rc.2`; both are `NotSigned`, so they are internal candidates rather than public release artifacts.
- The NSIS installer upgraded the installed rc.1 in place with exit code 0 and produced exactly one rc.2 uninstall entry, one install directory, and one Start Menu shortcut. The application database SHA-256 remained `AAF2CFC17FBA6780C35BAD5E8535DD6E1D5250C21801620C0F740E87D3E5E7D7` before installation, after installation, after first rc.2 launch, and after a second launch. Read-only checks after restart found migrations `1,2,3,4,5`, `integrity_check=ok`, zero foreign-key violations, four sessions (two completed, two abandoned, zero open), two plants, a 45-minute default, zero plant/session receipt mismatches, and no completed session missing its plant.
- The installed executable has SHA-256 `46375460F33A43494415DFD5815F6151ABD0326CD8CD4C07F757A50BC208634D` and differs from the pre-install executable at exactly three bytes: Tauri's `__TAURI_BUNDLE_TYPE_VAR_UNK` marker becomes `..._NSS`. The bundled Tauri 2.9.3 source maps `NSS` to `BundleType::Nsis`; no other byte differs. The installed executable separately passed the privacy/secret scan. Two post-install launches were responsive, closed normally, left no process, and preserved the exact database hash and all migration/session/plant invariants.
- Current-host evidence is limited to Windows NT build 26200 / registry display version 25H2. The computer-control surface did not expose native Windows applications, so Narrator, operating-system High Contrast interaction, and 200% display-scaling acceptance are not claimed. A direct 0.1.0-to-rc.2 isolated upgrade and uninstall/reinstall data-retention exercise are also deferred rather than risking the verified local database.
- M13 and V1.0 remain open. Public release is blocked on a Human-selected Focus Garden distribution license/EULA, final legal review of the generated notice, Windows publisher/code-signing identity, signed-artifact verification, the remaining Windows 10/11 and assistive-technology matrix, and final Human visual/product acceptance.

## M12 — UX / Accessibility / Performance Hardening — CLOSED

- Hardened the Chinese Focus flow around stable focus placement, explicit duration validation, meaningful state announcements, and a fully contained abandonment dialog. The wall-clock projection interval exists only while an active session has time remaining and clears itself at zero; paused and recovery previews remain static. Garden entry, retry, pagination exhaustion, and detail-dialog close all preserve a predictable keyboard focus target.
- Improved the existing visual system without changing domain truth: `PlantSvg` requires a Chinese accessible label and is memoized for static Garden cards, while live animation is restricted to active Focus plants. Contrast, forced-colors, reduced-motion, high zoom, and short-window layouts now have explicit presentation rules.
- Reworked the existing Garden reads without a schema change. Cursor pages use the existing `(planted_at, id)` index as a row-value range seek, and the overview is deliberately plants-driven before its indexed session lookup. A 240-plant service test covers ten pages, ordering, uniqueness, and exact overview totals; bundled SQLite query-plan assertions prevent full plant/session scans and temporary sorts on the bounded paths.
- On Windows, the normal gray-green caption remains in place, while system High Contrast selects DWM default caption, text, and border colors. The policy is refreshed on theme changes and when the window regains focus.
- Validation: frontend typecheck, lint, production build, and 64 tests passed. Cargo formatting, check, clippy with warnings denied, 51 library tests, and 2 release-shell tests passed. A native development executable compiled and launched successfully. Read-only verification of the application database found migrations `1,2,3,4,5`, four sessions (two completed, two abandoned, zero open), two plants, zero receipt mismatches, and `integrity_check=ok`.
- Schema remains version 5; the migration chain, generator version 1, immutable generation receipts, and Cargo lockfile are unchanged. The current computer-control surface did not expose native Windows applications, so manual Narrator, operating-system High Contrast switching, and 200% display-scaling interaction remain explicit human/M13 acceptance items rather than claimed evidence.

## M11 — Plant System Expansion & Visual Quality — CLOSED

- Delivered Plant Visual System V1.1 as a renderer-only expansion. The five persisted morphologies now project into different structures: fern fronds, a low succulent rosette, an open wildflower, vine tendrils, and a branched shrub canopy. Oval, lanceolate, serrated, and round leaves have distinct paths; bloom, bell, star, and cluster flowers use distinct SVG geometry.
- New secondary stems, tendrils, rosette leaves, cluster heads, and ground shadow remain disposable scene data derived only from the persisted genome and supplied growth. No randomness, clock, native call, authoritative trait, schema change, migration, or rewrite of historical plant receipts was introduced. Schema remains v5 and generator version remains 1.
- Growth-stage opacity remains authoritative for every organ. Live Focus plants use restrained sway, pollen drift, and dew brightness; Garden and completion plants remain static, and the existing reduced-motion rule suppresses non-essential animation.
- Validation: frontend 49 tests, including morphology signatures, four leaf paths, four flower geometries, stage boundaries, finite scene geometry, deterministic/no-mutation behavior, and accessible SVG metadata. Full frontend and Rust gates passed; the Rust V1 golden generator fixture remains unchanged.
- Native visual validation loaded two persisted historical V1 fern genomes in the Garden without rewriting their receipts. Their cards and detail view rendered the expanded frond, flower, effect, and ground-shadow scene correctly; closing the detail with Escape restored keyboard focus to its source card. Automated structural tests cover the remaining morphologies and growth stages; they are not represented as manual native visual checks.

## M10 — Chinese Windows Release Candidate — CLOSED

- Unified release metadata at version 1.0.0-rc.1. The interface, window title, bundle descriptions, and NSIS installer language are Simplified Chinese; the stable `Focus Garden` package identity and Tauri identifier `com.focusgarden.desktop` are retained for upgrade and data-path continuity. A test with a translated package name produced a side-by-side installation, so that unsafe identity change was reverted before release.
- Release windows default to 960×720 with a 640×560 minimum. The native Windows caption and border use the page's gray-green palette with dark title text where DWM caption colors are supported. Production CSP permits only local application assets and Tauri IPC, while the separate development CSP contains the Vite localhost and WebSocket allowances.
- A release-only Windows startup failure retains diagnostic stderr output and shows a stable Chinese MessageBox that does not expose internals or imply an automatic local-data reset. Both product dialogs now trap Tab and Shift+Tab and restore focus on close.
- Validation: frontend 41 tests; Rust 49 library tests plus 2 release-shell tests; typecheck, lint, production web build, Cargo formatting/check/test/clippy, native development launch, and `npm run tauri build` passed. The release executable is 5,918,208 bytes with SHA-256 `FA9CF42FFDB14ADFD144399A5D6D1A3938DEFA451FE895DEC7554CAC1B03594F`; the NSIS installer is 2,151,531 bytes with SHA-256 `58F385352DA9973C827D6E0E67634A30F8CE678E3280DE80CD4D9B9BF3614A25`. Both carry product version `1.0.0-rc.1` and are unsigned.
- Upgrade verification installed the historical `Focus Garden 0.1.0` package and then the RC. The corrected stable package identity produced exactly one `1.0.0-rc.1` uninstall entry, one install directory, and one Start Menu shortcut. The separate `com.focusgarden.desktop` database hash was unchanged across install and upgrade. The installed RC launched and restarted successfully; read-only verification after restart found migrations `1,2,3,4,5`, three sessions (one completed, two abandoned, zero open), one plant, no foreign-key violations, and `integrity_check=ok`.

## M9 — Core Garden & Local Overview — CLOSED

- Schema remains version 5: M9 adds no migration, table, or index. Rust exposes a bounded keyset Garden page (default 24, maximum 50), explicit plant detail, and a single-query aggregate overview over the existing plant/session relationship.
- The Chinese Garden projection loads only when entered, leaving Focus boot independent. It displays safe summaries, appends deduplicated pages, preserves loaded cards on later-page failure, and presents a focus-managed accessible detail dialog without source IDs or seeds.
- Validation recorded for M9: Rust 48 tests and frontend 41 tests, plus typecheck, lint, production build, Cargo formatting/check/test/clippy, and diff checks.

## M8 — Formal Focus Flow & Duration Settings — CLOSED

- Schema version 5 adds a lossless singleton `app_settings` default duration, initialized to 25 minutes and constrained to 1–180 minutes in SQLite and Rust.
- The Chinese formal flow persists preset or valid custom durations, serializes pending actions, confirms abandonment explicitly, and retains completion as a result card until the user chooses the Garden or another focus.
- Final validation is recorded with the M8 delivery commit.

## M7 — Chinese Product Shell — CLOSED

- Replaced the English prototype surface with a Chinese-first shell, explicit loading/ready/fatal startup states, retry, localized status/date rendering, and accessible navigation controls.
- Native wrapper rejections are normalized and stable native error codes map to safe Chinese copy. Data-safety and migration blockers explicitly state that local data is protected and will not be reset automatically.
- Focus and Garden are separate React projections; Garden reads can fail and retry independently without blocking native-authoritative Focus work. Schema and native business commands remain at version 4.

## M6 — MVP Transactional Vertical Slice — CLOSED

- Added schema version 4, `004_transactional_plants`, append-only `plants`, exact source/session triggers, immutable receipts, v3 preflight, and post-migration startup reconciliation.
- Added one immediate transactional finalizer, idempotent exact completed reads, typed Garden list/detail commands, and a minimal Focus/Garden UI with explicit `Finish & plant` action.
- Added tests for valid and invalid v3 historical reconciliation, v4 missing-plant startup blocking, abandoned zero-plant behavior, atomic rollback injection, idempotence, restart list/detail, and native wrapper/UI pending behavior.
- Native release evidence: session `1d35b42c-8344-4df0-a0d2-e78402a25842` started with a full bundle and no plant; after full process exit and an elapsed target it restarted as `RECOVERY_REQUIRED` with no plant; explicit Continue followed by `Finish & plant` yielded one completed session and plant with matching ID/source and timestamp `1790053533747`. A third fresh launch still rendered the Garden plant. Read-only database validation confirmed `[1,2,3,4]`, completed=1, open=0, plants=1, exact joined invariant=1, no foreign-key rows, and `integrity_check=ok`; no process remained.
- Release package evidence: `focus-garden.exe` 5,890,048 bytes, SHA-256 `72E0F59FA08906E58DE559ED332B11CE96BB78EA766467769D64C5AC3CEF7FE9`; one NSIS installer `Focus Garden_0.1.0_x64-setup.exe` 2,140,985 bytes, SHA-256 `8BE79E7752F16C3DA370AABECC545E67E736091952607BC5B8809E45198432FB`.
- All M6 gates passed. Epoch A is complete; M7 has not started.

## M5 — Focus Flow & Real-time Growth

- Added schema version 3, `003_session_plant_preview`, with nullable `plant_seed`, `generator_version`, and `plant_genome_json` columns plus SQLite all-null/all-present enforcement triggers.
- New starts atomically persist a Rust-generated V1 preview bundle; 64-bit seeds use fixed lowercase hexadecimal strings across SQLite, DTOs, and React.
- Startup atomically backfills only all-null M2 open sessions before active-session recovery. Invalid or partial generation data returns `SESSION_GENERATION_DATA_INVALID` and is neither repaired nor regenerated. Complete persisted genomes are deserialized and retained exactly.
- The Focus Screen now offers 10/25/45 minute starts, PlantSvg live growth, pause/resume/abandon, and explicit recovery continuation. React intervals update only a wall-clock display projection.
- Deferred intact: M6 completion finalizer, reward/plant creation, plants table, final garden, installers, and all subsequent milestones.

## M4 — SVG Plant Renderer V1

- Added a TypeScript mirror of the serialized Rust `PlantGenome` V1 contract; it remains presentation-only and does not alter the Rust generator or persistence schema.
- `buildPlantScene(genome, growth)` is a deterministic, non-mutating SVG projection. It clamps supplied growth and derives seed, stem, leaves, branches, buds, flower, palette, and effects solely from the supplied genome.
- `PlantSvg` is an accessible, reusable SVG component with no native, persistence, clock, or random dependency. M5 may supply live growth; M6 may reuse it at growth `1`.
- Deferred: focus-flow wiring, session completion finalization, genome persistence, and garden presentation.

## M3 — Deterministic Plant Generator V1

- Schema version remains 2; M3 adds no migration or persistent plant table.
- Rust-domain V1 accepts validated stable generation input, resolves a replayable seed, and generates an integer-only ordered genome with morphology, stem, leaves, flower, palette, and effects.
- FNV-1a 64-bit seed resolution and SplitMix64 generation are documented fixed contracts; no clock or uncontrolled randomness participates.
- Deferred: SQLite seed/genome persistence, focus-flow integration, growth, and SVG rendering.

## M2 — Focus Session Domain & Timer

- Schema version: 2 (`002_focus_sessions`)
- Rust owns the persisted wall-clock state machine; React only projects it.
- A partial unique SQLite index and `BEGIN IMMEDIATE` writes enforce one open session.
- Startup changes `ACTIVE` to `RECOVERY_REQUIRED`; paused sessions remain paused and recovery requires an explicit resolution.
- Deferred: plants, genomes, SVG rendering, completion finalization, and garden projection.
