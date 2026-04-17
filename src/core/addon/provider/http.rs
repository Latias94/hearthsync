use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::Client;

use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_BUFFER_SIZE: usize = 64 * 1024;

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

pub trait HttpClient {
    fn get(&self, request: HttpRequest) -> AppResult<HttpResponse>;

    fn download_to_path(
        &self,
        request: HttpDownloadRequest,
        cancellation: &dyn CancellationToken,
    ) -> AppResult<()>;
}

#[derive(Debug, Clone)]
pub struct ReqwestHttpClient {
    client: Client,
    connect_timeout: Duration,
    request_timeout: Duration,
}

impl ReqwestHttpClient {
    pub fn with_timeouts(connect_timeout: Duration, request_timeout: Duration) -> Self {
        let client = Client::builder()
            .connect_timeout(connect_timeout)
            .timeout(request_timeout)
            .build()
            .expect("reqwest blocking client");
        Self {
            client,
            connect_timeout,
            request_timeout,
        }
    }

    pub fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
    }
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self::with_timeouts(DEFAULT_CONNECT_TIMEOUT, DEFAULT_REQUEST_TIMEOUT)
    }
}

impl HttpClient for ReqwestHttpClient {
    fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
        let mut builder = self.client.get(&request.url);
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
    ) -> AppResult<()> {
        ensure_download_not_cancelled(cancellation)?;
        let mut builder = self.client.get(&request.url);
        for header in &request.headers {
            builder = builder.header(&header.name, &header.value);
        }
        let mut response = builder.send()?.error_for_status()?;
        write_response_to_path(&mut response, &request.destination, cancellation)
    }
}

fn write_response_to_path(
    response: &mut reqwest::blocking::Response,
    destination: &Path,
    cancellation: &dyn CancellationToken,
) -> AppResult<()> {
    ensure_download_not_cancelled(cancellation)?;
    let mut file = File::create(destination)?;
    let mut buffer = [0u8; DOWNLOAD_BUFFER_SIZE];
    loop {
        ensure_download_not_cancelled(cancellation)?;

        let bytes_read = response.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
    }
    Ok(())
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
