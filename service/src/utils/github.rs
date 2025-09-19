use serde_json::Value;

async fn fetch_latest_release(owner: &str, name: &str) -> Result<Value, String> {
    let api = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, name);
    let resp = reqwest::Client::new()
        .get(api)
        .header("user-agent", "framework-control-service")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())
}

fn extract_latest_version_tag(parsed: &Value) -> Option<String> {
    let tag = parsed.get("tag_name").and_then(|v| v.as_str())?;
    let v = tag.trim_start_matches('v').to_string();
    if v.is_empty() { None } else { Some(v) }
}

fn find_asset_url_ending_with(parsed: &Value, preferred_suffixes: &[&str]) -> Option<String> {
    let assets = parsed.get("assets")?.as_array()?.clone();
    assets.iter().find_map(|a| {
        let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let matches = preferred_suffixes.iter().any(|s| name.ends_with(s));
        if matches {
            a.get("browser_download_url").and_then(|v| v.as_str()).map(|s| s.to_string())
        } else {
            None
        }
    })
}

pub async fn get_latest_release_version_tag(owner: &str, name: &str) -> Result<Option<String>, String> {
    let parsed = fetch_latest_release(owner, name).await?;
    Ok(extract_latest_version_tag(&parsed))
}

pub async fn get_latest_release_url_ending_with(
    owner: &str,
    name: &str,
    preferred_suffixes: &[&str],
) -> Result<Option<String>, String> {
    let parsed = fetch_latest_release(owner, name).await?;
    Ok(find_asset_url_ending_with(&parsed, preferred_suffixes))
}


