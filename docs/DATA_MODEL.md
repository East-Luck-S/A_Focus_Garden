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

No session, plant, tag, garden, statistics, achievement, or other business-domain table exists in M1. Future schema changes must append a new versioned migration; existing migration definitions are immutable.
