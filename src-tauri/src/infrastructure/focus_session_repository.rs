use rusqlite::{params, OptionalExtension, Transaction};
use thiserror::Error;

use crate::domain::focus_session::{FocusSession, SessionError, SessionStatus};

#[derive(Debug, Default)]
pub struct FocusSessionRepository;

impl FocusSessionRepository {
    pub fn insert(
        &self,
        transaction: &Transaction<'_>,
        session: &FocusSession,
    ) -> Result<(), SessionRepositoryError> {
        transaction.execute("INSERT INTO sessions (id, planned_duration_ms, started_at, target_end_at, paused_at, total_paused_ms, completed_at, status, pause_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", params![session.id, session.planned_duration_ms, session.started_at, session.target_end_at, session.paused_at, session.total_paused_ms, session.completed_at, session.status.as_db(), session.pause_count, session.created_at, session.updated_at]).map_err(map_write_error)?;
        Ok(())
    }

    pub fn find_by_id(
        &self,
        transaction: &Transaction<'_>,
        id: &str,
    ) -> Result<FocusSession, SessionRepositoryError> {
        transaction.query_row("SELECT id, planned_duration_ms, started_at, target_end_at, paused_at, total_paused_ms, completed_at, status, pause_count, created_at, updated_at FROM sessions WHERE id = ?1", [id], decode_session).optional().map_err(map_read_error)?.ok_or(SessionRepositoryError::NotFound)
    }

    pub fn find_current(
        &self,
        transaction: &Transaction<'_>,
    ) -> Result<Option<FocusSession>, SessionRepositoryError> {
        transaction.query_row("SELECT id, planned_duration_ms, started_at, target_end_at, paused_at, total_paused_ms, completed_at, status, pause_count, created_at, updated_at FROM sessions WHERE status IN ('ACTIVE', 'PAUSED', 'RECOVERY_REQUIRED') ORDER BY created_at DESC LIMIT 1", [], decode_session).optional().map_err(map_read_error)
    }

    pub fn update(
        &self,
        transaction: &Transaction<'_>,
        session: &FocusSession,
    ) -> Result<(), SessionRepositoryError> {
        let changed = transaction.execute("UPDATE sessions SET target_end_at = ?2, paused_at = ?3, total_paused_ms = ?4, completed_at = ?5, status = ?6, pause_count = ?7, updated_at = ?8 WHERE id = ?1", params![session.id, session.target_end_at, session.paused_at, session.total_paused_ms, session.completed_at, session.status.as_db(), session.pause_count, session.updated_at]).map_err(map_write_error)?;
        if changed == 1 {
            Ok(())
        } else {
            Err(SessionRepositoryError::NotFound)
        }
    }
}

fn decode_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<FocusSession> {
    let status: String = row.get(7)?;
    let status = SessionStatus::from_db(&status).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            7,
            rusqlite::types::Type::Text,
            Box::new(SessionError::DataInvalid),
        )
    })?;
    Ok(FocusSession {
        id: row.get(0)?,
        planned_duration_ms: row.get(1)?,
        started_at: row.get(2)?,
        target_end_at: row.get(3)?,
        paused_at: row.get(4)?,
        total_paused_ms: row.get(5)?,
        completed_at: row.get(6)?,
        status,
        pause_count: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn map_write_error(error: rusqlite::Error) -> SessionRepositoryError {
    match &error {
        rusqlite::Error::SqliteFailure(code, _)
            if code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            SessionRepositoryError::OpenSessionExists
        }
        rusqlite::Error::FromSqlConversionFailure(..) => SessionRepositoryError::DataInvalid,
        _ => SessionRepositoryError::Database(error),
    }
}

fn map_read_error(error: rusqlite::Error) -> SessionRepositoryError {
    match error {
        rusqlite::Error::FromSqlConversionFailure(..) => SessionRepositoryError::DataInvalid,
        other => SessionRepositoryError::Database(other),
    }
}

#[derive(Debug, Error)]
pub enum SessionRepositoryError {
    #[error("an open focus session already exists")]
    OpenSessionExists,
    #[error("the focus session was not found")]
    NotFound,
    #[error("the focus session record is invalid")]
    DataInvalid,
    #[error("focus session database operation failed")]
    Database(#[source] rusqlite::Error),
}
