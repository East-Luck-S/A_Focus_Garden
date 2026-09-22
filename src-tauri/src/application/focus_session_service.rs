use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::TransactionBehavior;
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        focus_session::{FocusSession, RecoveryResolution, SessionError, SessionStatus},
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

#[derive(Debug, Error)]
pub enum FocusSessionServiceError {
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

    use super::{test_support::FakeClock, FocusSessionService, FocusSessionServiceError};
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
        assert_eq!(plants, 0);
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
}
