use std::sync::Arc;

use crate::cli::framework_tool::{resolve_or_install, tool_suspect};
use crate::cli::FrameworkTool;
use crate::types::Config;

#[cfg(target_os = "windows")]
use crate::cli::RyzenAdj;

#[cfg(target_os = "linux")]
use crate::cli::LinuxPower;

#[derive(Clone)]
pub struct AppState {
    pub framework_tool: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
    pub config: Arc<tokio::sync::RwLock<Config>>,
    pub telemetry_samples: Arc<tokio::sync::RwLock<std::collections::VecDeque<crate::types::TelemetrySample>>>,

    #[cfg(target_os = "windows")]
    pub ryzenadj: Arc<tokio::sync::RwLock<Option<RyzenAdj>>>,

    #[cfg(target_os = "linux")]
    pub linux_power: Arc<tokio::sync::RwLock<Option<LinuxPower>>>,
}

impl AppState {
    pub async fn initialize() -> Self {
        let config = Arc::new(tokio::sync::RwLock::new(crate::config::load()));

        // Wrap framework_tool in a lock and spawn a passive resolver (no auto-install here)
        let framework_tool = Arc::new(tokio::sync::RwLock::new(None));
        Self::spawn_framework_tool_resolver(framework_tool.clone(), config.clone());

        #[cfg(target_os = "windows")]
        let ryzenadj = {
            // Do not auto-install RyzenAdj on init; only periodically resolve if user has installed
            let ryz = Arc::new(tokio::sync::RwLock::new(RyzenAdj::new().await.ok()));
            Self::spawn_ryzenadj_resolver(ryz.clone());
            ryz
        };

        #[cfg(target_os = "linux")]
        let linux_power = {
            // Initialize Linux power management
            let lp = Arc::new(tokio::sync::RwLock::new(LinuxPower::new().await.ok()));
            Self::spawn_linux_power_resolver(lp.clone());
            lp
        };

        Self {
            framework_tool,
            config,
            telemetry_samples: Arc::new(tokio::sync::RwLock::new(Default::default())),
            #[cfg(target_os = "windows")]
            ryzenadj,
            #[cfg(target_os = "linux")]
            linux_power,
        }
    }

    #[cfg(target_os = "windows")]
    fn spawn_ryzenadj_resolver(ryz_lock: Arc<tokio::sync::RwLock<Option<RyzenAdj>>>) {
        tokio::spawn(async move {
            use tokio::time::{sleep, Duration};
            loop {
                let is_missing = { ryz_lock.read().await.is_none() };
                if is_missing {
                    if let Ok(new_ryz) = RyzenAdj::new().await {
                        {
                            let mut w = ryz_lock.write().await;
                            *w = Some(new_ryz);
                        }
                        tracing::info!("state: ryzenadj is now available");
                    }
                }
                sleep(Duration::from_secs(5)).await;
            }
        });
    }

    #[cfg(target_os = "linux")]
    fn spawn_linux_power_resolver(lp_lock: Arc<tokio::sync::RwLock<Option<LinuxPower>>>) {
        tokio::spawn(async move {
            use tokio::time::{sleep, Duration};
            loop {
                let is_missing = { lp_lock.read().await.is_none() };
                if is_missing {
                    if let Ok(new_lp) = LinuxPower::new().await {
                        {
                            let mut w = lp_lock.write().await;
                            *w = Some(new_lp);
                        }
                        tracing::info!("state: linux_power is now available");
                    }
                }
                sleep(Duration::from_secs(5)).await;
            }
        });
    }

    fn spawn_framework_tool_resolver(
        ft_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
        config: Arc<tokio::sync::RwLock<Config>>,
    ) {
        tokio::spawn(async move {
            use tokio::time::{sleep, Duration};
            // Steady cadence while present; exponential backoff while absent.
            const BASE: Duration = Duration::from_secs(5);
            const MAX_BACKOFF: Duration = Duration::from_secs(300);
            let mut backoff = BASE;

            loop {
                let current = { ft_lock.read().await.clone() };
                let wait = match current {
                    // Confirm before clearing so a transient failure doesn't drop a tool that still works.
                    Some(cli) => {
                        if tool_suspect() && cli.versions().await.is_err() {
                            let mut w = ft_lock.write().await;
                            *w = None;
                            tracing::warn!("state: framework_tool no longer responding; cleared from state");
                        }
                        backoff = BASE;
                        BASE
                    }
                    None => {
                        let cfg = { config.read().await.clone() };
                        match resolve_or_install(&cfg.framework_tool).await {
                            Ok(cli) => {
                                let mut w = ft_lock.write().await;
                                *w = Some(cli);
                                tracing::info!("state: framework_tool is now available");
                                backoff = BASE;
                                BASE
                            }
                            Err(_) => {
                                let this_wait = backoff;
                                backoff = (backoff * 2).min(MAX_BACKOFF);
                                tracing::debug!("state: framework_tool unavailable; next attempt in {:?}", this_wait);
                                this_wait
                            }
                        }
                    }
                };
                sleep(wait).await;
            }
        });
    }
}
