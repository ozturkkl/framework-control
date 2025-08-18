use std::fs::OpenOptions;
use std::io::Write;
use std::net::SocketAddr;

use poem::{listener::TcpListener, EndpointExt, Route};
use poem::middleware::Cors;
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
    // Write startup log
    let log_path = r"C:\Program Files\FrameworkControl\service.log";
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap_or_else(|_| {
            // If can't write to Program Files, try temp
            let temp_path = std::env::temp_dir().join("framework-control.log");
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(temp_path)
                .expect("Failed to open log file")
        });
    
    writeln!(log_file, "\n=== Service starting at {} ===", chrono::Local::now()).ok();
    writeln!(log_file, "Current user: {:?}", std::env::var("USERNAME")).ok();
    writeln!(log_file, "PATH: {:?}", std::env::var("PATH")).ok();
    
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .without_time()
        .init();

    let state = state::AppState::initialize().await;

    let cors = Cors::new();

    // Boot background tasks (fan curve if enabled)
    tasks::boot(&state).await;

    // Build OpenApiService from routes::Api
    let api = OpenApiService::new(crate::routes::Api, "framework-control-service", env!("CARGO_PKG_VERSION"))
        .server("/");
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

    let host = std::env::var("FRAMEWORK_CONTROL_HOST").unwrap_or_else(|_| "127.0.0.1".into());
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
