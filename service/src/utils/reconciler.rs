use tokio::time::{Duration, Instant};

/// A generic, drift-aware reconciler for "desired vs observed" settings.
///
/// Preserves the "don't fight the system" behavior:
/// - apply immediately when the target changes
/// - if the value drifts, wait for a quiet window before reapplying
/// - never reapply more frequently than a cooldown
///
/// Intentionally domain-agnostic (power, fan, etc.).
#[derive(Debug, Clone)]
pub struct ReconcilerPolicy {
    /// How long the observed value must remain unchanged before we consider the system "quiet"
    /// and eligible for drift correction.
    pub quiet_window: Duration,

    /// Minimum time between successful apply attempts (drift correction / reapply).
    pub reapply_cooldown: Duration,
}

impl Default for ReconcilerPolicy {
    fn default() -> Self {
        Self {
            quiet_window: Duration::from_secs(60),
            reapply_cooldown: Duration::from_secs(120),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReconcilerState<T> {
    last_target: Option<T>,
    last_observed: Option<T>,
    last_observed_change_at: Instant,
    last_apply_at: Option<Instant>,
    warmed_up: bool,
}

impl<T> ReconcilerState<T> {
    pub fn new(now: Instant) -> Self {
        Self {
            last_target: None,
            last_observed: None,
            last_observed_change_at: now,
            last_apply_at: None,
            warmed_up: false,
        }
    }
}

/// Setting IO primitive: how to read the current value and how to apply a target value.
///
/// - `read_current` is optional: return `Ok(None)` if the platform/backend can't read.
/// - `apply_target` should attempt to apply and return an error string on failure.
pub trait SettingIo<T>: Send + Sync {
    fn read_current<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<T>, String>> + Send + 'a>>;

    fn apply_target<'a>(
        &'a self,
        target: &'a T,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>;
}

/// Result of a single reconcile attempt.
#[derive(Debug, Clone)]
pub enum ReconcileOutcome {
    Noop,
    Applied,
    ApplyFailed(String),
    Disabled,
    Cooldown { remaining: Duration },
    QuietWindow { remaining: Duration },
    AppliedImmediate,
}

/// A drift-aware reconciler instance for a single setting.
///
/// `T` is the value type (e.g. u32, String).
pub struct Reconciler<T> {
    policy: ReconcilerPolicy,
    state: ReconcilerState<T>,
}

impl<T> Reconciler<T>
where
    T: Clone + PartialEq,
{
    pub fn new(policy: ReconcilerPolicy, now: Instant) -> Self {
        Self {
            policy,
            state: ReconcilerState::new(now),
        }
    }

    pub async fn reconcile(&mut self, enabled: bool, target: Option<T>, io: &dyn SettingIo<T>) -> ReconcileOutcome {
        let now = Instant::now();

        // Warmup: mark as warmed up on first call regardless of enabled state
        if !self.state.warmed_up {
            self.state.warmed_up = true;
            // If disabled on first call, just mark warmup complete and return
            if !enabled || target.is_none() {
                return ReconcileOutcome::Noop;
            }
            // If enabled on first call, save target without applying
            let target = target.expect("checked above");
            self.state.last_target = Some(target);
            return ReconcileOutcome::Noop;
        }

        // After warmup: handle disabled state
        if !enabled || target.is_none() {
            if self.state.last_target.is_some() {
                self.state.last_target = None;
                return ReconcileOutcome::Disabled;
            }
            return ReconcileOutcome::Noop;
        }

        let target = target.expect("checked above");
        let target_changed = self.state.last_target.as_ref() != Some(&target);

        // Manual config change: apply immediately, bypass boot delay and cooldown
        if target_changed {
            match io.apply_target(&target).await {
                Ok(_) => {
                    self.state.last_target = Some(target);
                    self.state.last_apply_at = Some(now);
                    return ReconcileOutcome::AppliedImmediate;
                }
                Err(e) => return ReconcileOutcome::ApplyFailed(e),
            }
        }

        let current = match io.read_current().await {
            Ok(v) => v,
            Err(_) => None,
        };

        if let Some(ref cur) = current {
            match self.state.last_observed.as_ref() {
                None => {
                    self.state.last_observed = Some(cur.clone());
                    self.state.last_observed_change_at = now;
                }
                Some(prev) => {
                    if prev != cur {
                        self.state.last_observed = Some(cur.clone());
                        self.state.last_observed_change_at = now;
                    }
                }
            }
        }

        // Drift correction logic: apply if conditions permit

        // If we can read current and it matches target, nothing to do
        if let Some(cur) = &current {
            if cur == &target {
                return ReconcileOutcome::Noop;
            }
        }

        // Check cooldown gate
        if let Some(t) = self.state.last_apply_at {
            let elapsed = now.saturating_duration_since(t);
            if elapsed < self.policy.reapply_cooldown {
                let remaining = self.policy.reapply_cooldown.saturating_sub(elapsed);
                return ReconcileOutcome::Cooldown { remaining };
            }
        }

        // If we have a current value (drift case), check quiet window
        if current.is_some() {
            let elapsed = now.saturating_duration_since(self.state.last_observed_change_at);
            if elapsed < self.policy.quiet_window {
                let remaining = self.policy.quiet_window.saturating_sub(elapsed);
                return ReconcileOutcome::QuietWindow { remaining };
            }
        }

        // All gates passed: apply the target
        match io.apply_target(&target).await {
            Ok(_) => {
                self.state.last_apply_at = Some(now);
                ReconcileOutcome::Applied
            }
            Err(e) => ReconcileOutcome::ApplyFailed(e),
        }
    }
}
