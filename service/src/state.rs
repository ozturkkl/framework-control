use std::sync::Arc;

use crate::cli::{FrameworkTool, RyzenAdj};
use crate::cli::framework_tool::resolve_or_install;
use crate::types::Config;

#[derive(Clone)]
pub struct AppState {
    pub framework_tool: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
    pub ryzenadj: Arc<tokio::sync::RwLock<Option<RyzenAdj>>>,
    pub config: Arc<tokio::sync::RwLock<Config>>,
    pub token: Option<String>,
    pub telemetry_samples: Arc<tokio::sync::RwLock<std::collections::VecDeque<crate::types::TelemetrySample>>>,
}

impl AppState {
    pub async fn initialize() -> Self {
        let config = Arc::new(tokio::sync::RwLock::new(crate::config::load()));
        let token = std::env::var("FRAMEWORK_CONTROL_TOKEN")
            .ok()
            .or_else(|| option_env!("FRAMEWORK_CONTROL_TOKEN").map(String::from));

        // Do not auto-install RyzenAdj on init; only periodically resolve if user has installed
        let ryzenadj = Arc::new(tokio::sync::RwLock::new(RyzenAdj::new().await.ok()));
        Self::spawn_ryzenadj_resolver(ryzenadj.clone());

        // Wrap framework_tool in a lock and spawn a passive resolver (no auto-install here)
        let framework_tool = Arc::new(tokio::sync::RwLock::new(None));
        Self::spawn_framework_tool_resolver(framework_tool.clone());

        Self {
            framework_tool,
            ryzenadj,
            config,
            token,
            telemetry_samples: Arc::new(tokio::sync::RwLock::new(Default::default())),
        }
    }

    pub fn is_valid_token(&self, auth_header: Option<&str>) -> bool {
        let Some(expected) = self.token.as_deref() else {
            return false;
        };
        let Some(provided) = auth_header else {
            return false;
        };
        let provided = provided.strip_prefix("Bearer ").unwrap_or(provided);
        provided == expected
    }

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

    fn spawn_framework_tool_resolver(ft_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>) {
        tokio::spawn(async move {
            use tokio::time::{sleep, Duration};
            loop {
                let current = { ft_lock.read().await.clone() };
                match current {
                    Some(cli) => {
                        // Validate liveness; if not runnable, clear state (no auto-install here)
                        if cli.versions().await.is_err() {
                            let mut w = ft_lock.write().await;
                            *w = None;
                            tracing::warn!("state: framework_tool became unavailable; clearing from state");
                        }
                    }
                    None => {
                        // Try resolving or installing if not present
                        if let Ok(cli) = resolve_or_install().await {
                            let mut w = ft_lock.write().await;
                            *w = Some(cli);
                            tracing::info!("state: framework_tool is now available");
                        }
                    }
                }
                sleep(Duration::from_secs(5)).await;
            }
        });
    }
}
