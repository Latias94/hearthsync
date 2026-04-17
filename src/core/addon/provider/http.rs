use std::fs::File;
use std::path::{Path, PathBuf};

use reqwest::blocking::Client;

use crate::core::error::AppResult;

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

    fn download_to_path(&self, request: HttpDownloadRequest) -> AppResult<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReqwestHttpClient;

impl HttpClient for ReqwestHttpClient {
    fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
        let client = Client::builder().build()?;
        let mut builder = client.get(&request.url);
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

    fn download_to_path(&self, request: HttpDownloadRequest) -> AppResult<()> {
        let client = Client::builder().build()?;
        let mut builder = client.get(&request.url);
        for header in &request.headers {
            builder = builder.header(&header.name, &header.value);
        }
        let mut response = builder.send()?.error_for_status()?;
        write_response_to_path(&mut response, &request.destination)
    }
}

fn write_response_to_path(
    response: &mut reqwest::blocking::Response,
    destination: &Path,
) -> AppResult<()> {
    let mut file = File::create(destination)?;
    response.copy_to(&mut file)?;
    Ok(())
}
