use std::collections::VecDeque;
use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::cli::FrameworkTool;
use crate::types::{Config, TelemetrySample};

pub async fn run(
    cli_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
    cfg_lock: Arc<tokio::sync::RwLock<Config>>,
    samples_lock: Arc<tokio::sync::RwLock<VecDeque<TelemetrySample>>>,
) {
    info!("Telemetry task started");

    loop {
        // Snapshot config at loop start
        let tel_cfg = {
            let cfg = cfg_lock.read().await;
            cfg.telemetry.clone()
        };
        let poll_interval = Duration::from_millis(tel_cfg.poll_ms.max(200));

        // Obtain CLI
        let maybe_cli = { cli_lock.read().await.clone() };
        let Some(cli) = maybe_cli else {
            sleep(poll_interval).await;
            continue;
        };

        // Read thermal
        match cli.thermal().await {
            Ok(parsed) => {
                let now_ms = unix_time_ms();
                let sample = TelemetrySample {
                    ts_ms: now_ms,
                    temps: parsed.temps,
                    rpms: parsed.rpms,
                };
                {
                    let mut w = samples_lock.write().await;
                    w.push_back(sample);
                    // Trim by retain_seconds
                    let cutoff_ms = now_ms - (tel_cfg.retain_seconds as i64 * 1000);
                    while let Some(front) = w.front() {
                        if front.ts_ms < cutoff_ms {
                            w.pop_front();
                        } else {
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("telemetry read failed: {}", e);
            }
        }

        sleep(poll_interval).await;
    }
}

fn unix_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}


