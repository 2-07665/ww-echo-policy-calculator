use echo_app::{
    AppError, AppResult,
    data::{DataStore, MAX_BYTES},
};
use serde::Deserialize;
use std::{io::Read, sync::Arc, time::Duration};

#[derive(Deserialize)]
pub struct Download {
    url: String,
}
#[derive(Deserialize)]
pub struct Install {
    content: String,
}
fn download_error() -> AppError {
    AppError {
        code: "data_download",
        message: "数据下载失败，请检查地址和网络；已保留原数据。".into(),
    }
}
fn download(url: &str) -> AppResult<Vec<u8>> {
    let url = reqwest::Url::parse(url).map_err(|_| download_error())?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err(download_error());
    }
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| download_error())?;
    let response = client
        .get(url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|_| download_error())?;
    if response
        .content_length()
        .is_some_and(|n| n > MAX_BYTES as u64)
    {
        return Err(download_error());
    }
    let mut bytes = Vec::new();
    response
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| download_error())?;
    if bytes.len() > MAX_BYTES {
        return Err(download_error());
    }
    Ok(bytes)
}
#[tauri::command]
pub async fn data_download(
    state: tauri::State<'_, Arc<DataStore>>,
    payload: Download,
) -> AppResult<serde_json::Value> {
    let store = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        store.install(&download(&payload.url)?)?;
        let (pack, status) = store.snapshot();
        Ok(echo_app::bootstrap_with_data(&pack, status))
    })
    .await
    .map_err(|_| download_error())?
}
#[tauri::command]
pub async fn data_install(
    state: tauri::State<'_, Arc<DataStore>>,
    payload: Install,
) -> AppResult<serde_json::Value> {
    let store = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        store.install(payload.content.as_bytes())?;
        let (pack, status) = store.snapshot();
        Ok(echo_app::bootstrap_with_data(&pack, status))
    })
    .await
    .map_err(|_| download_error())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires access to raw.githubusercontent.com"]
    fn downloads_https_json() {
        // Exercise the production TLS/HTTP path against an already published source.
        let bytes = download("https://raw.githubusercontent.com/2-07665/ww-echo-policy-calculator/main/crates/echo_app/presets/wuwa_echo_tool.json").unwrap();
        let source: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(source["presets"].as_array().is_some_and(|p| !p.is_empty()));
        // A weight-source file is not a complete installable data pack.
        assert!(echo_app::data::DataPack::parse(&bytes).is_err());
    }
    #[test]
    fn rejects_insecure_urls_before_connecting() {
        for url in [
            "http://localhost/data.json",
            "file:///tmp/data.json",
            "https://user:password@example.com/data.json",
            "not a URL",
        ] {
            assert_eq!(download(url).unwrap_err().code, "data_download");
        }
    }
}
