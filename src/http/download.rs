use crate::http::IoTools;
use graph_error::GraphFailure;
use graph_error::{GraphError, GraphResult};
use reqwest::{RequestBuilder, Response};
use std::convert::TryFrom;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

pub trait Download {
    fn download(&mut self) -> GraphResult<FetchClient>;
}

/// The FetchBuilder provides an abstraction for downloading files.
pub struct FetchClient {
    path: PathBuf,
    token: String,
    target_url: String,
    file_name: Option<OsString>,
    extension: Option<String>,
    redirect: Option<RequestBuilder>,
    client: reqwest::Client,
}

impl Default for FetchClient {
    fn default() -> Self {
        FetchClient {
            path: Default::default(),
            token: Default::default(),
            target_url: Default::default(),
            file_name: None,
            extension: None,
            redirect: None,
            client: reqwest::Client::new(),
        }
    }
}

impl FetchClient {
    pub fn new(target_url: &str, path: PathBuf, token: &str) -> FetchClient {
        FetchClient {
            path,
            token: token.into(),
            target_url: target_url.into(),
            file_name: None,
            extension: None,
            redirect: None,
            client: reqwest::Client::new(),
        }
    }

    pub fn rename(&mut self, value: OsString) -> &mut Self {
        self.file_name = Some(value);
        self
    }

    pub fn set_extension(&mut self, value: &str) -> &mut Self {
        self.extension = Some(value.into());
        self
    }

    pub fn directory(&self) -> &PathBuf {
        &self.path
    }

    pub fn file_name(&self) -> Option<&OsString> {
        self.file_name.as_ref()
    }

    pub fn extension(&self) -> Option<&String> {
        self.extension.as_ref()
    }

    pub fn send(&mut self) -> GraphResult<PathBuf> {
        self.download(self.path.clone())
    }

    pub(crate) fn set_redirect(&mut self, redirect: RequestBuilder) {
        self.redirect = Some(redirect);
    }

    fn parse_content_disposition(&mut self, header: &str) -> Option<OsString> {
        let mut v: Vec<&str> = header.split(';').collect();
        v.retain(|s| !s.is_empty());

        // The filename* indicates that the filename is encoded
        if let Some(value) = v.iter().find(|s| s.starts_with("filename*=utf-8''")) {
            let s = value.replace("filename*=utf-8''", "");
            if let Ok(s) = percent_encoding::percent_decode(s.as_bytes()).decode_utf8() {
                return Some(OsString::from(s.to_string()));
            }
        }

        if let Some(value) = v.last() {
            if value.starts_with("filename=") {
                let s = value.replace("\"", "");
                let s = s.replace("filename=", "");
                return Some(OsString::from(s.to_string()));
            }
        }
        None
    }

    fn download<P: AsRef<Path>>(&mut self, directory: P) -> GraphResult<PathBuf> {
        // Create the directory if it does not exist.
        IoTools::create_dir(&directory)?;

        if let Some(redirect) = self.redirect.take() {
            let mut response = redirect.send()?;
            let status = response.status().as_u16();
            if GraphError::is_error(status) {
                return Err(GraphFailure::try_from(&mut response).unwrap_or_default());
            }
            self.target_url = response.url().as_str().to_string();
        }

        let mut response = self
            .client
            .get(self.target_url.as_str())
            .bearer_auth(self.token.as_str())
            .send()?;

        let status = response.status().as_u16();
        if GraphError::is_error(status) {
            return Err(GraphFailure::from(
                GraphError::try_from(&mut response).unwrap_or_default(),
            ));
        }

        // If a filename was specified beforehand.
        if let Some(name) = self.file_name.as_ref() {
            if name.len() <= 255 {
                let path = directory.as_ref().join(name);
                return self.finish((path, response));
            }
        }

        // The content-disposition header, if available, may include the
        // filename either in its normal form or percent encoded.
        if let Some(value) = response.headers().get("content-disposition") {
            if let Ok(s) = std::str::from_utf8(value.as_ref()) {
                if let Some(name) = self.parse_content_disposition(s) {
                    if name.len() <= 255 {
                        let path = directory.as_ref().join(name);
                        return self.finish((path, response));
                    }
                }
            }
        }

        // This is a last ditch effort to find the file name and it
        // may not be the correct one.
        if let Some(name) = response
            .url()
            .path_segments()
            .and_then(std::iter::Iterator::last)
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
        {
            if name.len() <= 255 {
                let path = directory.as_ref().join(name);
                return self.finish((path, response));
            }
        }

        Err(GraphFailure::none_err(
            "Could not determine file name or the file name exceeded 255 characters",
        ))
    }

    fn finish(&mut self, values: (PathBuf, Response)) -> GraphResult<PathBuf> {
        if let Some(ext) = self.extension.as_ref() {
            values.0.with_extension(ext.as_str());
        }
        IoTools::copy(values)
    }
}