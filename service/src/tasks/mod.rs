use crate::state::AppState;
use crate::update::check_and_apply_now;

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
                    let _ = check_and_apply_now().await;
                }
                // sleep 6h
                tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
            }
        });
    }
}

pub mod fan_curve;


