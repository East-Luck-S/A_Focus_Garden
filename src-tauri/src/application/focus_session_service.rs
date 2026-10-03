use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{OptionalExtension, Row, TransactionBehavior};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        focus_session::{FocusSession, RecoveryResolution, SessionError, SessionStatus},
        plant_bundle::validate_bundle,
        plant_generator::{
            generate_plant, resolve_seed, PlantGenerationInput, PlantGenome,
            PLANT_GENERATOR_VERSION,
        },
    },
    infrastructure::{
        focus_session_repository::{FocusSessionRepository, SessionRepositoryError},
        persistence::{Database, PersistenceError},
    },
};

pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now_millis(&self) -> Result<i64, FocusSessionServiceError>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> Result<i64, FocusSessionServiceError> {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| FocusSessionServiceError::TimeValueInvalid)?
            .as_millis();
        i64::try_from(millis).map_err(|_| FocusSessionServiceError::TimeValueInvalid)
    }
}

#[derive(Debug)]
pub struct FocusSessionService {
    database: Database,
    repository: FocusSessionRepository,
    clock: Arc<dyn Clock>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusSessionSnapshot {
    pub id: String,
    pub planned_duration_ms: i64,
    pub started_at: i64,
    pub target_end_at: i64,
    pub paused_at: Option<i64>,
    pub total_paused_ms: i64,
    pub completed_at: Option<i64>,
    pub status: SessionStatus,
    pub pause_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub remaining_ms: i64,
    pub observed_at: i64,
    pub plant_seed: String,
    pub generator_version: i64,
    pub plant_genome: PlantGenome,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GardenPlantSummary {
    pub id: String,
    pub planted_at: i64,
    pub generator_version: i64,
    pub plant_genome: PlantGenome,
    pub source_planned_duration_ms: i64,
    pub source_pause_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GardenCursor {
    pub planted_at: i64,
    pub id: String,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListGardenPlantsRequest {
    pub cursor: Option<GardenCursor>,
    pub limit: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GardenPlantPage {
    pub items: Vec<GardenPlantSummary>,
    pub next_cursor: Option<GardenCursor>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GardenOverview {
    pub plant_count: i64,
    pub total_planned_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GardenPlantDetail {
    pub id: String,
    pub planted_at: i64,
    pub plant_seed: String,
    pub generator_version: i64,
    pub plant_genome: PlantGenome,
    pub source_session: GardenSourceSession,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GardenSourceSession {
    pub id: String,
    pub planned_duration_ms: i64,
    pub started_at: i64,
    pub target_end_at: i64,
    pub completed_at: i64,
    pub total_paused_ms: i64,
    pub pause_count: i64,
}

const GARDEN_FIRST_PAGE_SQL: &str =
    "SELECT p.id, p.planted_at, p.generator_version, p.plant_genome_json,
            s.planned_duration_ms, s.pause_count
     FROM plants p JOIN sessions s ON s.id = p.source_session_id
     ORDER BY p.planted_at DESC, p.id DESC LIMIT ?1";

const GARDEN_CURSOR_PAGE_SQL: &str =
    "SELECT p.id, p.planted_at, p.generator_version, p.plant_genome_json,
            s.planned_duration_ms, s.pause_count
     FROM plants p JOIN sessions s ON s.id = p.source_session_id
     WHERE (p.planted_at, p.id) < (?1, ?2)
     ORDER BY p.planted_at DESC, p.id DESC LIMIT ?3";

const GARDEN_OVERVIEW_SQL: &str = "SELECT COUNT(*), COALESCE(SUM(s.planned_duration_ms), 0)
     FROM plants p CROSS JOIN sessions s ON s.id = p.source_session_id";

impl FocusSessionService {
    pub fn new(database: Database) -> Self {
        Self::with_clock(database, Arc::new(SystemClock))
    }

    pub fn with_clock(database: Database, clock: Arc<dyn Clock>) -> Self {
        Self {
            database,
            repository: FocusSessionRepository,
            clock,
        }
    }

    pub fn recover_active_sessions_on_launch(&self) -> Result<(), FocusSessionServiceError> {
        let now = self.now()?;
        let mut connection = self.database.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(PersistenceError::Transaction)?;
        if let Some(mut session) = self.repository.find_current(&transaction)? {
            self.hydrate_legacy_generation(&transaction, &mut session)?;
            if session.status == SessionStatus::Active {
                session.mark_recovery_required(now)?;
                self.repository.update(&transaction, &session)?;
            }
        }
        transaction
            .commit()
            .map_err(PersistenceError::Transaction)?;
        Ok(())
    }

    pub fn get_current(&self) -> Result<Option<FocusSessionSnapshot>, FocusSessionServiceError> {
        self.read_current()
    }

    pub fn start(
        &self,
        planned_duration_ms: i64,
    ) -> Result<FocusSessionSnapshot, FocusSessionServiceError> {
        let now = self.now()?;
        let mut connection = self.database.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(PersistenceError::Transaction)?;
        let mut session =
            FocusSession::start(Uuid::new_v4().to_string(), planned_duration_ms, now)?;
        self.assign_generated_preview(&mut session)?;
        self.repository.insert(&transaction, &session)?;
        transaction
            .commit()
            .map_err(PersistenceError::Transaction)?;
        self.snapshot(session, now)
    }

    pub fn pause(&self, id: &str) -> Result<FocusSessionSnapshot, FocusSessionServiceError> {
        self.change(id, |session, now| session.pause(now))
    }
    pub fn resume(&self, id: &str) -> Result<FocusSessionSnapshot, FocusSessionServiceError> {
        self.change(id, |session, now| session.resume(now))
    }
    pub fn abandon(&self, id: &str) -> Result<FocusSessionSnapshot, FocusSessionServiceError> {
        self.change(id, |session, now| session.abandon(now))
    }
    pub fn resolve_recovery(
        &self,
        id: &str,
        resolution: RecoveryResolution,
    ) -> Result<FocusSessionSnapshot, FocusSessionServiceError> {
        self.change(id, |session, now| session.resolve_recovery(resolution, now))
    }

    /// The only completion path: its session transition and immutable plant
    /// receipt share one IMMEDIATE SQLite transaction.
    pub fn finalize_focus_session(
        &self,
        id: &str,
    ) -> Result<GardenPlantDetail, FocusSessionServiceError> {
        let now = self.now()?;
        let mut connection = self.database.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(PersistenceError::Transaction)?;
        let mut session = self.repository.find_by_id(&transaction, id)?;
        if session.status == SessionStatus::Completed {
            let detail = read_garden_plant(&transaction, id)?
                .ok_or(FocusSessionServiceError::DataSafetyBlocker)?;
            transaction
                .commit()
                .map_err(PersistenceError::Transaction)?;
            return Ok(detail);
        }
        if session.status != SessionStatus::Active {
            return Err(FocusSessionServiceError::Domain(
                SessionError::InvalidTransition,
            ));
        }
        let bundle = bundle_from_session(&session)?;
        session.complete(now)?;
        self.repository.update(&transaction, &session)?;
        transaction.execute(
            "INSERT INTO plants (id, source_session_id, plant_seed, generator_version, plant_genome_json, planted_at) VALUES (?1, ?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![session.id, bundle.seed, bundle.generator_version, bundle.genome_json, session.completed_at],
        ).map_err(PersistenceError::DatabaseOperation)?;
        let detail = read_garden_plant(&transaction, id)?
            .ok_or(FocusSessionServiceError::DataSafetyBlocker)?;
        transaction
            .commit()
            .map_err(PersistenceError::Transaction)?;
        Ok(detail)
    }

    pub fn list_garden_plants_page(
        &self,
        request: ListGardenPlantsRequest,
    ) -> Result<GardenPlantPage, FocusSessionServiceError> {
        if !(1..=50).contains(&request.limit)
            || request
                .cursor
                .as_ref()
                .is_some_and(|cursor| cursor.planted_at < 0 || cursor.id.trim().is_empty())
        {
            return Err(FocusSessionServiceError::GardenQueryInvalid);
        }
        let connection = self.database.open_connection()?;
        let cursor = request.cursor.as_ref();
        let mut statement = connection
            .prepare(if cursor.is_some() {
                GARDEN_CURSOR_PAGE_SQL
            } else {
                GARDEN_FIRST_PAGE_SQL
            })
            .map_err(PersistenceError::DatabaseOperation)?;
        let rows = match cursor {
            Some(cursor) => statement
                .query_map(
                    rusqlite::params![cursor.planted_at, cursor.id, request.limit + 1],
                    decode_garden_summary,
                )
                .map_err(PersistenceError::DatabaseOperation)?
                .collect::<Result<Vec<_>, _>>(),
            None => statement
                .query_map(rusqlite::params![request.limit + 1], decode_garden_summary)
                .map_err(PersistenceError::DatabaseOperation)?
                .collect::<Result<Vec<_>, _>>(),
        }
        .map_err(|_| FocusSessionServiceError::DataSafetyBlocker)?;
        let mut items = rows;
        let has_more = items.len() > request.limit as usize;
        if has_more {
            items.pop();
        }
        let next_cursor = if has_more {
            items.last().map(|item| GardenCursor {
                planted_at: item.planted_at,
                id: item.id.clone(),
            })
        } else {
            None
        };
        Ok(GardenPlantPage { items, next_cursor })
    }

    pub fn get_garden_overview(&self) -> Result<GardenOverview, FocusSessionServiceError> {
        let connection = self.database.open_connection()?;
        connection
            .query_row(GARDEN_OVERVIEW_SQL, [], |row| {
                Ok(GardenOverview {
                    plant_count: row.get(0)?,
                    total_planned_duration_ms: row.get(1)?,
                })
            })
            .map_err(|error| PersistenceError::DatabaseOperation(error).into())
    }

    pub fn get_garden_plant(
        &self,
        id: &str,
    ) -> Result<GardenPlantDetail, FocusSessionServiceError> {
        let connection = self.database.open_connection()?;
        read_garden_plant(&connection, id)?.ok_or(FocusSessionServiceError::PlantNotFound)
    }

    fn change<F>(
        &self,
        id: &str,
        operation: F,
    ) -> Result<FocusSessionSnapshot, FocusSessionServiceError>
    where
        F: FnOnce(&mut FocusSession, i64) -> Result<(), SessionError>,
    {
        let now = self.now()?;
        let mut connection = self.database.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(PersistenceError::Transaction)?;
        let mut session = self.repository.find_by_id(&transaction, id)?;
        if !matches!(
            session.status,
            SessionStatus::Completed | SessionStatus::Abandoned
        ) {
            self.hydrate_legacy_generation(&transaction, &mut session)?;
        }
        operation(&mut session, now)?;
        self.repository.update(&transaction, &session)?;
        transaction
            .commit()
            .map_err(PersistenceError::Transaction)?;
        self.snapshot(session, now)
    }

    fn read_current(&self) -> Result<Option<FocusSessionSnapshot>, FocusSessionServiceError> {
        let now = self.now()?;
        let mut connection = self.database.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(PersistenceError::Transaction)?;
        let mut session = self.repository.find_current(&transaction)?;
        if let Some(value) = session.as_mut() {
            self.hydrate_legacy_generation(&transaction, value)?;
        }
        transaction
            .commit()
            .map_err(PersistenceError::Transaction)?;
        session.map(|value| self.snapshot(value, now)).transpose()
    }

    fn snapshot(
        &self,
        session: FocusSession,
        observed_at: i64,
    ) -> Result<FocusSessionSnapshot, FocusSessionServiceError> {
        let remaining_ms = session.remaining_at(observed_at)?;
        let preview = preview_from_session(&session)?;
        Ok(FocusSessionSnapshot {
            id: session.id,
            planned_duration_ms: session.planned_duration_ms,
            started_at: session.started_at,
            target_end_at: session.target_end_at,
            paused_at: session.paused_at,
            total_paused_ms: session.total_paused_ms,
            completed_at: session.completed_at,
            status: session.status,
            pause_count: session.pause_count,
            created_at: session.created_at,
            updated_at: session.updated_at,
            remaining_ms,
            observed_at,
            plant_seed: preview.seed,
            generator_version: preview.generator_version,
            plant_genome: preview.genome,
        })
    }

    fn assign_generated_preview(
        &self,
        session: &mut FocusSession,
    ) -> Result<(), FocusSessionServiceError> {
        let input = PlantGenerationInput {
            session_id: session.id.clone(),
            planned_duration_ms: session.planned_duration_ms,
            started_at: session.started_at,
            generator_version: PLANT_GENERATOR_VERSION,
        };
        let seed =
            resolve_seed(&input).map_err(|_| FocusSessionServiceError::GenerationDataInvalid)?;
        let genome = generate_plant(seed, PLANT_GENERATOR_VERSION)
            .map_err(|_| FocusSessionServiceError::GenerationDataInvalid)?;
        session.plant_seed = Some(format!("{seed:016x}"));
        session.generator_version = Some(i64::from(PLANT_GENERATOR_VERSION));
        session.plant_genome_json = Some(
            serde_json::to_string(&genome)
                .map_err(|_| FocusSessionServiceError::GenerationDataInvalid)?,
        );
        Ok(())
    }

    /// Hydrates only an M2-era all-null *open* session. A partially-written
    /// bundle is corruption, not a cue to guess or regenerate its identity.
    fn hydrate_legacy_generation(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        session: &mut FocusSession,
    ) -> Result<(), FocusSessionServiceError> {
        match (
            &session.plant_seed,
            session.generator_version,
            &session.plant_genome_json,
        ) {
            (None, None, None) => {
                self.assign_generated_preview(session)?;
                self.repository.update_generation(transaction, session)?;
                Ok(())
            }
            (Some(_), Some(_), Some(_)) => {
                preview_from_session(session)?;
                Ok(())
            }
            _ => Err(FocusSessionServiceError::GenerationDataInvalid),
        }
    }

    fn now(&self) -> Result<i64, FocusSessionServiceError> {
        self.clock.now_millis()
    }
}

fn bundle_from_session(
    session: &FocusSession,
) -> Result<crate::domain::plant_bundle::PlantBundle, FocusSessionServiceError> {
    match (
        &session.plant_seed,
        session.generator_version,
        &session.plant_genome_json,
    ) {
        (Some(seed), Some(version), Some(json)) => validate_bundle(seed, version, json)
            .map_err(|_| FocusSessionServiceError::DataSafetyBlocker),
        _ => Err(FocusSessionServiceError::DataSafetyBlocker),
    }
}

fn decode_garden_summary(row: &Row<'_>) -> rusqlite::Result<GardenPlantSummary> {
    let genome_json: String = row.get(3)?;
    let genome: PlantGenome =
        serde_json::from_str(&genome_json).map_err(|_| rusqlite::Error::InvalidQuery)?;
    let generator_version: i64 = row.get(2)?;
    let planned_duration: i64 = row.get(4)?;
    let pause_count: i64 = row.get(5)?;
    if genome.generator_version as i64 != generator_version
        || planned_duration <= 0
        || pause_count < 0
    {
        return Err(rusqlite::Error::InvalidQuery);
    }
    Ok(GardenPlantSummary {
        id: row.get(0)?,
        planted_at: row.get(1)?,
        generator_version,
        plant_genome: genome,
        source_planned_duration_ms: planned_duration,
        source_pause_count: pause_count,
    })
}

fn read_garden_plant(
    connection: &rusqlite::Connection,
    id: &str,
) -> Result<Option<GardenPlantDetail>, FocusSessionServiceError> {
    let detail = connection.query_row(
        "SELECT p.id, p.planted_at, p.plant_seed, p.generator_version, p.plant_genome_json,
                s.id, s.planned_duration_ms, s.started_at, s.target_end_at, s.completed_at, s.total_paused_ms, s.pause_count,
                s.status, s.plant_seed, s.generator_version, s.plant_genome_json
         FROM plants p JOIN sessions s ON s.id = p.source_session_id WHERE p.id = ?1", [id], |row| {
            let genome_json: String = row.get(4)?;
            let genome: PlantGenome = serde_json::from_str(&genome_json).map_err(|error| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error)))?;
            let status: String = row.get(12)?; let completed: Option<i64> = row.get(9)?;
            let seed: String = row.get(2)?; let version: i64 = row.get(3)?;
            let session_seed: Option<String> = row.get(13)?; let session_version: Option<i64> = row.get(14)?; let session_json: Option<String> = row.get(15)?;
            if status != "COMPLETED" || completed != Some(row.get(1)?) || session_seed.as_deref() != Some(seed.as_str()) || session_version != Some(version) || session_json.as_deref() != Some(genome_json.as_str()) || validate_bundle(&seed, version, &genome_json).is_err() { return Err(rusqlite::Error::InvalidQuery); }
            Ok(GardenPlantDetail { id: row.get(0)?, planted_at: row.get(1)?, plant_seed: seed, generator_version: version, plant_genome: genome, source_session: GardenSourceSession { id: row.get(5)?, planned_duration_ms: row.get(6)?, started_at: row.get(7)?, target_end_at: row.get(8)?, completed_at: completed.expect("checked"), total_paused_ms: row.get(10)?, pause_count: row.get(11)? } })
        }).optional().map_err(|error| match error {
            rusqlite::Error::InvalidQuery | rusqlite::Error::FromSqlConversionFailure(..) => FocusSessionServiceError::DataSafetyBlocker,
            other => FocusSessionServiceError::Persistence(PersistenceError::DatabaseOperation(other)),
        })?;
    Ok(detail)
}

#[derive(Debug, Error)]
pub enum FocusSessionServiceError {
    #[error("the garden query is invalid")]
    GardenQueryInvalid,
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error(transparent)]
    Domain(#[from] SessionError),
    #[error(transparent)]
    Repository(#[from] SessionRepositoryError),
    #[error("a time value is invalid")]
    TimeValueInvalid,
    #[error("the persisted plant-generation bundle is invalid")]
    GenerationDataInvalid,
    #[error("persisted transactional data cannot be safely reconciled")]
    DataSafetyBlocker,
    #[error("the garden plant was not found")]
    PlantNotFound,
}

#[derive(Debug)]
struct PreviewBundle {
    seed: String,
    generator_version: i64,
    genome: PlantGenome,
}

fn preview_from_session(session: &FocusSession) -> Result<PreviewBundle, FocusSessionServiceError> {
    let (seed, generator_version, genome_json) = match (
        &session.plant_seed,
        session.generator_version,
        &session.plant_genome_json,
    ) {
        (Some(seed), Some(version), Some(json)) => (seed, version, json),
        _ => return Err(FocusSessionServiceError::GenerationDataInvalid),
    };
    if seed.len() != 16
        || !seed
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
        || u64::from_str_radix(seed, 16).is_err()
        || generator_version != i64::from(PLANT_GENERATOR_VERSION)
    {
        return Err(FocusSessionServiceError::GenerationDataInvalid);
    }
    let genome: PlantGenome = serde_json::from_str(genome_json)
        .map_err(|_| FocusSessionServiceError::GenerationDataInvalid)?;
    if genome.generator_version != PLANT_GENERATOR_VERSION {
        return Err(FocusSessionServiceError::GenerationDataInvalid);
    }
    Ok(PreviewBundle {
        seed: seed.clone(),
        generator_version,
        genome,
    })
}

#[cfg(test)]
pub mod test_support {
    use super::{Clock, FocusSessionServiceError};
    use std::sync::atomic::{AtomicI64, Ordering};

    #[derive(Debug)]
    pub struct FakeClock(pub AtomicI64);
    impl FakeClock {
        pub fn new(now: i64) -> Self {
            Self(AtomicI64::new(now))
        }
        pub fn set(&self, now: i64) {
            self.0.store(now, Ordering::SeqCst);
        }
    }
    impl Clock for FakeClock {
        fn now_millis(&self) -> Result<i64, FocusSessionServiceError> {
            Ok(self.0.load(Ordering::SeqCst))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::tempdir;

    use super::{
        test_support::FakeClock, FocusSessionService, FocusSessionServiceError,
        ListGardenPlantsRequest, GARDEN_CURSOR_PAGE_SQL, GARDEN_OVERVIEW_SQL,
    };
    use crate::{
        domain::{
            focus_session::{RecoveryResolution, SessionError, SessionStatus},
            plant_generator::{generate_plant, PLANT_GENERATOR_VERSION},
        },
        infrastructure::{focus_session_repository::SessionRepositoryError, persistence::Database},
    };

    fn service(
        now: i64,
    ) -> (
        tempfile::TempDir,
        Arc<FakeClock>,
        FocusSessionService,
        Database,
    ) {
        let directory = tempdir().unwrap();
        let database = Database::new(directory.path().join("sessions.sqlite3"));
        database.initialize().unwrap();
        let clock = Arc::new(FakeClock::new(now));
        let service = FocusSessionService::with_clock(database.clone(), clock.clone());
        (directory, clock, service, database)
    }

    fn seed_completed_plants(
        service: &FocusSessionService,
        clock: &FakeClock,
        count: usize,
    ) -> Vec<String> {
        let mut ids = Vec::with_capacity(count);
        for index in 0..count {
            let started_at = 1_000_000 + index as i64 * 240_000;
            clock.set(started_at);
            let started = service.start(60_000 + (index as i64 % 3) * 60_000).unwrap();
            clock.set(started.target_end_at);
            ids.push(service.finalize_focus_session(&started.id).unwrap().id);
        }
        ids
    }

    #[test]
    fn garden_pages_are_keyset_ordered_and_overview_is_exact() {
        let (_directory, clock, service, _database) = service(1_000_000);
        let plant_count = 240_usize;
        assert!(service
            .list_garden_plants_page(ListGardenPlantsRequest {
                cursor: None,
                limit: 24
            })
            .unwrap()
            .items
            .is_empty());
        let ids = seed_completed_plants(&service, &clock, plant_count);
        let mut cursor = None;
        let mut collected = Vec::new();
        loop {
            let page = service
                .list_garden_plants_page(ListGardenPlantsRequest { cursor, limit: 24 })
                .unwrap();
            assert!(page.items.len() <= 24);
            collected.extend(page.items.iter().map(|item| item.id.clone()));
            if page.next_cursor.is_none() {
                break;
            }
            cursor = page.next_cursor;
        }
        assert_eq!(collected.len(), plant_count);
        assert_eq!(
            collected
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            plant_count
        );
        assert_eq!(collected.first(), ids.last());
        let overview = service.get_garden_overview().unwrap();
        assert_eq!(overview.plant_count, plant_count as i64);
        assert_eq!(
            overview.total_planned_duration_ms,
            (0..plant_count)
                .map(|index| 60_000_i64 + (index % 3) as i64 * 60_000)
                .sum::<i64>()
        );
    }

    #[test]
    fn garden_queries_keep_the_existing_indexes_on_their_bounded_paths() {
        let (_directory, _clock, _service, database) = service(1_000_000);
        let connection = database.open_connection().unwrap();

        let cursor_plan = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {GARDEN_CURSOR_PAGE_SQL}"))
            .unwrap()
            .query_map(rusqlite::params![1_000_i64, "cursor-id", 25_i64], |row| {
                row.get::<_, String>(3)
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(cursor_plan
            .iter()
            .any(|detail| detail.contains("SEARCH p USING INDEX plants_planted_at_id")));
        assert!(!cursor_plan
            .iter()
            .any(|detail| detail.contains("SCAN p") || detail.contains("TEMP B-TREE")));

        let overview_plan = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {GARDEN_OVERVIEW_SQL}"))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(overview_plan
            .iter()
            .any(|detail| detail.starts_with("SCAN p")));
        assert!(overview_plan
            .iter()
            .any(|detail| detail.starts_with("SEARCH s")));
        assert!(!overview_plan
            .iter()
            .any(|detail| detail.starts_with("SCAN s") || detail.contains("TEMP B-TREE")));
    }

    #[test]
    fn garden_page_rejects_invalid_limits_and_cursors() {
        let (_directory, _clock, service, _database) = service(1_000_000);
        for request in [
            ListGardenPlantsRequest {
                cursor: None,
                limit: 0,
            },
            ListGardenPlantsRequest {
                cursor: None,
                limit: 51,
            },
            ListGardenPlantsRequest {
                cursor: Some(super::GardenCursor {
                    planted_at: -1,
                    id: "id".into(),
                }),
                limit: 1,
            },
            ListGardenPlantsRequest {
                cursor: Some(super::GardenCursor {
                    planted_at: 0,
                    id: " ".into(),
                }),
                limit: 1,
            },
        ] {
            assert!(matches!(
                service.list_garden_plants_page(request),
                Err(FocusSessionServiceError::GardenQueryInvalid)
            ));
        }
    }

    #[test]
    fn garden_keyset_uses_id_as_a_stable_same_timestamp_tiebreaker() {
        let (_directory, _clock, service, database) = service(1_000_000);
        let connection = database.open_connection().unwrap();
        for id in ["tie-a", "tie-b"] {
            let input = crate::domain::plant_generator::PlantGenerationInput {
                session_id: id.into(),
                planned_duration_ms: 60_000,
                started_at: 1_000,
                generator_version: PLANT_GENERATOR_VERSION,
            };
            let seed = crate::domain::plant_generator::resolve_seed(&input).unwrap();
            let genome =
                crate::domain::plant_generator::generate_plant(seed, PLANT_GENERATOR_VERSION)
                    .unwrap();
            let seed = format!("{seed:016x}");
            let json = serde_json::to_string(&genome).unwrap();
            connection.execute("INSERT INTO sessions (id,planned_duration_ms,started_at,target_end_at,total_paused_ms,completed_at,status,pause_count,created_at,updated_at,plant_seed,generator_version,plant_genome_json) VALUES (?1,60000,1000,61000,0,61000,'COMPLETED',0,1000,61000,?2,1,?3)", rusqlite::params![id, seed, json]).unwrap();
            connection.execute("INSERT INTO plants (id,source_session_id,plant_seed,generator_version,plant_genome_json,planted_at) VALUES (?1,?1,?2,1,?3,61000)", rusqlite::params![id, seed, json]).unwrap();
        }
        let first = service
            .list_garden_plants_page(ListGardenPlantsRequest {
                cursor: None,
                limit: 1,
            })
            .unwrap();
        assert_eq!(first.items[0].id, "tie-b");
        let second = service
            .list_garden_plants_page(ListGardenPlantsRequest {
                cursor: first.next_cursor,
                limit: 1,
            })
            .unwrap();
        assert_eq!(second.items[0].id, "tie-a");
        assert_eq!(second.next_cursor, None);
    }

    #[test]
    fn only_one_open_session_is_allowed_and_state_survives_restart() {
        let (_directory, clock, service, database) = service(1_000_000);
        let first = service.start(60_000).unwrap();
        let duplicate = service.start(60_000);
        assert!(matches!(
            duplicate,
            Err(FocusSessionServiceError::Repository(
                SessionRepositoryError::OpenSessionExists
            ))
        ));
        let restarted = FocusSessionService::with_clock(database, clock);
        assert_eq!(restarted.get_current().unwrap().unwrap().id, first.id);
    }

    #[test]
    fn launch_converts_active_to_recovery_but_keeps_paused() {
        let (_directory, clock, service, database) = service(1_000_000);
        let active = service.start(60_000).unwrap();
        clock.set(1_001_000);
        let restarted = FocusSessionService::with_clock(database.clone(), clock.clone());
        restarted.recover_active_sessions_on_launch().unwrap();
        let recovery = restarted.get_current().unwrap().unwrap();
        assert_eq!(recovery.id, active.id);
        assert_eq!(recovery.status, SessionStatus::RecoveryRequired);
        let original_target = recovery.target_end_at;
        clock.set(1_002_000);
        let continued = restarted
            .resolve_recovery(&recovery.id, RecoveryResolution::Continue)
            .unwrap();
        assert_eq!(continued.status, SessionStatus::Active);
        assert_eq!(continued.target_end_at, original_target);

        let abandoned = restarted.abandon(&continued.id).unwrap();
        assert_eq!(abandoned.status, SessionStatus::Abandoned);
        let paused = restarted.start(60_000).unwrap();
        restarted.pause(&paused.id).unwrap();
        let third_start = FocusSessionService::with_clock(database, clock);
        third_start.recover_active_sessions_on_launch().unwrap();
        assert_eq!(
            third_start.get_current().unwrap().unwrap().status,
            SessionStatus::Paused
        );
    }

    #[test]
    fn launch_recovery_rejects_clock_rollback_without_mutating_active_session() {
        let (_directory, clock, service, database) = service(1_000_000);
        let active = service.start(60_000).unwrap();
        clock.set(999_999);
        let restarted = FocusSessionService::with_clock(database, clock.clone());

        assert!(matches!(
            restarted.recover_active_sessions_on_launch(),
            Err(FocusSessionServiceError::Domain(
                crate::domain::focus_session::SessionError::TimeValueInvalid
            ))
        ));

        let current = restarted.get_current().unwrap().unwrap();
        assert_eq!(current.id, active.id);
        assert_eq!(current.status, SessionStatus::Active);
        assert_eq!(current.updated_at, active.updated_at);
    }

    #[test]
    fn start_persists_a_complete_u64_safe_preview_bundle_atomically() {
        let (_directory, _clock, service, database) = service(1_000_000);
        let started = service.start(60_000).unwrap();
        assert_eq!(started.plant_seed.len(), 16);
        assert!(started
            .plant_seed
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
        assert_eq!(started.generator_version, 1);
        let connection = database.open_connection().unwrap();
        let stored: (Option<String>, Option<i64>, Option<String>) = connection.query_row(
            "SELECT plant_seed, generator_version, plant_genome_json FROM sessions WHERE id = ?1",
            [&started.id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!(stored.0.as_deref(), Some(started.plant_seed.as_str()));
        assert_eq!(stored.1, Some(1));
        assert_eq!(
            stored.2.as_deref(),
            Some(
                serde_json::to_string(&started.plant_genome)
                    .unwrap()
                    .as_str()
            )
        );
        let plants: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'plants'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(plants, 1);
    }

    #[test]
    fn failed_start_rolls_back_without_leaving_a_partial_session() {
        let (_directory, _clock, service, database) = service(1_000_000);
        assert!(matches!(
            service.start(59_999),
            Err(FocusSessionServiceError::Domain(
                SessionError::InvalidDuration
            ))
        ));
        let connection = database.open_connection().unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn legacy_all_null_open_session_is_hydrated_before_recovery() {
        let (_directory, clock, service, database) = service(1_000_000);
        let connection = database.open_connection().unwrap();
        connection.execute(
            "INSERT INTO sessions (id, planned_duration_ms, started_at, target_end_at, total_paused_ms, status, pause_count, created_at, updated_at)
             VALUES ('legacy-open', 60000, 1000000, 1060000, 0, 'ACTIVE', 0, 1000000, 1000000)", [],
        ).unwrap();
        clock.set(1_001_000);
        service.recover_active_sessions_on_launch().unwrap();
        let current = service.get_current().unwrap().unwrap();
        assert_eq!(current.status, SessionStatus::RecoveryRequired);
        assert_eq!(current.plant_seed.len(), 16);
        let stored: i64 = connection.query_row("SELECT COUNT(*) FROM sessions WHERE id = 'legacy-open' AND plant_seed IS NOT NULL AND generator_version = 1 AND plant_genome_json IS NOT NULL", [], |row| row.get(0)).unwrap();
        assert_eq!(stored, 1);
    }

    #[test]
    fn partial_generation_data_is_rejected_without_repairing_or_transitioning() {
        let (_directory, clock, service, database) = service(1_000_000);
        let started = service.start(60_000).unwrap();
        let connection = database.open_connection().unwrap();
        connection
            .execute_batch("DROP TRIGGER sessions_generation_bundle_update;")
            .unwrap();
        connection
            .execute(
                "UPDATE sessions SET plant_genome_json = NULL WHERE id = ?1",
                [&started.id],
            )
            .unwrap();
        clock.set(1_001_000);
        assert!(matches!(
            service.recover_active_sessions_on_launch(),
            Err(FocusSessionServiceError::GenerationDataInvalid)
        ));
        let row: (String, Option<String>) = connection
            .query_row(
                "SELECT status, plant_genome_json FROM sessions WHERE id = ?1",
                [&started.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(row.0, "ACTIVE");
        assert_eq!(row.1, None);
    }

    #[test]
    fn persisted_genome_is_reused_across_restart_not_regenerated() {
        let (_directory, clock, service, database) = service(1_000_000);
        let started = service.start(60_000).unwrap();
        let replacement = generate_plant(u64::MAX, PLANT_GENERATOR_VERSION).unwrap();
        assert_ne!(replacement, started.plant_genome);
        let replacement_json = serde_json::to_string(&replacement).unwrap();
        let connection = database.open_connection().unwrap();
        connection.execute(
            "UPDATE sessions SET plant_seed = 'ffffffffffffffff', generator_version = 1, plant_genome_json = ?1 WHERE id = ?2",
            (&replacement_json, &started.id),
        ).unwrap();
        clock.set(1_001_000);
        let restarted = FocusSessionService::with_clock(database, clock);
        restarted.recover_active_sessions_on_launch().unwrap();
        let loaded = restarted.get_current().unwrap().unwrap();
        assert_eq!(loaded.plant_seed, "ffffffffffffffff");
        assert_eq!(loaded.plant_genome, replacement);
    }

    #[test]
    fn active_pause_resume_and_recovery_growth_timer_edges_are_preserved() {
        let (_directory, clock, service, database) = service(1_000_000);
        let started = service.start(60_000).unwrap();
        clock.set(1_060_000);
        assert_eq!(service.get_current().unwrap().unwrap().remaining_ms, 0);
        clock.set(1_010_000);
        let paused = service.pause(&started.id).unwrap();
        clock.set(1_050_000);
        assert_eq!(
            service.get_current().unwrap().unwrap().remaining_ms,
            paused.remaining_ms
        );
        let resumed = service.resume(&started.id).unwrap();
        assert_eq!(resumed.status, SessionStatus::Active);
        let preview = resumed.plant_genome.clone();
        let restarted = FocusSessionService::with_clock(database, clock);
        restarted.recover_active_sessions_on_launch().unwrap();
        let recovery = restarted.get_current().unwrap().unwrap();
        assert_eq!(recovery.plant_genome, preview);
    }

    #[test]
    fn finalization_is_atomic_idempotent_and_creates_one_exact_plant() {
        let (_directory, clock, service, database) = service(1_000_000);
        let started = service.start(60_000).unwrap();
        assert!(matches!(
            service.finalize_focus_session(&started.id),
            Err(FocusSessionServiceError::Domain(
                SessionError::NotReadyToComplete
            ))
        ));
        clock.set(1_060_000);
        let first = service.finalize_focus_session(&started.id).unwrap();
        let second = service.finalize_focus_session(&started.id).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.plant_seed, started.plant_seed);
        let connection = database.open_connection().unwrap();
        let counts: (i64, i64) = connection.query_row("SELECT (SELECT COUNT(*) FROM sessions WHERE status = 'COMPLETED'), (SELECT COUNT(*) FROM plants)", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(counts, (1, 1));
        drop(connection);
        let restarted = FocusSessionService::with_clock(database, clock);
        let listed = restarted
            .list_garden_plants_page(ListGardenPlantsRequest {
                cursor: None,
                limit: 24,
            })
            .unwrap();
        assert_eq!(listed.items.len(), 1);
        assert_eq!(listed.items[0].id, first.id);
        assert_eq!(listed.items[0].plant_genome, first.plant_genome);
        assert_eq!(restarted.get_garden_plant(&first.id).unwrap(), first);
    }

    #[test]
    fn insert_failure_rolls_back_completion() {
        let (_directory, clock, service, database) = service(1_000_000);
        let started = service.start(60_000).unwrap();
        clock.set(1_060_000);
        let connection = database.open_connection().unwrap();
        connection.execute_batch("CREATE TRIGGER fail_plant BEFORE INSERT ON plants BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(service.finalize_focus_session(&started.id).is_err());
        let status: String = connection
            .query_row(
                "SELECT status FROM sessions WHERE id = ?1",
                [&started.id],
                |row| row.get(0),
            )
            .unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM plants", [], |row| row.get(0))
            .unwrap();
        assert_eq!(status, "ACTIVE");
        assert_eq!(count, 0);
    }
}
