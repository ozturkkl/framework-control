use super::*;

#[test]
fn paths_with_parent_segments_are_rejected() {
    assert_eq!(sanitize_path("/../config.json"), None);
    assert_eq!(sanitize_path("/assets/../../etc/passwd"), None);
    assert_eq!(sanitize_path("/assets/app.js"), Some("assets/app.js".to_string()));
}
