# Architecture

## Platform boundary

Focus Garden is a Windows 10/11 desktop application. Tauri 2 supplies the native shell and Windows integration. The architecture does not include a web backend, Electron, Next.js, or a cross-platform product commitment.

## Responsibility boundary

```text
React / TypeScript                 Rust                         SQLite (future)
------------------                 ----                         ---------------
Presentation             ->        Authoritative domain  <->   Canonical source
Interaction                        Application services         of persistent truth
Read projections                   Transactions
SVG rendering                      Migrations
Transient UI state                 Backup / restore
                                   Windows integration
```

React components may render state, collect user intent, and manage transient interface state. They must not become the authoritative owner of focus-session or plant-domain state, and they must not access SQLite directly.

Rust will own authoritative business rules, session transitions, deterministic plant generation, transactions, persistence coordination, and native Windows integration as those milestones are approved.

SQLite is planned as the canonical persistent source of truth. It is deliberately absent in M0; no schema or persistence shortcut is established here. Browser `localStorage` and Zustand persistence must not be used for business persistence.

## Future plant renderer boundary

The planned SVG renderer is a React/TypeScript projection of an authoritative plant description produced by Rust. Rendering may interpret the description visually, but it must not generate or mutate plant identity, genome, rarity, or growth truth.

## Native command boundary

Tauri commands will be introduced only with their owning use cases. Command payloads form an explicit boundary between frontend intent/projections and Rust application services. The M0 shell exposes no product command.

## Runtime constraints

The formal runtime has no AI API, cloud backend, account, cloud sync, social system, payment, advertising, or telemetry. These are constraints, not merely missing M0 tasks.
