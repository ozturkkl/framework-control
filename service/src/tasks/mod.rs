use crate::state::AppState;
use crate::update::check_and_apply_now;
use tracing::{error, info};

pub async fn boot(state: &AppState) {
    if let Some(cli) = &state.cli {
        let cli_clone = cli.clone();
        let cfg_clone = state.config.clone();
        // The loop reads config each tick; no restart/cancel complexity needed
        tokio::spawn(async move {
            crate::tasks::fan_curve::run(cli_clone, cfg_clone).await;
        });
    }

    // Auto-update background task
    {
        let app_state = state.clone();
        tokio::spawn(async move {
            loop {
                // read settings
                let cfg = app_state.config.read().await.clone();
                if cfg.updates.auto_install {
                    match check_and_apply_now().await {
                        Ok(true) => info!("auto-update: installer launched"),
                        Ok(false) => { /* no update available */ }
                        Err(e) => error!("auto-update: check/apply failed: {}", e),
                    }
                }
                // sleep 6h
                tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
            }
        });
    }
}

pub mod fan_curve;


