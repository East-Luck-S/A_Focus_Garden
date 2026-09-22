//! Versioned, deterministic plant-genome generation.
//!
//! This module is deliberately a pure Rust-domain boundary: it has no clock,
//! persistence, renderer, Tauri, or frontend dependency. Its seed contract is
//! FNV-1a 64 over explicitly little-endian fields, followed by SplitMix64 for
//! generation. Both algorithms are fixed here so a saved seed remains replayable.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::focus_session::MINIMUM_FOCUS_DURATION_MS;

pub const PLANT_GENERATOR_VERSION: u16 = 1;

/// Stable data that is available at plant-generation time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlantGenerationInput {
    pub session_id: String,
    pub planned_duration_ms: i64,
    pub started_at: i64,
    pub generator_version: u16,
}

impl PlantGenerationInput {
    pub fn validate(&self) -> Result<(), PlantGenerationError> {
        if self.session_id.trim().is_empty() {
            return Err(PlantGenerationError::InvalidSessionId);
        }
        if self.planned_duration_ms < MINIMUM_FOCUS_DURATION_MS {
            return Err(PlantGenerationError::InvalidDuration);
        }
        if self.started_at < 0 {
            return Err(PlantGenerationError::InvalidStartedAt);
        }
        ensure_supported_version(self.generator_version)
    }
}

/// Resolves a replayable seed using FNV-1a 64-bit hashing.
///
/// Bytes are consumed in this fixed order: domain separator, generator version
/// as little-endian `u16`, session-id byte length as little-endian `u64`, UTF-8
/// session-id bytes, planned duration as little-endian `i64`, then start time as
/// little-endian `i64`.
pub fn resolve_seed(input: &PlantGenerationInput) -> Result<u64, PlantGenerationError> {
    input.validate()?;

    let mut hash = FNV_OFFSET_BASIS;
    hash_bytes(&mut hash, b"focus-garden.plant-seed.v1\0");
    hash_bytes(&mut hash, &input.generator_version.to_le_bytes());
    hash_bytes(&mut hash, &(input.session_id.len() as u64).to_le_bytes());
    hash_bytes(&mut hash, input.session_id.as_bytes());
    hash_bytes(&mut hash, &input.planned_duration_ms.to_le_bytes());
    hash_bytes(&mut hash, &input.started_at.to_le_bytes());
    Ok(hash)
}

