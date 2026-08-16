use crate::utils::extract::archive_contains_any_suffix;
use serde_json::Value;

async fn fetch_github_json(url: String) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .get(url)
        .header("user-agent", "framework-control-service")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())
}

async fn fetch_release(owner: &str, name: &str, tag: Option<&str>) -> Result<Value, String> {
    let api = match tag {
        Some(t) => format!("https://api.github.com/repos/{}/{}/releases/tags/{}", owner, name, t),
        None => format!("https://api.github.com/repos/{}/{}/releases/latest", owner, name),
    };
    fetch_github_json(api).await
}

/// Most recent release tag names (e.g. "v0.6.5"), newest first.
pub async fn list_release_tags(
    owner: &str,
    name: &str,
    count: u32,
    required_asset: &str,
    stable_only: bool,
) -> Result<Vec<String>, String> {
    let api = format!(
        "https://api.github.com/repos/{}/{}/releases?per_page={}",
        owner, name, count
    );
    let parsed = fetch_github_json(api).await?;
    let releases = parsed
        .as_array()
        .ok_or_else(|| "unexpected releases response".to_string())?;
    Ok(releases
        .iter()
        .filter(|r| {
            if stable_only {
                if r.get("draft").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return false;
                }
                if r.get("prerelease").and_then(|v| v.as_bool()).unwrap_or(false) {
                    return false;
                }
            }
            r.get("assets").and_then(|a| a.as_array()).is_some_and(|assets| {
                assets
                    .iter()
                    .any(|a| a.get("name").and_then(|n| n.as_str()) == Some(required_asset))
            })
        })
        .filter_map(|r| r.get("tag_name").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect())
}

fn extract_latest_version_tag(parsed: &Value) -> Option<String> {
    let tag = parsed.get("tag_name").and_then(|v| v.as_str())?;
    let v = tag.trim_start_matches('v').to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn find_asset_url_ending_with(parsed: &Value, preferred_suffixes: &[&str]) -> Option<String> {
    let assets = parsed.get("assets")?.as_array()?.clone();
    assets.iter().find_map(|a| {
        let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let name_lc = name.to_ascii_lowercase();
        let matches = preferred_suffixes
            .iter()
            .any(|s| name_lc.ends_with(&s.to_ascii_lowercase()));
        if matches {
            a.get("browser_download_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    })
}

pub async fn get_latest_release_version_tag(owner: &str, name: &str) -> Result<Option<String>, String> {
    let parsed = fetch_release(owner, name, None).await?;
    Ok(extract_latest_version_tag(&parsed))
}

pub async fn get_release(
    owner: &str,
    name: &str,
    tag: Option<&str>,
    preferred_suffixes: &[&str],
) -> Result<Option<String>, String> {
    let parsed = fetch_release(owner, name, tag).await?;
    if let Some(u) = find_asset_url_ending_with(&parsed, preferred_suffixes) {
        return Ok(Some(u));
    }
    // Fallback: try zip assets and peek inside
    if let Some(assets) = parsed.get("assets").and_then(|v| v.as_array()) {
        // Prefer archives that look like tool binaries, avoid lib-only zips like "libryzenadj-*.zip"
        for a in assets {
            let name = a
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !name.ends_with(".zip") {
                continue;
            }
            let Some(url) = a.get("browser_download_url").and_then(|v| v.as_str()) else {
                continue;
            };
            if archive_contains_any_suffix(url, preferred_suffixes)
                .await
                .unwrap_or(false)
            {
                return Ok(Some(url.to_string()));
            }
        }
    }
    Ok(None)
}
