use super::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

const QUIET: Duration = Duration::from_secs(10);
const COOLDOWN: Duration = Duration::from_secs(30);

struct FakeIo {
    current: Mutex<Result<Option<u32>, String>>,
    applied: Mutex<Vec<u32>>,
}

impl FakeIo {
    fn reading(current: Result<Option<u32>, String>) -> Self {
        Self {
            current: Mutex::new(current),
            applied: Mutex::new(Vec::new()),
        }
    }

    fn set_current(&self, value: u32) {
        *self.current.lock().unwrap() = Ok(Some(value));
    }

    fn applied(&self) -> Vec<u32> {
        self.applied.lock().unwrap().clone()
    }
}

impl SettingIo<u32> for FakeIo {
    fn read_current<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<Option<u32>, String>> + Send + 'a>> {
        let current = self.current.lock().unwrap().clone();
        Box::pin(async move { current })
    }

    fn apply_target<'a>(&'a self, target: &'a u32) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.applied.lock().unwrap().push(*target);
        Box::pin(async { Ok(()) })
    }
}

async fn warmed_up(target: u32, io: &FakeIo) -> Reconciler<u32> {
    let policy = ReconcilerPolicy {
        quiet_window: QUIET,
        reapply_cooldown: COOLDOWN,
    };
    let mut reconciler = Reconciler::new(policy, Instant::now());
    assert_eq!(
        reconciler.reconcile(true, Some(target), io).await,
        ReconcileOutcome::Noop
    );
    reconciler
}

#[tokio::test(start_paused = true)]
async fn startup_does_not_apply_before_quiet_window() {
    let io = FakeIo::reading(Ok(Some(50)));
    let mut reconciler = warmed_up(30, &io).await;

    assert_eq!(
        reconciler.reconcile(true, Some(30), &io).await,
        ReconcileOutcome::QuietWindow { remaining: QUIET }
    );
    assert!(io.applied().is_empty());
}

#[tokio::test(start_paused = true)]
async fn observed_change_restarts_quiet_window() {
    let io = FakeIo::reading(Ok(Some(50)));
    let mut reconciler = warmed_up(30, &io).await;
    reconciler.reconcile(true, Some(30), &io).await;

    tokio::time::advance(QUIET / 2).await;
    io.set_current(60);
    assert_eq!(
        reconciler.reconcile(true, Some(30), &io).await,
        ReconcileOutcome::QuietWindow { remaining: QUIET }
    );

    tokio::time::advance(QUIET).await;
    assert_eq!(
        reconciler.reconcile(true, Some(30), &io).await,
        ReconcileOutcome::Applied
    );
    assert_eq!(io.applied(), vec![30]);
}

#[tokio::test(start_paused = true)]
async fn target_change_applies_immediately_despite_cooldown() {
    let io = FakeIo::reading(Ok(Some(30)));
    let mut reconciler = warmed_up(30, &io).await;

    assert_eq!(
        reconciler.reconcile(true, Some(40), &io).await,
        ReconcileOutcome::AppliedImmediate
    );
    assert_eq!(
        reconciler.reconcile(true, Some(50), &io).await,
        ReconcileOutcome::AppliedImmediate
    );
    assert_eq!(io.applied(), vec![40, 50]);
}

#[tokio::test(start_paused = true)]
async fn drift_after_apply_waits_for_cooldown() {
    let io = FakeIo::reading(Ok(Some(30)));
    let mut reconciler = warmed_up(30, &io).await;
    reconciler.reconcile(true, Some(40), &io).await;

    io.set_current(50);
    assert_eq!(
        reconciler.reconcile(true, Some(40), &io).await,
        ReconcileOutcome::Cooldown { remaining: COOLDOWN }
    );

    tokio::time::advance(COOLDOWN).await;
    assert_eq!(
        reconciler.reconcile(true, Some(40), &io).await,
        ReconcileOutcome::Applied
    );
    assert_eq!(io.applied(), vec![40, 40]);
}

#[tokio::test(start_paused = true)]
async fn reenabling_reapplies_unchanged_target() {
    let io = FakeIo::reading(Ok(Some(30)));
    let mut reconciler = warmed_up(30, &io).await;

    assert_eq!(
        reconciler.reconcile(false, Some(30), &io).await,
        ReconcileOutcome::Disabled
    );
    assert_eq!(reconciler.reconcile(false, Some(30), &io).await, ReconcileOutcome::Noop);
    assert_eq!(
        reconciler.reconcile(true, Some(30), &io).await,
        ReconcileOutcome::AppliedImmediate
    );
    assert_eq!(io.applied(), vec![30]);
}

#[tokio::test(start_paused = true)]
async fn unreadable_value_is_reapplied_once_per_cooldown() {
    let io = FakeIo::reading(Err("read failed".to_string()));
    let mut reconciler = warmed_up(30, &io).await;

    assert_eq!(
        reconciler.reconcile(true, Some(30), &io).await,
        ReconcileOutcome::Applied
    );
    assert_eq!(
        reconciler.reconcile(true, Some(30), &io).await,
        ReconcileOutcome::Cooldown { remaining: COOLDOWN }
    );

    tokio::time::advance(COOLDOWN).await;
    assert_eq!(
        reconciler.reconcile(true, Some(30), &io).await,
        ReconcileOutcome::Applied
    );
    assert_eq!(io.applied(), vec![30, 30]);
}
