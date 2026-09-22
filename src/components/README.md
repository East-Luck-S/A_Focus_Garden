# Components

Reusable presentation components belong here. Components must not own authoritative business state.

`plant/PlantSvg` is a pure, accessible SVG projection of a supplied Rust-authoritative
`PlantGenome` plus a supplied growth value. It does not generate genomes, use native
commands, or own timer state, so focus flow and garden views can reuse it.
