use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::TransactionBehavior;
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::focus_session::{FocusSession, RecoveryResolution, SessionError, SessionStatus},
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
        let session = FocusSession::start(Uuid::new_v4().to_string(), planned_duration_ms, now)?;
        let mut connection = self.database.open_connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(PersistenceError::Transaction)?;
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
            .transaction()
            .map_err(PersistenceError::Transaction)?;
        let session = self.repository.find_current(&transaction)?;
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
        })
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
        domain::focus_session::{RecoveryResolution, SessionStatus},
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
}
