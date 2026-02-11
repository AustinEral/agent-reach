use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::types::RegistryEntry;

/// In-memory registry of DID -> endpoint mappings
#[derive(Clone)]
pub struct Registry {
    inner: Arc<RwLock<HashMap<String, RegistryEntry>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register or update an agent's endpoint
    pub fn register(&self, entry: RegistryEntry) {
        let mut map = self.inner.write();
        map.insert(entry.did.clone(), entry);
    }

    /// Look up an agent by DID
    pub fn lookup(&self, did: &str) -> Option<RegistryEntry> {
        let map = self.inner.read();
        map.get(did).cloned()
    }

    /// Remove an agent's registration
    pub fn deregister(&self, did: &str) -> bool {
        let mut map = self.inner.write();
        map.remove(did).is_some()
    }

    /// Remove expired entries (call periodically)
    pub fn cleanup_expired(&self) {
        let now = chrono::Utc::now().timestamp();
        let mut map = self.inner.write();
        map.retain(|_, entry| entry.expires_at > now);
    }

    /// Get count of registered agents
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