/// Generates the versioned plant genome from a previously resolved seed.
pub fn generate_plant(seed: u64, version: u16) -> Result<PlantGenome, PlantGenerationError> {
    ensure_supported_version(version)?;
    let mut random = SplitMix64::new(seed);

    let morphology = MORPHOLOGIES[random.index(MORPHOLOGIES.len())];
    let stem = Stem {
        height: random.inclusive_u8(42, 128),
        thickness: random.inclusive_u8(3, 11),
        curvature: random.inclusive_i8(-24, 24),
        branch_count: random.inclusive_u8(1, 6),
    };

    let leaf_count = random.inclusive_u8(3, 9);
    let starts_on_left = random.next_bool();
    let mut leaves = Vec::with_capacity(usize::from(leaf_count));
    for index in 0..leaf_count {
        let is_left = if starts_on_left {
            index % 2 == 0
        } else {
            index % 2 != 0
        };
        leaves.push(Leaf {
            side: if is_left {
                LeafSide::Left
            } else {
                LeafSide::Right
            },
            position: 14 + index * 10,
            angle_degrees: if is_left {
                -random.inclusive_i16(20, 62)
            } else {
                random.inclusive_i16(20, 62)
            },
            size: random.inclusive_u8(12, 42),
            shape: LEAF_SHAPES[random.index(LEAF_SHAPES.len())],
        });
    }

    let flower = Flower {
        kind: FLOWER_KINDS[random.index(FLOWER_KINDS.len())],
        petal_count: random.inclusive_u8(4, 10),
        size: random.inclusive_u8(14, 38),
        tilt_degrees: random.inclusive_i8(-18, 18),
    };
    let palette = PALETTES[random.index(PALETTES.len())];
    let effects = Effects {
        glow: random.inclusive_u8(0, 100),
        dew_drops: random.inclusive_u8(0, 7),
        pollen: random.inclusive_u8(0, 15),
        breeze_offset: random.inclusive_i8(-12, 12),
    };

    Ok(PlantGenome {
        generator_version: version,
        morphology,
        stem,
        leaves,
        flower,
        palette,
        effects,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlantGenome {
    pub generator_version: u16,
    pub morphology: Morphology,
    pub stem: Stem,
    pub leaves: Vec<Leaf>,
    pub flower: Flower,
    pub palette: Palette,
    pub effects: Effects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Morphology {
    Fern,
    Succulent,
    Wildflower,
    Vine,
    Shrub,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stem {
    pub height: u8,
    pub thickness: u8,
    pub curvature: i8,
    pub branch_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leaf {
    pub side: LeafSide,
    pub position: u8,
    pub angle_degrees: i16,
    pub size: u8,
    pub shape: LeafShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeafSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeafShape {
    Oval,
    Lanceolate,
    Serrated,
    Round,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flower {
    pub kind: FlowerKind,
    pub petal_count: u8,
    pub size: u8,
    pub tilt_degrees: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FlowerKind {
    Bloom,
    Bell,
    Star,
    Cluster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub stem: Rgb,
    pub leaf_primary: Rgb,
    pub leaf_secondary: Rgb,
    pub flower_primary: Rgb,
    pub flower_secondary: Rgb,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effects {
    pub glow: u8,
    pub dew_drops: u8,
    pub pollen: u8,
    pub breeze_offset: i8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlantGenerationError {
    #[error("the session id is invalid")]
    InvalidSessionId,
    #[error("the planned duration is invalid")]
    InvalidDuration,
    #[error("the session start time is invalid")]
    InvalidStartedAt,
    #[error("plant generator version {requested} is unsupported")]
    UnsupportedVersion { requested: u16 },
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

const MORPHOLOGIES: [Morphology; 5] = [
    Morphology::Fern,
    Morphology::Succulent,
    Morphology::Wildflower,
    Morphology::Vine,
    Morphology::Shrub,
];
const LEAF_SHAPES: [LeafShape; 4] = [
    LeafShape::Oval,
    LeafShape::Lanceolate,
    LeafShape::Serrated,
    LeafShape::Round,
];
const FLOWER_KINDS: [FlowerKind; 4] = [
    FlowerKind::Bloom,
    FlowerKind::Bell,
    FlowerKind::Star,
    FlowerKind::Cluster,
];
const PALETTES: [Palette; 4] = [
    Palette {
        stem: Rgb {
            red: 74,
            green: 103,
            blue: 65,
        },
        leaf_primary: Rgb {
            red: 92,
            green: 141,
            blue: 78,
        },
        leaf_secondary: Rgb {
            red: 155,
            green: 189,
            blue: 89,
        },
        flower_primary: Rgb {
            red: 242,
            green: 145,
            blue: 165,
        },
        flower_secondary: Rgb {
            red: 255,
            green: 220,
            blue: 139,
        },
    },
    Palette {
        stem: Rgb {
            red: 66,
            green: 92,
            blue: 60,
        },
        leaf_primary: Rgb {
            red: 57,
            green: 121,
            blue: 91,
        },
        leaf_secondary: Rgb {
            red: 112,
            green: 173,
            blue: 132,
        },
        flower_primary: Rgb {
            red: 120,
            green: 158,
            blue: 232,
        },
        flower_secondary: Rgb {
            red: 218,
            green: 226,
            blue: 255,
        },
    },
    Palette {
        stem: Rgb {
            red: 105,
            green: 83,
            blue: 56,
        },
        leaf_primary: Rgb {
            red: 113,
            green: 132,
            blue: 64,
        },
        leaf_secondary: Rgb {
            red: 185,
            green: 190,
            blue: 98,
        },
        flower_primary: Rgb {
            red: 245,
            green: 177,
            blue: 72,
        },
        flower_secondary: Rgb {
            red: 255,
            green: 238,
            blue: 174,
        },
    },
    Palette {
        stem: Rgb {
            red: 79,
            green: 80,
            blue: 91,
        },
        leaf_primary: Rgb {
            red: 85,
            green: 115,
            blue: 109,
        },
        leaf_secondary: Rgb {
            red: 142,
            green: 172,
            blue: 156,
        },
        flower_primary: Rgb {
            red: 181,
            green: 126,
            blue: 215,
        },
        flower_secondary: Rgb {
            red: 240,
            green: 206,
            blue: 255,
        },
    },
];

fn ensure_supported_version(version: u16) -> Result<(), PlantGenerationError> {
    if version == PLANT_GENERATOR_VERSION {
        Ok(())
    } else {
        Err(PlantGenerationError::UnsupportedVersion { requested: version })
    }
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

/// SplitMix64 with an explicitly specified wrapping-integer implementation.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn index(&mut self, upper_exclusive: usize) -> usize {
        debug_assert!(upper_exclusive > 0);
        (self.next_u64() % upper_exclusive as u64) as usize
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    fn inclusive_u8(&mut self, minimum: u8, maximum: u8) -> u8 {
        debug_assert!(minimum <= maximum);
        minimum + (self.next_u64() % u64::from(maximum - minimum + 1)) as u8
    }

    fn inclusive_i8(&mut self, minimum: i8, maximum: i8) -> i8 {
        debug_assert!(minimum <= maximum);
        minimum + (self.next_u64() % (i16::from(maximum) - i16::from(minimum) + 1) as u64) as i8
    }

    fn inclusive_i16(&mut self, minimum: i16, maximum: i16) -> i16 {
        debug_assert!(minimum <= maximum);
        minimum + (self.next_u64() % (i32::from(maximum) - i32::from(minimum) + 1) as u64) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> PlantGenerationInput {
        PlantGenerationInput {
            session_id: "550e8400-e29b-41d4-a716-446655440000".into(),
            planned_duration_ms: 1_500_000,
            started_at: 1_725_000_123_456,
            generator_version: PLANT_GENERATOR_VERSION,
        }
    }

    #[test]
    fn same_input_resolves_the_same_seed_and_genome() {
        let first_seed = resolve_seed(&input()).unwrap();
        let second_seed = resolve_seed(&input()).unwrap();
        assert_eq!(first_seed, second_seed);
        assert_eq!(
            generate_plant(first_seed, PLANT_GENERATOR_VERSION).unwrap(),
            generate_plant(second_seed, PLANT_GENERATOR_VERSION).unwrap()
        );
    }

    #[test]
    fn same_seed_replays_the_same_genome() {
        let first = generate_plant(42, PLANT_GENERATOR_VERSION).unwrap();
        let second = generate_plant(42, PLANT_GENERATOR_VERSION).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn distinct_seeds_create_meaningful_variation() {
        let first = generate_plant(1, PLANT_GENERATOR_VERSION).unwrap();
        let second = generate_plant(2, PLANT_GENERATOR_VERSION).unwrap();
        assert_ne!(first, second);
        assert!(
            first.morphology != second.morphology
                || first.stem != second.stem
                || first.leaves != second.leaves
                || first.flower != second.flower
                || first.palette != second.palette
                || first.effects != second.effects
        );
    }

    #[test]
    fn boundary_values_are_supported() {
        let minimum = PlantGenerationInput {
            session_id: "a".into(),
            planned_duration_ms: MINIMUM_FOCUS_DURATION_MS,
            started_at: 0,
            generator_version: PLANT_GENERATOR_VERSION,
        };
        assert!(resolve_seed(&minimum).is_ok());
        assert!(generate_plant(u64::MIN, PLANT_GENERATOR_VERSION).is_ok());
        assert!(generate_plant(u64::MAX, PLANT_GENERATOR_VERSION).is_ok());
    }

    #[test]
    fn invalid_inputs_and_versions_are_rejected() {
        let mut invalid = input();
        invalid.session_id = " \t ".into();
        assert_eq!(
            resolve_seed(&invalid),
            Err(PlantGenerationError::InvalidSessionId)
        );
        invalid = input();
        invalid.planned_duration_ms = MINIMUM_FOCUS_DURATION_MS - 1;
        assert_eq!(
            resolve_seed(&invalid),
            Err(PlantGenerationError::InvalidDuration)
        );
        invalid = input();
        invalid.started_at = -1;
        assert_eq!(
            resolve_seed(&invalid),
            Err(PlantGenerationError::InvalidStartedAt)
        );
        invalid = input();
        invalid.generator_version = 2;
        assert_eq!(
            resolve_seed(&invalid),
            Err(PlantGenerationError::UnsupportedVersion { requested: 2 })
        );
        assert_eq!(
            generate_plant(7, 2),
            Err(PlantGenerationError::UnsupportedVersion { requested: 2 })
        );
    }

    #[test]
    fn genome_json_round_trip_is_lossless() {
        let genome = generate_plant(987_654_321, PLANT_GENERATOR_VERSION).unwrap();
        let json = serde_json::to_string(&genome).unwrap();
        assert_eq!(serde_json::from_str::<PlantGenome>(&json).unwrap(), genome);
    }

    #[test]
    fn golden_fixture_locks_v1_seed_and_genome() {
        let seed = resolve_seed(&input()).unwrap();
        let genome = generate_plant(seed, PLANT_GENERATOR_VERSION).unwrap();
        assert_eq!(seed, 12_238_341_369_376_252_549);
        assert_eq!(
            serde_json::to_string(&genome).unwrap(),
            r#"{"generator_version":1,"morphology":"SHRUB","stem":{"height":73,"thickness":3,"curvature":-21,"branch_count":4},"leaves":[{"side":"RIGHT","position":14,"angle_degrees":21,"size":20,"shape":"SERRATED"},{"side":"LEFT","position":24,"angle_degrees":-41,"size":40,"shape":"ROUND"},{"side":"RIGHT","position":34,"angle_degrees":61,"size":34,"shape":"OVAL"},{"side":"LEFT","position":44,"angle_degrees":-30,"size":30,"shape":"SERRATED"},{"side":"RIGHT","position":54,"angle_degrees":49,"size":22,"shape":"OVAL"},{"side":"LEFT","position":64,"angle_degrees":-51,"size":22,"shape":"LANCEOLATE"},{"side":"RIGHT","position":74,"angle_degrees":56,"size":24,"shape":"LANCEOLATE"},{"side":"LEFT","position":84,"angle_degrees":-43,"size":29,"shape":"SERRATED"}],"flower":{"kind":"STAR","petal_count":5,"size":33,"tilt_degrees":-16},"palette":{"stem":{"red":105,"green":83,"blue":56},"leaf_primary":{"red":113,"green":132,"blue":64},"leaf_secondary":{"red":185,"green":190,"blue":98},"flower_primary":{"red":245,"green":177,"blue":72},"flower_secondary":{"red":255,"green":238,"blue":174}},"effects":{"glow":55,"dew_drops":6,"pollen":1,"breeze_offset":-12}}"#
        );
    }
}
