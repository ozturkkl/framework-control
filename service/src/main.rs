use std::net::SocketAddr;

use poem::{listener::TcpListener, EndpointExt, Route, get};
use poem::middleware::Cors;
use poem::http::{Method};
use poem_openapi::OpenApiService;
use tracing::{info};

mod cli;
mod config;
mod routes;
mod update;
mod shortcuts;
mod state;
mod tasks;
pub mod types;
mod utils;

mod r#static;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .without_time()
        .init();

    // If we're only generating OpenAPI, do it immediately and exit without requiring env or starting tasks
    let flag_arg = std::env::args().any(|a| a == "--generate-openapi");
    if flag_arg {
        let api = OpenApiService::new(crate::routes::Api, "framework-control-service", env!("CARGO_PKG_VERSION"))
            .server("");
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("web")
            .join("openapi.json");
        if let Some(parent) = out.parent() { let _ = std::fs::create_dir_all(parent); }
        let spec_json = api.spec();
        let _ = std::fs::write(&out, spec_json);
        return;
    }

    // Check if installer requested shortcut creation on first run
    shortcuts::create_shortcuts_if_installer_requested().await;

    let state = state::AppState::initialize().await;

    // Determine bind address to derive self-origins for CORS
    let bind_host = "127.0.0.1";
    let configured_port: u16 = std::env::var("FRAMEWORK_CONTROL_PORT")
        .expect("FRAMEWORK_CONTROL_PORT must be set (no defaults)")
        .parse()
        .expect("FRAMEWORK_CONTROL_PORT must be a valid u16 (e.g. 8090)");
    let self_origins = vec![format!("http://{}:{}", bind_host, configured_port)];

    // Merge configured origins with self-origins (dedup), then apply common rules
    let mut origins: Vec<String> = std::env::var("FRAMEWORK_CONTROL_ALLOWED_ORIGINS")
        .ok()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for so in &self_origins {
        if !origins.iter().any(|o| o.eq_ignore_ascii_case(so)) {
            origins.push(so.clone());
        }
    }
    let cors = origins
        .iter()
        .fold(Cors::new(), |c, origin| c.allow_origin(origin.as_str()))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(["content-type", "authorization"]) // allow bearer token
        .max_age(600);

    // Boot background tasks (fan curve if enabled)
    tasks::boot(&state).await;

    // Build OpenApiService from routes::Api
    let api = OpenApiService::new(crate::routes::Api, "framework-control-service", env!("CARGO_PKG_VERSION"))
        .server("");

    // Build the actual Poem app and apply CORS globally (API and static UI)
    let app = Route::new()
        .nest("/api", api)
        .at("/", get(r#static::serve_static))
        .at("/*path", get(r#static::serve_static))
        .data(state.clone())
        .with(cors);

    let addr: SocketAddr = (bind_host.parse::<std::net::IpAddr>().unwrap(), configured_port).into();
    info!("listening on http://{}", addr);
    poem::Server::new(TcpListener::bind(addr))
        .run(app)
        .await
        .unwrap();
}
