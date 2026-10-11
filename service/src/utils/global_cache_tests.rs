use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// The cache is process-global, so every test uses its own key.
const TTL: Duration = Duration::from_secs(60);

#[tokio::test]
async fn concurrent_callers_share_one_refresh() {
    let calls = AtomicUsize::new(0);
    let fetch = || {
        cache_get_or_update("test.single_flight", TTL, false, || async {
            calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<u32, String>(7)
        })
    };

    let (first, second) = tokio::join!(fetch(), fetch());

    assert_eq!((first, second), (Ok(7), Ok(7)));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cached_error_is_served_until_replaced_by_uncached_success() {
    let key = "test.cached_error";
    let _ = cache_get_or_update(key, TTL, true, || async { Err::<u32, String>("down".into()) }).await;

    let served = cache_get_or_update(key, TTL, true, || async { Ok::<u32, String>(1) }).await;
    assert_eq!(served, Err("down".to_string()));

    let bypassed = cache_get_or_update(key, TTL, false, || async { Ok::<u32, String>(1) }).await;
    assert_eq!(bypassed, Ok(1));

    let after_success = cache_get_or_update(key, TTL, true, || async { Ok::<u32, String>(2) }).await;
    assert_eq!(after_success, Ok(1));
}
