use crate::types::*;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub struct SessionManager {
    sessions: HashMap<Uuid, SessionCredential>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        SessionManager {
            sessions: HashMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        agent_id: Uuid,
        task_scope: String,
        compute_budget: f64,
        ttl: Duration,
        hardware_binding: Option<HardwareAttestation>,
    ) -> SessionCredential {
        let session = SessionCredential {
            id: Uuid::new_v4(),
            agent_id,
            task_scope,
            compute_budget,
            expires_at: SystemTime::now() + ttl,
            hardware_binding,
        };
        self.sessions.insert(session.id, session.clone());
        session
    }

    pub fn validate_session(&self, session_id: Uuid) -> Result<&SessionCredential, ZkpaError> {
        let session = self
            .sessions
            .get(&session_id)
            .ok_or(ZkpaError::SessionExpired)?;
        if SystemTime::now() > session.expires_at {
            return Err(ZkpaError::SessionExpired);
        }
        Ok(session)
    }

    pub fn revoke_session(&mut self, session_id: Uuid) -> Result<(), ZkpaError> {
        self.sessions
            .remove(&session_id)
            .ok_or(ZkpaError::SessionExpired)?;
        Ok(())
    }

    pub fn active_sessions(&self) -> Vec<&SessionCredential> {
        let now = SystemTime::now();
        self.sessions
            .values()
            .filter(|s| s.expires_at > now)
            .collect()
    }

    pub fn expired_sessions(&self) -> Vec<&SessionCredential> {
        let now = SystemTime::now();
        self.sessions
            .values()
            .filter(|s| s.expires_at <= now)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_validate_session() {
        let mut mgr = SessionManager::new();
        let session = mgr.create_session(
            Uuid::new_v4(),
            "task-1".to_string(),
            50.0,
            Duration::from_secs(3600),
            None,
        );
        let validated = mgr.validate_session(session.id).unwrap();
        assert_eq!(validated.id, session.id);
    }

    #[test]
    fn test_validate_expired_session_fails() {
        let mut mgr = SessionManager::new();
        let session = mgr.create_session(
            Uuid::new_v4(),
            "task-2".to_string(),
            50.0,
            Duration::from_secs(0),
            None,
        );
        // Session created with 0 TTL is immediately expired
        std::thread::sleep(Duration::from_millis(10));
        assert!(mgr.validate_session(session.id).is_err());
    }

    #[test]
    fn test_revoke_session() {
        let mut mgr = SessionManager::new();
        let session = mgr.create_session(
            Uuid::new_v4(),
            "task-3".to_string(),
            50.0,
            Duration::from_secs(3600),
            None,
        );
        mgr.revoke_session(session.id).unwrap();
        assert!(mgr.validate_session(session.id).is_err());
    }

    #[test]
    fn test_active_sessions_filter() {
        let mut mgr = SessionManager::new();
        let agent = Uuid::new_v4();
        mgr.create_session(
            agent,
            "active".to_string(),
            50.0,
            Duration::from_secs(3600),
            None,
        );
        mgr.create_session(
            agent,
            "expired".to_string(),
            50.0,
            Duration::from_secs(0),
            None,
        );
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(mgr.active_sessions().len(), 1);
        assert_eq!(mgr.expired_sessions().len(), 1);
    }

    #[test]
    fn test_validate_nonexistent_session() {
        let mgr = SessionManager::new();
        assert!(mgr.validate_session(Uuid::new_v4()).is_err());
    }
}
