# Shared

Small frontend utilities and shared presentation types belong here. Business persistence does not.

`plant/types.ts` mirrors the serialized Rust `PlantGenome` V1 DTO exactly, including
snake_case field names. It is a frontend contract, not a second generator.
