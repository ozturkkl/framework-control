use std::sync::Arc;

use crate::cli::{FrameworkTool, resolve_or_install};
use crate::types::Config;

#[derive(Clone)]
pub struct AppState {
    pub cli: Option<FrameworkTool>,
    pub config: Arc<tokio::sync::RwLock<Config>>,
    pub token: Option<String>,
}

impl AppState {
    pub async fn initialize() -> Self {
        let config = Arc::new(tokio::sync::RwLock::new(crate::config::load()));
        let token = std::env::var("FRAMEWORK_CONTROL_TOKEN").ok();
        match resolve_or_install().await {
            Ok(cli) => Self { cli: Some(cli), config, token },
            Err(_e) => {
                Self { cli: None, config, token }
            }
        }
    }

    pub fn is_valid_token(&self, auth_header: Option<&str>) -> bool {
        let Some(expected) = self.token.as_deref() else { return false; };
        let Some(provided) = auth_header else { return false; };
        let provided = provided.strip_prefix("Bearer ").unwrap_or(provided);
        provided == expected
    }
}


