# Execution Log

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
