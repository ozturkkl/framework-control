use std::net::SocketAddr;

use poem::{listener::TcpListener, EndpointExt, Route};
use poem::middleware::Cors;
use poem::http::Method;
use poem_openapi::OpenApiService;
use tracing::info;

mod cli;
mod config;
mod routes;
mod state;
mod tasks;
pub mod types;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .without_time()
        .init();

    let state = state::AppState::initialize().await;

    let cors = match std::env::var("FRAMEWORK_CONTROL_ALLOWED_ORIGINS") {
        Ok(val) if !val.trim().is_empty() => {
            val.split(',')
                .map(str::trim)
                .fold(Cors::new(), |c, origin| c.allow_origin(origin))
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(["content-type", "authorization"]) // allow bearer token
                .max_age(600)
        }
        _ => {
            tracing::warn!(
                "CORS: no FRAMEWORK_CONTROL_ALLOWED_ORIGINS configured; denying all cross-origin requests"
            );
            Cors::new()
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(["content-type", "authorization"]) // allow bearer token
                .max_age(600)
        }
    };

    // Boot background tasks (fan curve if enabled)
    tasks::boot(&state).await;

    // Build OpenApiService from routes::Api
    let api = OpenApiService::new(crate::routes::Api, "framework-control-service", env!("CARGO_PKG_VERSION"))
        .server("");
    // Optionally, write OpenAPI to a known path when requested (CLI flag), then exit cleanly
    let flag_arg = std::env::args().any(|a| a == "--generate-openapi");
    if flag_arg {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("web")
            .join("openapi.json");
        if let Some(parent) = out.parent() { std::fs::create_dir_all(parent).ok(); }
        let spec_json = api.spec();
        std::fs::write(&out, spec_json).ok();
        return;
    }

    // Build the actual Poem app
    let app = Route::new()
        .nest("/", api)
        .data(state.clone())
        .with(cors);

    let host = "127.0.0.1".to_string();
    let port: u16 = std::env::var("FRAMEWORK_CONTROL_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8090);
    let addr: SocketAddr = (host.parse::<std::net::IpAddr>().unwrap(), port).into();
    info!("listening on http://{}", addr);
    poem::Server::new(TcpListener::bind(addr))
        .run(app)
        .await
        .unwrap();
}
