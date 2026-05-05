use std::fs;
use std::path::{Path, PathBuf};

use super::super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse, HttpHeader,
};
use super::super::source::{short_hash, source_cache_namespace};
use super::super::{AddonProviderOptions, AddonSourceRef};
use crate::core::error::{AppError, AppResult};
use crate::core::task::{CancellationToken, NeverCancel};

pub(super) const TEMP_DOWNLOAD_SUFFIX: &str = ".hearthsync-part";

pub(in crate::core::addon::provider) fn resolve_archive_path(
    source: &AddonSourceRef,
    archive_name: &str,
    stage_root: &Path,
    options: &AddonProviderOptions,
) -> PathBuf {
    let archive_name = normalize_archive_name(archive_name);
    match &options.download_cache_dir {
        Some(cache_dir) => cache_dir
            .join(source_cache_namespace(source))
            .join(short_hash(&source.display_name()))
            .join(archive_name),
        None => stage_root.join(archive_name),
    }
}

pub(in crate::core::addon::provider) fn download_to_path_with_headers(
    http_client: &impl HttpClient,
    url: &str,
    headers: Vec<HttpHeader>,
    destination: &Path,
    cancellation: Option<&dyn CancellationToken>,
    observer: Option<&dyn HttpDownloadProgressObserver>,
) -> AppResult<HttpDownloadResponse> {
    ensure_destination_is_replaceable(destination)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary_destination = temporary_download_path(destination);
    if temporary_destination.exists() {
        fs::remove_file(&temporary_destination)?;
    }

    let never_cancel = NeverCancel;
    let cancellation = cancellation.unwrap_or(&never_cancel);
    let download_result = http_client.download_to_path(
        HttpDownloadRequest::new(url, temporary_destination.clone()).with_headers(headers),
        cancellation,
        observer,
    );
    let response = match download_result {
        Ok(response) => response,
        Err(error) => {
            let _ = fs::remove_file(&temporary_destination);
            return Err(error);
        }
    };

    if response.is_not_modified() {
        let _ = fs::remove_file(&temporary_destination);
        return Ok(response);
    }

    if let Err(error) = replace_downloaded_file(&temporary_destination, destination) {
        let _ = fs::remove_file(&temporary_destination);
        return Err(error);
    }
    Ok(response)
}

fn replace_downloaded_file(temporary_destination: &Path, destination: &Path) -> AppResult<()> {
    ensure_destination_is_replaceable(destination)?;
    if destination.is_file() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary_destination, destination)?;
    Ok(())
}

fn ensure_destination_is_replaceable(destination: &Path) -> AppResult<()> {
    if destination.exists() && !destination.is_file() {
        return Err(AppError::Validation(format!(
            "addon download destination is not a replaceable file: {}",
            destination.display()
        )));
    }

    Ok(())
}

pub(in crate::core::addon::provider) fn guess_archive_name_from_url(url: &str) -> Option<String> {
    if let Ok(parsed_url) = reqwest::Url::parse(url)
        && let Some(file_name) = parsed_url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
    {
        return Some(file_name.to_string());
    }

    let stripped = url
        .split_once('#')
        .map_or(url, |(before_fragment, _)| before_fragment);
    let stripped = stripped
        .split_once('?')
        .map_or(stripped, |(before_query, _)| before_query);
    let file_name = Path::new(stripped).file_name()?.to_str()?;
    (!file_name.is_empty()).then(|| file_name.to_string())
}

pub(super) fn normalize_archive_name(archive_name: &str) -> String {
    Path::new(archive_name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip")
        .to_string()
}

fn temporary_download_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip");
    destination.with_file_name(format!("{file_name}{TEMP_DOWNLOAD_SUFFIX}"))
}
