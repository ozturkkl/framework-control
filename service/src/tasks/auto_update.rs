use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{error, info};

use crate::types::Config;
use crate::update::check_and_apply_now;

/// Auto-update background task
/// Periodically checks for updates and applies them if `auto_install` is enabled.
pub async fn run(cfg: Arc<tokio::sync::RwLock<Config>>) {
    loop {
        let cfg = cfg.read().await.clone();
        if cfg.updates.auto_install {
            match check_and_apply_now().await {
                Ok(true) => info!("auto-update: installer launched"),
                Ok(false) => { /* no update available */ }
                Err(e) => error!("auto-update: check/apply failed: {}", e),
            }
        }
        // sleep 6h
        sleep(Duration::from_secs(6 * 60 * 60)).await;
    }
}


