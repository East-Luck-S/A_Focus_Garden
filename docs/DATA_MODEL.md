# Data Model

Current schema version: **5**. Migrations `001` through `004` are immutable historical definitions; upgrading an existing local database appends migration `005` without deleting or resetting user data.

## `app_settings`

Created by `005_app_settings`. This singleton table is the canonical persisted default for a new focus session.

| Column | Type | Meaning |
| --- | --- | --- |
| `singleton` | `INTEGER PRIMARY KEY` | Always `1`, enforced by `CHECK`. |
| `default_focus_duration_ms` | `INTEGER` | Default duration, constrained to `60000..10800000` (1–180 minutes); migration default is 25 minutes. |

This document records the persistence model through M9. Business-domain tables begin with the immutable M2 `sessions` schema and its append-only migrations.

## `schema_migrations`

Migration tracking created by bootstrap infrastructure before registered migrations run.

| Column | Type | Meaning |
| --- | --- | --- |
| `version` | `INTEGER PRIMARY KEY` | Unique, monotonically increasing migration version. |
| `name` | `TEXT NOT NULL` | Stable migration name. |
| `applied_at` | `INTEGER NOT NULL` | UTC Unix epoch milliseconds at application time. |

## `app_metadata`

Created by `001_persistence_foundation`. This table is reserved for stable application-level metadata, not a settings system.

| Column | Type | Meaning |
| --- | --- | --- |
| `key` | `TEXT PRIMARY KEY` | Unique metadata key. |
| `value` | `TEXT NOT NULL` | Metadata value. |
| `updated_at` | `INTEGER NOT NULL` | UTC Unix epoch milliseconds. |

## `sessions`

Created by immutable `002_focus_sessions`. It is the canonical focus-session record; M5 preview columns are appended separately by migration 003.

| Column | Type | Meaning |
| --- | --- | --- |
| `id` | `TEXT PRIMARY KEY` | Rust-generated UUID domain identifier. |
| `planned_duration_ms` | `INTEGER` | Positive planned duration in milliseconds. |
| `started_at` / `target_end_at` | `INTEGER` | UTC milliseconds used as wall-clock timer truth. |
| `paused_at` / `total_paused_ms` | `INTEGER` | Current pause marker and accumulated paused duration. |
| `completed_at` | `INTEGER` | Completion timestamp for a terminal completed session. |
| `status` | `TEXT` | `ACTIVE`, `PAUSED`, `COMPLETED`, `ABANDONED`, or `RECOVERY_REQUIRED`. |
| `pause_count` | `INTEGER` | Number of successful pauses. |
| `created_at` / `updated_at` | `INTEGER` | UTC milliseconds for persistence auditing. |

`sessions_single_open_session` is a partial unique index across non-terminal statuses, preventing more than one `ACTIVE`, `PAUSED`, or `RECOVERY_REQUIRED` session. Migration 003 adds the nullable generation bundle below without rewriting M2 session rows. Future schema changes must append a new versioned migration; existing migration definitions are immutable.

## Session plant preview bundle (M5 / migration 003)

| Column | Type | Meaning |
| --- | --- | --- |
| `plant_seed` | `TEXT NULL` | A fixed-width, 16-character lowercase hexadecimal encoding of the Rust `u64` seed. It is intentionally not an SQLite integer or JavaScript number. |
| `generator_version` | `INTEGER NULL` | The explicit generator version for the preview. |
| `plant_genome_json` | `TEXT NULL` | Exact serde JSON for the authoritative `PlantGenome`. |

SQLite insert/update triggers require this bundle to be all-null or all-present. Rust additionally validates seed syntax, supported version, and that the genome JSON deserializes with the same generator version. `ACTIVE`, `PAUSED`, and `RECOVERY_REQUIRED` records exposed by M5 always have a full valid bundle: M2-era all-null open records are atomically backfilled on startup. Historical `ABANDONED` records may remain all-null. No `plants` table exists in M5.

## `plants` (M6 / migration 004)

| Column | Type | Meaning |
| --- | --- | --- |
| `id` | `TEXT PRIMARY KEY` | Immutable plant identifier (currently the source session ID). |
| `source_session_id` | `TEXT UNIQUE NOT NULL FK` | The sole completed session that created this plant. |
| `plant_seed` / `generator_version` / `plant_genome_json` | `NOT NULL` | Exact validated copy of the session generation bundle. |
| `planted_at` | `INTEGER NOT NULL` | Exact copy of `sessions.completed_at`. |

Migration 004 and startup auditing require every `COMPLETED` session to have exactly one matching plant and every other session to have none. Foreign keys, exact-match insertion triggers, and immutable plant/session triggers enforce the relationship. The application never repairs a missing or mismatched receipt; it returns `DATA_SAFETY_BLOCKER`.

M6 native acceptance also independently verified the resulting production journal read-only: history `[1,2,3,4]`, one completed session and one plant, no open session, one exact joined invariant, an empty foreign-key check, and `integrity_check = ok`.

## M9 Garden read model

M9 adds no migration: the schema remains version 5. The Garden page and overview are read models over the existing `plants` and `sessions` relationship. Summaries contain the plant's display genome, `planted_at`, source planned duration, and source pause count; they do not expose the source session ID or seed to the product UI. Keyset pagination uses the existing `planted_at` and plant `id` values as the descending `(planted_at, id)` cursor. No M9-specific table or index is introduced; the joined read path is explicitly audited as a single query, and a future additive index remains possible if production profiling warrants one.

## M3 deterministic plant genome (domain value)

`PlantGenerationInput` is the stable generation-time value: `session_id`, `planned_duration_ms`, `started_at`, and `generator_version`. It resolves to a `u64` seed through a documented FNV-1a 64-bit byte contract (little-endian numeric fields and length-prefixed UTF-8 session ID). `generate_plant(seed, 1)` produces `PlantGenome` V1, containing morphology, stem, ordered leaves, flower, palette, and effects. Genomes contain only explicitly ordered structs, enums, vectors, and integers, so serialization does not depend on hash-map iteration or floating-point behavior.

M4 adds a frontend-only TypeScript DTO mirror with the same serialized snake_case fields and a deterministic SVG projection. M5 supplies a persisted native preview and a separately supplied clamped display-growth value. The renderer does not persist, mutate, generate, or otherwise make a genome authoritative.
