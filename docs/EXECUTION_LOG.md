# Execution Log

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
