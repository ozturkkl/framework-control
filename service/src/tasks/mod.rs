use crate::state::AppState;

pub async fn boot(state: &AppState) {
    // Fan curve task: always start; it will wait until framework_tool is available
    {
        let ft_clone = state.framework_tool.clone();
        let cfg_clone = state.config.clone();
        tokio::spawn(async move {
            crate::tasks::fan_curve::run(ft_clone, cfg_clone).await;
        });
    }

    // Power settings task: start once at boot; it will wait until RyzenAdj is available
    {
        let ryz_clone = state.ryzenadj.clone();
        let cfg_clone = state.config.clone();
        tokio::spawn(async move {
            crate::tasks::power::run(ryz_clone, cfg_clone).await;
        });
    }

    // Auto-update background task
    {
        let cfg_clone = state.config.clone();
        tokio::spawn(async move {
            crate::tasks::auto_update::run(cfg_clone).await;
        });
    }
}

pub mod fan_curve;
pub mod power;
pub mod auto_update;
