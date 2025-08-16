use crate::state::AppState;

pub async fn boot(state: &AppState) {
    if let Some(cli) = &state.cli {
        let cli_clone = cli.clone();
        let cfg_clone = state.config.clone();
        // The loop reads config each tick; no restart/cancel complexity needed
        tokio::spawn(async move {
            crate::tasks::fan_curve::run(cli_clone, cfg_clone).await;
        });
    }
}

pub mod fan_curve;


