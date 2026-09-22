# Execution Log

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
