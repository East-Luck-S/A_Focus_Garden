use crate::domain::plant_generator::{generate_plant, PlantGenome, PLANT_GENERATOR_VERSION};

/// The persisted generator identity is deliberately validated independently of
/// session state so migrations, finalization, and startup auditing agree about
/// what constitutes a plant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlantBundle {
    pub seed: String,
    pub generator_version: i64,
    pub genome_json: String,
    pub genome: PlantGenome,
}

pub fn validate_bundle(
    seed: &str,
    generator_version: i64,
    genome_json: &str,
) -> Result<PlantBundle, ()> {
    if seed.len() != 16
        || !seed
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(());
    }
    let seed_value = u64::from_str_radix(seed, 16).map_err(|_| ())?;
    let version = u16::try_from(generator_version).map_err(|_| ())?;
    if version != PLANT_GENERATOR_VERSION {
        return Err(());
    }
    let genome: PlantGenome = serde_json::from_str(genome_json).map_err(|_| ())?;
    if genome.generator_version != version {
        return Err(());
    }
    // JSON has to be a faithful generator output, not merely syntactically valid.
    if generate_plant(seed_value, version).map_err(|_| ())? != genome {
        return Err(());
    }
    Ok(PlantBundle {
        seed: seed.into(),
        generator_version,
        genome_json: genome_json.into(),
        genome,
    })
}
