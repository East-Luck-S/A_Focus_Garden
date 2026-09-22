# Execution Log

## M2 — Focus Session Domain & Timer

- Schema version: 2 (`002_focus_sessions`)
- Rust owns the persisted wall-clock state machine; React only projects it.
- A partial unique SQLite index and `BEGIN IMMEDIATE` writes enforce one open session.
- Startup changes `ACTIVE` to `RECOVERY_REQUIRED`; paused sessions remain paused and recovery requires an explicit resolution.
- Deferred: plants, genomes, SVG rendering, completion finalization, and garden projection.
