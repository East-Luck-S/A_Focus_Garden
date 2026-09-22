# Data Model

M1 establishes only persistence infrastructure. Business domain schema is **not implemented in M1**.

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

Created by immutable `002_focus_sessions`. It is the canonical M2 focus-session record; no plant or garden data is present yet.

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

`sessions_single_open_session` is a partial unique index across non-terminal statuses, preventing more than one `ACTIVE`, `PAUSED`, or `RECOVERY_REQUIRED` session. The M2 schema intentionally has no plant, genome, rarity, garden, statistics, achievement, or tag table. Future schema changes must append a new versioned migration; existing migration definitions are immutable.
