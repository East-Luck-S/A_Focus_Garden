# Execution Log

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
