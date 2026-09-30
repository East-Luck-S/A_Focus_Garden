use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MINIMUM_FOCUS_DURATION_MS: i64 = 60_000;
pub const MAXIMUM_FOCUS_DURATION_MS: i64 = 180 * 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Abandoned,
    RecoveryRequired,
}

impl SessionStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Paused => "PAUSED",
            Self::Completed => "COMPLETED",
            Self::Abandoned => "ABANDONED",
            Self::RecoveryRequired => "RECOVERY_REQUIRED",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, SessionError> {
        match value {
            "ACTIVE" => Ok(Self::Active),
            "PAUSED" => Ok(Self::Paused),
            "COMPLETED" => Ok(Self::Completed),
            "ABANDONED" => Ok(Self::Abandoned),
            "RECOVERY_REQUIRED" => Ok(Self::RecoveryRequired),
            _ => Err(SessionError::DataInvalid),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecoveryResolution {
    Continue,
    Abandon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusSession {
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
    /// M5's immutable preview bundle. It is produced and validated by the
    /// application layer because it crosses persistence and generator bounds.
    pub plant_seed: Option<String>,
    pub generator_version: Option<i64>,
    pub plant_genome_json: Option<String>,
}

impl FocusSession {
    pub fn start(id: String, planned_duration_ms: i64, now: i64) -> Result<Self, SessionError> {
        if !(MINIMUM_FOCUS_DURATION_MS..=MAXIMUM_FOCUS_DURATION_MS).contains(&planned_duration_ms) {
            return Err(SessionError::InvalidDuration);
        }
        if now < 0 {
            return Err(SessionError::TimeValueInvalid);
        }
        let target_end_at = now
            .checked_add(planned_duration_ms)
            .ok_or(SessionError::TimeValueInvalid)?;
        Ok(Self {
            id,
            planned_duration_ms,
            started_at: now,
            target_end_at,
            paused_at: None,
            total_paused_ms: 0,
            completed_at: None,
            status: SessionStatus::Active,
            pause_count: 0,
            created_at: now,
            updated_at: now,
            plant_seed: None,
            generator_version: None,
            plant_genome_json: None,
        })
    }

    pub fn remaining_at(&self, now: i64) -> Result<i64, SessionError> {
        if now < 0 {
            return Err(SessionError::TimeValueInvalid);
        }
        let reference_time = match self.status {
            SessionStatus::Paused => self.paused_at.ok_or(SessionError::DataInvalid)?,
            SessionStatus::Completed | SessionStatus::Abandoned => return Ok(0),
            SessionStatus::Active | SessionStatus::RecoveryRequired => now,
        };
        Ok(self.target_end_at.saturating_sub(reference_time).max(0))
    }

    pub fn pause(&mut self, now: i64) -> Result<(), SessionError> {
        self.require_status(SessionStatus::Active)?;
        self.ensure_action_time(now, self.started_at)?;
        self.paused_at = Some(now);
        self.pause_count = self
            .pause_count
            .checked_add(1)
            .ok_or(SessionError::TimeValueInvalid)?;
        self.updated_at = now;
        self.status = SessionStatus::Paused;
        Ok(())
    }

    pub fn resume(&mut self, now: i64) -> Result<(), SessionError> {
        self.require_status(SessionStatus::Paused)?;
        let paused_at = self.paused_at.ok_or(SessionError::DataInvalid)?;
        self.ensure_action_time(now, paused_at)?;
        let pause_duration = now
            .checked_sub(paused_at)
            .ok_or(SessionError::TimeValueInvalid)?;
        self.total_paused_ms = self
            .total_paused_ms
            .checked_add(pause_duration)
            .ok_or(SessionError::TimeValueInvalid)?;
        self.target_end_at = self
            .target_end_at
            .checked_add(pause_duration)
            .ok_or(SessionError::TimeValueInvalid)?;
        self.paused_at = None;
        self.updated_at = now;
        self.status = SessionStatus::Active;
        Ok(())
    }

    #[allow(dead_code)] // M2 proves completion in the domain; M6 will expose transactional finalization.
    pub fn complete(&mut self, now: i64) -> Result<(), SessionError> {
        self.require_status(SessionStatus::Active)?;
        self.ensure_action_time(now, self.started_at)?;
        if self.remaining_at(now)? != 0 {
            return Err(SessionError::NotReadyToComplete);
        }
        self.status = SessionStatus::Completed;
        self.completed_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    pub fn abandon(&mut self, now: i64) -> Result<(), SessionError> {
        if !matches!(
            self.status,
            SessionStatus::Active | SessionStatus::Paused | SessionStatus::RecoveryRequired
        ) {
            return Err(SessionError::InvalidTransition);
        }
        self.ensure_action_time(now, self.started_at)?;
        self.status = SessionStatus::Abandoned;
        self.paused_at = None;
        self.updated_at = now;
        Ok(())
    }

    pub fn mark_recovery_required(&mut self, now: i64) -> Result<(), SessionError> {
        self.require_status(SessionStatus::Active)?;
        self.ensure_action_time(now, self.started_at)?;
        self.status = SessionStatus::RecoveryRequired;
        self.updated_at = now;
        Ok(())
    }

    pub fn resolve_recovery(
        &mut self,
        resolution: RecoveryResolution,
        now: i64,
    ) -> Result<(), SessionError> {
        self.require_status(SessionStatus::RecoveryRequired)?;
        self.ensure_action_time(now, self.started_at)?;
        match resolution {
            RecoveryResolution::Continue => {
                // A recovery continuation deliberately preserves timer truth: no artificial pause is added.
                self.status = SessionStatus::Active;
                self.updated_at = now;
                Ok(())
            }
            RecoveryResolution::Abandon => self.abandon_from_recovery(now),
        }
    }

    fn abandon_from_recovery(&mut self, now: i64) -> Result<(), SessionError> {
        self.status = SessionStatus::Abandoned;
        self.paused_at = None;
        self.updated_at = now;
        Ok(())
    }

    fn require_status(&self, expected: SessionStatus) -> Result<(), SessionError> {
        if self.status == expected {
            Ok(())
        } else {
            Err(SessionError::InvalidTransition)
        }
    }

    fn ensure_action_time(&self, now: i64, earliest: i64) -> Result<(), SessionError> {
        if now < 0 || now < earliest || now < self.updated_at {
            Err(SessionError::TimeValueInvalid)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("the planned duration is invalid")]
    InvalidDuration,
    #[error("the requested session state transition is invalid")]
    InvalidTransition,
    #[error("the session cannot be completed before its timer ends")]
    #[allow(dead_code)] // Completion becomes a native use case in M6.
    NotReadyToComplete,
    #[error("a persisted session contains invalid data")]
    DataInvalid,
    #[error("a supplied or calculated time value is invalid")]
    TimeValueInvalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active() -> FocusSession {
        FocusSession::start("id".into(), 60_000, 1_000).unwrap()
    }

    #[test]
    fn timer_uses_wall_clock_and_zero_boundary() {
        let session = active();
        assert_eq!(session.remaining_at(1_001).unwrap(), 59_999);
        assert_eq!(session.remaining_at(61_000).unwrap(), 0);
        assert_eq!(session.remaining_at(70_000).unwrap(), 0);
    }

    #[test]
    fn pause_resume_accumulates_and_shifts_target() {
        let mut session = active();
        session.pause(11_000).unwrap();
        assert_eq!(session.remaining_at(80_000).unwrap(), 50_000);
        session.resume(31_000).unwrap();
        session.pause(41_000).unwrap();
        session.resume(46_000).unwrap();
        assert_eq!(session.total_paused_ms, 25_000);
        assert_eq!(session.target_end_at, 86_000);
        assert_eq!(session.pause_count, 2);
    }

    #[test]
    fn terminal_and_illegal_transitions_are_rejected() {
        let mut session = active();
        assert_eq!(session.resume(2_000), Err(SessionError::InvalidTransition));
        assert_eq!(
            session.complete(2_000),
            Err(SessionError::NotReadyToComplete)
        );
        session.abandon(2_000).unwrap();
        assert_eq!(session.pause(3_000), Err(SessionError::InvalidTransition));
    }

    #[test]
    fn recovery_continue_preserves_target_and_pause_total() {
        let mut session = active();
        session.mark_recovery_required(2_000).unwrap();
        session
            .resolve_recovery(RecoveryResolution::Continue, 5_000)
            .unwrap();
        assert_eq!(session.target_end_at, 61_000);
        assert_eq!(session.total_paused_ms, 0);
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[test]
    fn invalid_duration_and_clock_values_are_rejected() {
        assert_eq!(
            FocusSession::start("id".into(), 1, 0),
            Err(SessionError::InvalidDuration)
        );
        assert_eq!(
            FocusSession::start("id".into(), 60_000, -1),
            Err(SessionError::TimeValueInvalid)
        );
        assert_eq!(
            FocusSession::start("id".into(), MAXIMUM_FOCUS_DURATION_MS + 1, 0),
            Err(SessionError::InvalidDuration)
        );
        let mut session = active();
        assert_eq!(session.pause(999), Err(SessionError::TimeValueInvalid));
        assert_eq!(
            FocusSession::start("id".into(), MAXIMUM_FOCUS_DURATION_MS, i64::MAX - 1),
            Err(SessionError::TimeValueInvalid)
        );
    }

    #[test]
    fn actions_reject_wall_clock_rollback_after_a_transition() {
        let mut session = active();
        session.pause(2_000).unwrap();
        assert_eq!(session.abandon(1_500), Err(SessionError::TimeValueInvalid));
        session.resume(3_000).unwrap();
        session.mark_recovery_required(4_000).unwrap();
        assert_eq!(
            session.resolve_recovery(RecoveryResolution::Continue, 3_500),
            Err(SessionError::TimeValueInvalid)
        );
    }

    #[test]
    fn completion_requires_zero_remaining_time() {
        let mut session = active();
        assert_eq!(
            session.complete(60_999),
            Err(SessionError::NotReadyToComplete)
        );
        session.complete(61_000).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
        assert_eq!(session.remaining_at(62_000).unwrap(), 0);
    }
}
