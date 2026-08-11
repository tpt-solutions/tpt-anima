use crate::types::*;
use std::collections::HashMap;
use uuid::Uuid;

pub struct DelegationManager {
    delegations: HashMap<Uuid, Vec<AuthorityDelegation>>,
}

impl Default for DelegationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DelegationManager {
    pub fn new() -> Self {
        DelegationManager {
            delegations: HashMap::new(),
        }
    }

    pub fn delegate(
        &mut self,
        parent_id: Uuid,
        child_id: Uuid,
        permissions: Vec<String>,
        zk_proof: Option<ZKProof>,
    ) -> Result<AuthorityDelegation, ZkpaError> {
        if permissions.is_empty() {
            return Err(ZkpaError::DelegationDenied(
                "cannot delegate with empty permissions".to_string(),
            ));
        }

        // Least-privilege enforcement: if `parent_id` itself only holds
        // authority via an inbound delegation (i.e. it is not a root
        // grantor), it cannot re-delegate permissions beyond what it was
        // granted. Without this check, a compromised or buggy intermediate
        // agent could escalate scope for its own children.
        if let Some(inherited) = self.granted_permissions(parent_id) {
            if let Some(excess) = permissions.iter().find(|p| !inherited.contains(*p)) {
                return Err(ZkpaError::DelegationDenied(format!(
                    "parent lacks permission '{excess}' and cannot delegate it"
                )));
            }
        }

        let delegation = AuthorityDelegation {
            parent_id,
            child_id,
            scoped_permissions: permissions,
            zk_proof,
            revoked: false,
        };

        self.delegations
            .entry(parent_id)
            .or_default()
            .push(delegation.clone());

        Ok(delegation)
    }

    pub fn revoke(&mut self, parent_id: Uuid, child_id: Uuid) -> Result<(), ZkpaError> {
        if let Some(delegations) = self.delegations.get_mut(&parent_id) {
            for d in delegations.iter_mut() {
                if d.child_id == child_id && !d.revoked {
                    d.revoked = true;
                    return Ok(());
                }
            }
        }
        Err(ZkpaError::DelegationDenied(
            "no active delegation found".to_string(),
        ))
    }

    pub fn is_authorized(
        &self,
        parent_id: Uuid,
        child_id: Uuid,
        required_permission: &str,
    ) -> bool {
        if let Some(delegations) = self.delegations.get(&parent_id) {
            for d in delegations {
                if d.child_id == child_id
                    && !d.revoked
                    && d.scoped_permissions
                        .iter()
                        .any(|p| p == required_permission)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Permissions `agent_id` currently holds as the child of some active
    /// delegation, union'd across every grantor. Returns `None` if
    /// `agent_id` has never appeared as a delegation child — such agents are
    /// treated as root grantors, free to delegate any permission set.
    fn granted_permissions(&self, agent_id: Uuid) -> Option<std::collections::HashSet<String>> {
        let mut found = false;
        let mut permissions = std::collections::HashSet::new();
        for delegations in self.delegations.values() {
            for d in delegations {
                if d.child_id == agent_id && !d.revoked {
                    found = true;
                    permissions.extend(d.scoped_permissions.iter().cloned());
                }
            }
        }
        found.then_some(permissions)
    }

    pub fn delegation_chain(&self, parent_id: Uuid) -> Vec<&AuthorityDelegation> {
        self.delegations
            .get(&parent_id)
            .map(|d| d.iter().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delegate_success() {
        let mut mgr = DelegationManager::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        let delegation = mgr
            .delegate(parent, child, vec!["read".to_string()], None)
            .unwrap();
        assert_eq!(delegation.parent_id, parent);
        assert_eq!(delegation.child_id, child);
        assert!(!delegation.revoked);
    }

    #[test]
    fn test_delegate_empty_permissions_fails() {
        let mut mgr = DelegationManager::new();
        let result = mgr.delegate(Uuid::new_v4(), Uuid::new_v4(), vec![], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_revoke_delegation() {
        let mut mgr = DelegationManager::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        mgr.delegate(parent, child, vec!["write".to_string()], None)
            .unwrap();
        mgr.revoke(parent, child).unwrap();
        assert!(!mgr.is_authorized(parent, child, "write"));
    }

    #[test]
    fn test_is_authorized() {
        let mut mgr = DelegationManager::new();
        let parent = Uuid::new_v4();
        let child = Uuid::new_v4();
        mgr.delegate(
            parent,
            child,
            vec!["read".to_string(), "write".to_string()],
            None,
        )
        .unwrap();
        assert!(mgr.is_authorized(parent, child, "read"));
        assert!(mgr.is_authorized(parent, child, "write"));
        assert!(!mgr.is_authorized(parent, child, "delete"));
    }

    #[test]
    fn test_delegation_chain() {
        let mut mgr = DelegationManager::new();
        let parent = Uuid::new_v4();
        let child1 = Uuid::new_v4();
        let child2 = Uuid::new_v4();
        mgr.delegate(parent, child1, vec!["read".to_string()], None)
            .unwrap();
        mgr.delegate(parent, child2, vec!["write".to_string()], None)
            .unwrap();
        let chain = mgr.delegation_chain(parent);
        assert_eq!(chain.len(), 2);
    }
}
