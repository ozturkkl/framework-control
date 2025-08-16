use std::fs::OpenOptions;
use std::io::Write;
use std::net::SocketAddr;

use axum::http::Method;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

mod cli;
mod config;
mod routes;
mod state;
mod tasks;

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

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(Any);

    // Boot background tasks (fan curve if enabled)
    tasks::boot(&state).await;

    let app = Router::new()
        .route("/api/power", get(routes::get_power))
        .route("/api/thermal", get(routes::get_thermal))
        .route("/api/health", get(routes::health))
        .route("/api/config", get(routes::get_config))
        .route("/api/config", post(routes::set_config))
        .with_state(state)
        .layer(cors);

    let host = std::env::var("FRAMEWORK_CONTROL_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("FRAMEWORK_CONTROL_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8090);
    let addr: SocketAddr = (host.parse::<std::net::IpAddr>().unwrap(), port).into();
    info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
