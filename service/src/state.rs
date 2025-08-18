use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;

use crate::cli::FrameworkTool;
use crate::types::Config;

#[derive(Clone)]
pub struct AppState {
    pub cli: Option<FrameworkTool>,
    pub config: Arc<tokio::sync::RwLock<Config>>,
}

impl AppState {
    pub async fn initialize() -> Self {
        let config = Arc::new(tokio::sync::RwLock::new(crate::config::load()));
        match FrameworkTool::new().await {
            Ok(cli) => Self { cli: Some(cli), config },
            Err(e) => {
                let log_path = r"C:\Program Files\FrameworkControl\service.log";
                if let Ok(mut f) = OpenOptions::new().append(true).open(log_path) {
                    writeln!(f, "CLI init failed: {}", e).ok();
                }
                Self { cli: None, config }
            }
        }
    }
}


