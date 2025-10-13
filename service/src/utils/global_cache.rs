use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

struct CacheState {
    values: RwLock<HashMap<String, (Arc<dyn Any + Send + Sync>, Instant)>>,
    // Separate store for negative (error) cache entries. We keep it distinct to
    // preserve type expectations of callers that only cache successful values.
    error_values: RwLock<HashMap<String, (Arc<dyn Any + Send + Sync>, Instant)>>,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

fn state() -> &'static CacheState {
    static INSTANCE: OnceLock<CacheState> = OnceLock::new();
    INSTANCE.get_or_init(|| CacheState {
        values: RwLock::new(HashMap::new()),
        error_values: RwLock::new(HashMap::new()),
        locks: Mutex::new(HashMap::new()),
    })
}

async fn get_lock_for_key(key: &str) -> Arc<Mutex<()>> {
    let st = state();
    let mut locks = st.locks.lock().await;
    if let Some(lock) = locks.get(key) {
        return lock.clone();
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(key.to_string(), lock.clone());
    lock
}

/// Global, key-based TTL cache with single-flight refresh per key.
/// - Returns cached value only within TTL.
/// - Optionally caches error results within TTL to throttle call pressure when upstream is failing.
pub async fn cache_get_or_update<T, F, Fut, E>(
    key: &str,
    ttl: Duration,
    cache_errors: bool,
    factory: F,
) -> Result<T, E>
where
    T: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let st = state();

    // Fast path: serve fresh success cache if key exists and type matches
    {
        let values = st.values.read().await;
        if let Some((arc_any, ts)) = values.get(key) {
            if ts.elapsed() < ttl {
                if let Some(v) = arc_any.as_ref().downcast_ref::<T>() {
                    return Ok(v.clone());
                }
            }
        }
    }
    // Fast path: serve negative cache if enabled and present
    if cache_errors {
        let error_values = st.error_values.read().await;
        if let Some((arc_any, ts)) = error_values.get(key) {
            if ts.elapsed() < ttl {
                if let Some(err) = arc_any.as_ref().downcast_ref::<E>() {
                    return Err(err.clone());
                }
            }
        }
    }

    // Slow path: per-key single-flight
    let per_key_lock = get_lock_for_key(key).await;
    let _lock_guard = per_key_lock.lock().await;

    // Check again after acquiring the lock
    {
        let values = st.values.read().await;
        if let Some((arc_any, ts)) = values.get(key) {
            if ts.elapsed() < ttl {
                if let Some(v) = arc_any.as_ref().downcast_ref::<T>() {
                    return Ok(v.clone());
                }
            }
        }
    }
    if cache_errors {
        let error_values = st.error_values.read().await;
        if let Some((arc_any, ts)) = error_values.get(key) {
            if ts.elapsed() < ttl {
                if let Some(err) = arc_any.as_ref().downcast_ref::<E>() {
                    return Err(err.clone());
                }
            }
        }
    }

    // Refresh via factory
    match factory().await {
        Ok(value) => {
            // On success, replace success cache and clear any error cache
            {
                let mut values = st.values.write().await;
                values.insert(key.to_string(), (Arc::new(value.clone()), Instant::now()));
            }
            // Always clear negative cache on success so future calls don't serve stale failures
            let mut error_values = st.error_values.write().await;
            error_values.remove(key);
            Ok(value)
        }
        Err(e) => {
            if cache_errors {
                // Store negative cache entry and clear any stale success entry
                {
                    let mut error_values = st.error_values.write().await;
                    error_values.insert(key.to_string(), (Arc::new(e.clone()), Instant::now()));
                }
                let mut values = st.values.write().await;
                values.remove(key);
            } else {
                // No error caching: ensure negative cache is cleared
                let mut error_values = st.error_values.write().await;
                error_values.remove(key);
                let mut values = st.values.write().await;
                values.remove(key);
            }
            Err(e)
        }
    }
}
