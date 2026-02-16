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

    // Power settings task: start once at boot; it will wait until power backend is available
    #[cfg(target_os = "windows")]
    {
        let power_backend = state.ryzenadj.clone();
        let cfg_clone = state.config.clone();
        let ft_clone = state.framework_tool.clone();
        tokio::spawn(async move {
            crate::tasks::power::run(power_backend, cfg_clone, ft_clone).await;
        });
    }

    #[cfg(target_os = "linux")]
    {
        let power_backend = state.linux_power.clone();
        let cfg_clone = state.config.clone();
        let ft_clone = state.framework_tool.clone();
        tokio::spawn(async move {
            crate::tasks::power::run(power_backend, cfg_clone, ft_clone).await;
        });
    }

    // Battery settings task: applies charge limit and rate on change and periodically
    {
        let ft_clone = state.framework_tool.clone();
        let cfg_clone = state.config.clone();
        tokio::spawn(async move {
            crate::tasks::battery::run(ft_clone, cfg_clone).await;
        });
    }

    // Auto-update background task
    {
        let cfg_clone = state.config.clone();
        tokio::spawn(async move {
            crate::tasks::auto_update::run(cfg_clone).await;
        });
    }

    // Telemetry history task
    {
        let ft_clone = state.framework_tool.clone();
        let cfg_clone = state.config.clone();
        let samples_clone = state.telemetry_samples.clone();
        tokio::spawn(async move {
            crate::tasks::telemetry::run(ft_clone, cfg_clone, samples_clone).await;
        });
    }
}

pub mod fan_curve;
pub mod power;
pub mod battery;
pub mod auto_update;
pub mod telemetry;
