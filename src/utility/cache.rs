use std::collections::HashMap;
use std::fmt;
use std::mem;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

const MAX_CACHE_SIZE_BYTES: usize = 25 * 1024 * 1024; // 25 MB

#[derive(Clone, Debug)]
pub struct CachedRecord {
    pub ip: IpAddr,
    pub record_id: String,
    pub provider: String,
    pub timestamp: Instant,
}

pub struct DnsCache {
    records: HashMap<String, CachedRecord>,
    ttl: Duration,
    current_size: usize,
}

impl DnsCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            records: HashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
            current_size: 0,
        }
    }

    fn calculate_entry_size(key: &str, record: &CachedRecord) -> usize {
        // Calculate approximate size in bytes
        key.len()
            + size_of::<IpAddr>()
            + record.record_id.len()
            + record.provider.len()
            + size_of::<Instant>()
    }

    pub fn get(&self, domain: &str) -> Option<CachedRecord> {
        self.records.get(domain).and_then(|record| {
            if record.timestamp.elapsed() < self.ttl {
                Some(record.clone())
            } else {
                None
            }
        })
    }

    pub fn insert(&mut self, domain: String, record: CachedRecord) {
        // Calculate new entry size
        let entry_size = Self::calculate_entry_size(&domain, &record);

        // If adding this entry would exceed the limit, remove old entries
        if self.current_size + entry_size > MAX_CACHE_SIZE_BYTES {
            self.evict_old_entries(entry_size);
        }

        // Remove old size if entry exists
        if let Some(old_record) = self.records.get(&domain) {
            self.current_size -= Self::calculate_entry_size(&domain, old_record);
        }

        // Insert new entry
        self.records.insert(domain.clone(), record);
        self.current_size += entry_size;
    }

    pub fn invalidate(&mut self, domain: &str) {
        if let Some(record) = self.records.remove(domain) {
            self.current_size -= Self::calculate_entry_size(domain, &record);
            debug!("Cache entry invalidated for domain: {}", domain);
        }
    }

    fn evict_old_entries(&mut self, needed_space: usize) {
        let mut entries: Vec<_> = self
            .records
            .iter()
            .map(|(k, v)| (k.clone(), v.clone(), v.timestamp))
            .collect();

        // Sort by timestamp (oldest first)
        entries.sort_by_key(|(_k, _v, t)| *t);

        // Remove entries until we have enough space
        let mut space_freed = 0;
        let mut removed = 0;

        for (domain, record, _) in entries {
            let entry_size = Self::calculate_entry_size(&domain, &record);
            self.records.remove(&domain);
            self.current_size -= entry_size;
            space_freed += entry_size;
            removed += 1;

            if self.current_size + needed_space <= MAX_CACHE_SIZE_BYTES {
                break;
            }
        }

        if removed > 0 {
            warn!(
                "Evicted {} cache entries ({} bytes) to make space for new entry",
                removed, space_freed
            );
        }
    }

    pub fn update_ttl(&mut self, ttl_seconds: u64) {
        self.ttl = Duration::from_secs(ttl_seconds);
        debug!("Cache TTL updated to {} seconds", ttl_seconds);
    }
}

#[derive(Clone)]
pub struct SharedDnsCache(Arc<RwLock<DnsCache>>);

impl fmt::Debug for SharedDnsCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedDnsCache")
            .field("inner", &"<DnsCache>")
            .finish()
    }
}

impl SharedDnsCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self(Arc::new(RwLock::new(DnsCache::new(ttl_seconds))))
    }

    pub async fn get(&self, domain: &str) -> Option<CachedRecord> {
        self.0.read().await.get(domain)
    }

    pub async fn insert(&self, domain: String, record: CachedRecord) {
        self.0.write().await.insert(domain, record);
    }

    pub async fn invalidate(&self, domain: &str) {
        self.0.write().await.invalidate(domain);
    }

    pub async fn update_ttl(&self, ttl_seconds: u64) {
        self.0.write().await.update_ttl(ttl_seconds);
    }
}
