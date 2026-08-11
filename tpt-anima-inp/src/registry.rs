//! Capability Registry for the Intent & Negotiation Protocol.
//!
//! Agents register their capabilities; the registry provides lookup by action
//! type, availability tracking, and cost profile management.

use std::collections::HashMap;

use crate::intent::{AgentId, Capability, CapabilityId};

// ---------------------------------------------------------------------------
// CapabilityRegistry
// ---------------------------------------------------------------------------

/// Central registry of all known agent capabilities.
pub struct CapabilityRegistry {
    /// All registered capabilities, keyed by ID.
    capabilities: HashMap<CapabilityId, Capability>,
    /// Reverse index: action type → capability IDs that handle it.
    action_index: HashMap<String, Vec<CapabilityId>>,
    /// Agent → capability IDs they own.
    agent_index: HashMap<AgentId, Vec<CapabilityId>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            action_index: HashMap::new(),
            agent_index: HashMap::new(),
        }
    }

    /// Register a capability. Returns its ID.
    pub fn register(&mut self, capability: Capability) -> CapabilityId {
        let id = capability.id;

        // Update action index
        for action in &capability.action_types {
            self.action_index
                .entry(action.clone())
                .or_default()
                .push(id);
        }

        // Update agent index
        self.agent_index
            .entry(capability.provider_agent_id)
            .or_default()
            .push(id);

        self.capabilities.insert(id, capability);
        id
    }

    /// Deregister a capability by ID.
    pub fn deregister(&mut self, id: &CapabilityId) -> Option<Capability> {
        if let Some(cap) = self.capabilities.remove(id) {
            // Remove from action index
            for action in &cap.action_types {
                if let Some(ids) = self.action_index.get_mut(action) {
                    ids.retain(|x| x != id);
                    if ids.is_empty() {
                        self.action_index.remove(action);
                    }
                }
            }

            // Remove from agent index
            if let Some(ids) = self.agent_index.get_mut(&cap.provider_agent_id) {
                ids.retain(|x| x != id);
                if ids.is_empty() {
                    self.agent_index.remove(&cap.provider_agent_id);
                }
            }

            Some(cap)
        } else {
            None
        }
    }

    /// Find all capabilities that handle a given action.
    pub fn lookup_by_action(&self, action: &str) -> Vec<&Capability> {
        self.action_index
            .get(action)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.capabilities.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all capabilities registered by a specific agent.
    pub fn capabilities_of(&self, agent_id: &AgentId) -> Vec<&Capability> {
        self.agent_index
            .get(agent_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.capabilities.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a capability by ID.
    pub fn get(&self, id: &CapabilityId) -> Option<&Capability> {
        self.capabilities.get(id)
    }

    /// Get all available capabilities (currently active, not disabled).
    pub fn available_capabilities(&self) -> Vec<&Capability> {
        self.capabilities
            .values()
            .filter(|c| c.availability.available)
            .collect()
    }

    /// Number of registered capabilities.
    pub fn count(&self) -> usize {
        self.capabilities.len()
    }

    /// Number of distinct action types registered.
    pub fn action_type_count(&self) -> usize {
        self.action_index.len()
    }

    /// Get all registered action types.
    pub fn action_types(&self) -> Vec<&str> {
        self.action_index.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{AvailabilityWindow, EconomicTerms, PerformanceProfile};
    use std::time::Duration;
    use uuid::Uuid;

    fn agent() -> AgentId {
        Uuid::new_v4()
    }

    fn make_capability(action: &str, agent_id: AgentId) -> Capability {
        Capability::new(
            vec![action.to_string()],
            PerformanceProfile {
                typical_latency: Duration::from_millis(100),
                max_latency: Duration::from_secs(5),
                typical_accuracy: 0.95,
                throughput_per_second: 100.0,
            },
            EconomicTerms::FixedPrice { credits: 0.5 },
            agent_id,
        )
    }

    #[test]
    fn register_and_lookup() {
        let mut registry = CapabilityRegistry::new();
        let aid = agent();
        let cap = make_capability("data_transfer", aid);
        let id = cap.id;
        registry.register(cap);

        let found = registry.lookup_by_action("data_transfer");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, id);
    }

    #[test]
    fn deregister_removes_capability() {
        let mut registry = CapabilityRegistry::new();
        let cap = make_capability("compute", agent());
        let id = cap.id;
        registry.register(cap);

        registry.deregister(&id);
        assert!(registry.lookup_by_action("compute").is_empty());
    }

    #[test]
    fn multiple_capabilities_per_action() {
        let mut registry = CapabilityRegistry::new();
        registry.register(make_capability("data_transfer", agent()));
        registry.register(make_capability("data_transfer", agent()));

        let found = registry.lookup_by_action("data_transfer");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn capabilities_of_agent() {
        let mut registry = CapabilityRegistry::new();
        let aid = agent();
        registry.register(make_capability("a", aid));
        registry.register(make_capability("b", aid));
        registry.register(make_capability("c", agent()));

        let caps = registry.capabilities_of(&aid);
        assert_eq!(caps.len(), 2);
    }

    #[test]
    fn unavailable_capability_excluded() {
        let mut registry = CapabilityRegistry::new();
        let mut cap = make_capability("compute", agent());
        cap.availability = AvailabilityWindow {
            available: false,
            start_hour: None,
            end_hour: None,
            timezone: None,
        };
        registry.register(cap);

        let available = registry.available_capabilities();
        assert!(available.is_empty());
    }

    #[test]
    fn action_types_and_count() {
        let mut registry = CapabilityRegistry::new();
        registry.register(make_capability("a", agent()));
        registry.register(make_capability("b", agent()));
        registry.register(make_capability("a", agent()));

        assert_eq!(registry.action_type_count(), 2);
        assert_eq!(registry.count(), 3);
    }
}
