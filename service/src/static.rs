use poem::{handler, IntoResponse, Response};
use poem::http::StatusCode;
use tracing::debug;

#[cfg(feature = "embed-ui")]
use rust_embed::RustEmbed;

#[cfg(feature = "embed-ui")]
#[derive(RustEmbed)]
#[folder = "../web/dist"]
struct EmbeddedWeb;

fn sanitize_path(p: &str) -> Option<String> {
    let mut s = p.trim_start_matches('/');
    if s.is_empty() || s.ends_with('/') { s = "index.html"; }
    if s.contains("..") { return None; }
    Some(s.to_string())
}

fn guess_mime(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") { return "text/html; charset=utf-8"; }
    if lower.ends_with(".js") || lower.ends_with(".mjs") { return "application/javascript"; }
    if lower.ends_with(".css") { return "text/css"; }
    if lower.ends_with(".json") || lower.ends_with(".map") { return "application/json"; }
    if lower.ends_with(".svg") { return "image/svg+xml"; }
    if lower.ends_with(".png") { return "image/png"; }
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") { return "image/jpeg"; }
    if lower.ends_with(".woff2") { return "font/woff2"; }
    "application/octet-stream"
}

#[handler]
pub fn serve_static(req: &poem::Request) -> Response {
    let request_path = req.uri().path();
    let rel = match sanitize_path(request_path) {
        Some(r) => r,
        None => return Response::builder().status(StatusCode::NOT_FOUND).body(()).into_response(),
    };
    #[cfg(feature = "embed-ui")]
    if let Some(content) = EmbeddedWeb::get(&rel) {
        debug!("static: embedded hit '{}'", rel);
        return Response::builder()
            .header("Content-Type", guess_mime(&rel))
            .body(content.data.into_owned())
            .into_response();
    }
    debug!("static: not found '{}'", rel);
    Response::builder().status(StatusCode::NOT_FOUND).body(()).into_response()
}


