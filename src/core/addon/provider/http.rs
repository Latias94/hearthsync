use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::header::ACCEPT_ENCODING;

use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const DOWNLOAD_BUFFER_SIZE: usize = 64 * 1024;
const DOWNLOAD_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(200);
const DOWNLOAD_PROGRESS_MIN_BYTES_DELTA: u64 = 256 * 1024;
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub headers: Vec<HttpHeader>,
    pub query: Vec<(String, String)>,
}

impl HttpRequest {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            query: Vec::new(),
        }
    }

    pub fn with_headers(mut self, headers: Vec<HttpHeader>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_query(mut self, query: Vec<(String, String)>) -> Self {
        self.query = query;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpDownloadRequest {
    pub url: String,
    pub headers: Vec<HttpHeader>,
    pub destination: PathBuf,
}

impl HttpDownloadRequest {
    pub fn new(url: impl Into<String>, destination: impl Into<PathBuf>) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            destination: destination.into(),
        }
    }

    pub fn with_headers(mut self, headers: Vec<HttpHeader>) -> Self {
        self.headers = headers;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status_code: u16,
    pub body: String,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpDownloadResponse {
    pub status_code: u16,
    pub headers: Vec<HttpHeader>,
}

impl HttpDownloadResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    pub fn is_not_modified(&self) -> bool {
        self.status_code == 304
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpDownloadProgress {
    pub bytes_current: u64,
    pub bytes_total: Option<u64>,
    pub bytes_per_second: Option<u64>,
}

pub trait HttpDownloadProgressObserver {
    fn on_progress(&self, progress: HttpDownloadProgress);
}

pub trait HttpClient {
    fn get(&self, request: HttpRequest) -> AppResult<HttpResponse>;

    fn download_to_path(
        &self,
        request: HttpDownloadRequest,
        cancellation: &dyn CancellationToken,
        observer: Option<&dyn HttpDownloadProgressObserver>,
    ) -> AppResult<HttpDownloadResponse>;
}

#[derive(Debug, Clone)]
pub struct ReqwestHttpClient {
    client: Client,
    connect_timeout: Duration,
    request_timeout: Duration,
    download_timeout: Duration,
}

impl ReqwestHttpClient {
    pub fn with_timeouts(connect_timeout: Duration, request_timeout: Duration) -> Self {
        Self::with_request_and_download_timeouts(connect_timeout, request_timeout, request_timeout)
    }

    pub fn with_request_and_download_timeouts(
        connect_timeout: Duration,
        request_timeout: Duration,
        download_timeout: Duration,
    ) -> Self {
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .user_agent(USER_AGENT_VALUE)
            .build()
            .expect("reqwest blocking client");
        Self {
            client,
            connect_timeout,
            request_timeout,
            download_timeout,
        }
    }

    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    pub fn download_timeout(&self) -> Duration {
        self.download_timeout
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::with_request_and_download_timeouts(
            DEFAULT_CONNECT_TIMEOUT,
            DEFAULT_REQUEST_TIMEOUT,
            DEFAULT_DOWNLOAD_TIMEOUT,
        )
    }
}

impl HttpClient for ReqwestHttpClient {
    fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
        let mut builder = self.client.get(&request.url).timeout(self.request_timeout);
        for header in &request.headers {
            builder = builder.header(&header.name, &header.value);
        }
        if !request.query.is_empty() {
            builder = builder.query(&request.query);
        }
        let response = builder.send()?;
        let status_code = response.status().as_u16();
        let body = response.text()?;
        Ok(HttpResponse { status_code, body })
    }

    fn download_to_path(
        &self,
        request: HttpDownloadRequest,
        cancellation: &dyn CancellationToken,
        observer: Option<&dyn HttpDownloadProgressObserver>,
    ) -> AppResult<HttpDownloadResponse> {
        ensure_download_not_cancelled(cancellation)?;
        let mut builder = self.client.get(&request.url).timeout(self.download_timeout);
        if !has_header(&request.headers, "Accept-Encoding") {
            builder = builder.header(ACCEPT_ENCODING, "identity");
        }
        for header in &request.headers {
            builder = builder.header(&header.name, &header.value);
        }
        let mut response = builder.send()?;
        let result = HttpDownloadResponse {
            status_code: response.status().as_u16(),
            headers: response
                .headers()
                .iter()
                .map(|(name, value)| HttpHeader {
                    name: name.as_str().to_string(),
                    value: String::from_utf8_lossy(value.as_bytes()).into_owned(),
                })
                .collect(),
        };
        if result.is_not_modified() {
            return Ok(result);
        }

        response.error_for_status_ref()?;
        write_response_to_path(&mut response, &request.destination, cancellation, observer)?;
        Ok(result)
    }
}

fn has_header(headers: &[HttpHeader], name: &str) -> bool {
    headers
        .iter()
        .any(|header| header.name.eq_ignore_ascii_case(name))
}

fn write_response_to_path(
    response: &mut reqwest::blocking::Response,
    destination: &Path,
    cancellation: &dyn CancellationToken,
    observer: Option<&dyn HttpDownloadProgressObserver>,
) -> AppResult<()> {
    ensure_download_not_cancelled(cancellation)?;
    let mut file = File::create(destination)?;
    let mut buffer = [0u8; DOWNLOAD_BUFFER_SIZE];
    let bytes_total = response.content_length();
    let started_at = Instant::now();
    let mut bytes_written = 0u64;
    let mut last_reported_bytes = 0u64;
    let mut last_report_at = started_at;

    emit_download_progress(observer, bytes_written, bytes_total, started_at);

    loop {
        ensure_download_not_cancelled(cancellation)?;

        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        bytes_written += bytes_read as u64;

        let now = Instant::now();
        if should_emit_download_progress(
            bytes_written,
            bytes_total,
            last_reported_bytes,
            last_report_at,
            now,
        ) {
            emit_download_progress(observer, bytes_written, bytes_total, started_at);
            last_reported_bytes = bytes_written;
            last_report_at = now;
        }
    }

    if bytes_written != last_reported_bytes {
        emit_download_progress(observer, bytes_written, bytes_total, started_at);
    }
    Ok(())
}

fn should_emit_download_progress(
    bytes_written: u64,
    bytes_total: Option<u64>,
    last_reported_bytes: u64,
    last_report_at: Instant,
    now: Instant,
) -> bool {
    bytes_total.is_some_and(|bytes_total| bytes_written >= bytes_total)
        || bytes_written.saturating_sub(last_reported_bytes) >= DOWNLOAD_PROGRESS_MIN_BYTES_DELTA
        || now.duration_since(last_report_at) >= DOWNLOAD_PROGRESS_MIN_INTERVAL
}

fn emit_download_progress(
    observer: Option<&dyn HttpDownloadProgressObserver>,
    bytes_current: u64,
    bytes_total: Option<u64>,
    started_at: Instant,
) {
    let Some(observer) = observer else {
        return;
    };

    observer.on_progress(HttpDownloadProgress {
        bytes_current,
        bytes_total,
        bytes_per_second: compute_bytes_per_second(bytes_current, started_at.elapsed()),
    });
}

fn compute_bytes_per_second(bytes_current: u64, elapsed: Duration) -> Option<u64> {
    if bytes_current == 0 || elapsed.is_zero() {
        return None;
    }

    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        return None;
    }

    Some(((bytes_current as u128 * 1_000_000_000u128) / nanos).min(u64::MAX as u128) as u64)
}

fn ensure_download_not_cancelled(cancellation: &dyn CancellationToken) -> AppResult<()> {
    if cancellation.is_cancelled() {
        Err(AppError::Cancelled(
            "addon provider download cancelled".to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reqwest_http_client_default_uses_bounded_timeouts() {
        let client = ReqwestHttpClient::default();

        assert_eq!(client.connect_timeout(), Duration::from_secs(30));
        assert_eq!(client.request_timeout(), Duration::from_secs(30));
        assert_eq!(client.download_timeout(), Duration::from_secs(10 * 60));
    }

    #[test]
    fn reqwest_http_client_with_timeouts_keeps_legacy_single_request_deadline() {
        let client =
            ReqwestHttpClient::with_timeouts(Duration::from_secs(1), Duration::from_secs(2));

        assert_eq!(client.connect_timeout(), Duration::from_secs(1));
        assert_eq!(client.request_timeout(), Duration::from_secs(2));
        assert_eq!(client.download_timeout(), Duration::from_secs(2));
    }

    #[test]
    fn has_header_matches_case_insensitively() {
        let headers = vec![HttpHeader {
            name: "accept-encoding".to_string(),
            value: "gzip".to_string(),
        }];

        assert!(has_header(&headers, "Accept-Encoding"));
        assert!(!has_header(&headers, "User-Agent"));
    }
}
