use super::*;
use serde_json::json;

#[test]
fn release_tags_require_asset_and_skip_unstable_when_asked() {
    let releases = json!([
        { "tag_name": "v0.6.0", "prerelease": true, "assets": [{ "name": "framework_tool" }] },
        { "tag_name": "v0.5.9", "draft": true, "assets": [{ "name": "framework_tool" }] },
        { "tag_name": "v0.5.8", "assets": [{ "name": "framework_tool.exe" }] },
        { "tag_name": "v0.5.7", "assets": [{ "name": "framework_tool" }] },
    ]);
    let releases = releases.as_array().unwrap();

    assert_eq!(tags_with_asset(releases, "framework_tool", true), vec!["v0.5.7"]);
    assert_eq!(
        tags_with_asset(releases, "framework_tool", false),
        vec!["v0.6.0", "v0.5.9", "v0.5.7"]
    );
}
