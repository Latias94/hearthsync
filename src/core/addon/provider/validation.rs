use std::fs;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::curseforge::CurseForgeFile;
use super::github::GitHubReleaseAsset;
use super::http::HttpHeader;
use crate::core::error::AppResult;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct RemoteArchiveValidators {
    pub(super) content_length: Option<u64>,
    pub(super) last_modified: Option<String>,
    pub(super) etag: Option<String>,
    pub(super) sha256: Option<String>,
    pub(super) sha1: Option<String>,
    pub(super) md5: Option<String>,
}

impl RemoteArchiveValidators {
    pub(super) fn is_empty(&self) -> bool {
        self.content_length.is_none()
            && self.last_modified.is_none()
            && self.etag.is_none()
            && self.sha256.is_none()
            && self.sha1.is_none()
            && self.md5.is_none()
    }
}

pub(super) fn remote_validators_for_github_asset(
    asset: &GitHubReleaseAsset,
) -> RemoteArchiveValidators {
    let mut validators = RemoteArchiveValidators {
        content_length: asset.size,
        last_modified: asset.updated_at.clone(),
        etag: None,
        sha256: None,
        sha1: None,
        md5: None,
    };

    if let Some(digest) = asset.digest.as_deref()
        && let Some(value) = digest.strip_prefix("sha256:")
    {
        validators.sha256 = Some(value.to_ascii_lowercase());
    }

    validators
}

const CURSEFORGE_HASH_ALGO_SHA1: u8 = 1;
const CURSEFORGE_HASH_ALGO_MD5: u8 = 2;

pub(super) fn remote_validators_for_curseforge_file(
    file: &CurseForgeFile,
) -> RemoteArchiveValidators {
    let mut validators = RemoteArchiveValidators {
        content_length: file.file_length,
        last_modified: (!file.file_date.is_empty()).then(|| file.file_date.clone()),
        etag: None,
        sha256: None,
        sha1: None,
        md5: None,
    };

    for hash in &file.hashes {
        if hash.value.is_empty() {
            continue;
        }
        match hash.algo {
            CURSEFORGE_HASH_ALGO_SHA1 if validators.sha1.is_none() => {
                validators.sha1 = Some(hash.value.to_ascii_lowercase());
            }
            CURSEFORGE_HASH_ALGO_MD5 if validators.md5.is_none() => {
                validators.md5 = Some(hash.value.to_ascii_lowercase());
            }
            _ => {}
        }
    }

    validators
}

pub(super) fn remote_validators_for_http_headers(
    headers: &[HttpHeader],
) -> RemoteArchiveValidators {
    RemoteArchiveValidators {
        content_length: header_value_case_insensitive(headers, "content-length")
            .and_then(|value| value.parse::<u64>().ok()),
        last_modified: header_value_case_insensitive(headers, "last-modified"),
        etag: header_value_case_insensitive(headers, "etag"),
        sha256: None,
        sha1: None,
        md5: None,
    }
}

fn header_value_case_insensitive(headers: &[HttpHeader], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn conditional_request_headers_for_transport_validators(
    validators: &RemoteArchiveValidators,
) -> Vec<HttpHeader> {
    let mut headers = Vec::new();
    if let Some(etag) = &validators.etag {
        headers.push(HttpHeader {
            name: "If-None-Match".to_string(),
            value: etag.clone(),
        });
    }
    if let Some(last_modified) = &validators.last_modified {
        headers.push(HttpHeader {
            name: "If-Modified-Since".to_string(),
            value: last_modified.clone(),
        });
    }
    headers
}

pub(super) fn file_sha256(path: &Path) -> AppResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
