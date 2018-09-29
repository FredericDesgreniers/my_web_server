#![feature(try_from)]

use core::convert::TryFrom;
use std::collections::HashMap;
use std::fmt::{self, Display};

/// Contains (key, value) headers
#[derive(Default, Debug)]
pub struct Headers {
    headers: HashMap<String, String>,
}

/// HTTP request type
#[derive(Debug, Copy, Clone)]
pub enum RequestType {
    GET,
    POST,
}

impl Display for RequestType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let type_display = match self {
            RequestType::GET => "GET",
            RequestType::POST => "POST",
        };

        write!(f, "{}", type_display)
    }
}

impl TryFrom<&str> for RequestType {
    // This is expected to fail, so we treat the error / result as an option
    type Error = ();

    fn try_from(from: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
        match from.trim().to_lowercase().as_str() {
            "get" => Ok(RequestType::GET),
            "post" => Ok(RequestType::POST),
            _ => Err(()),
        }
    }
}

/// HTTP request
#[derive(Debug)]
pub struct Request {
    request_type: RequestType,
    /// This should either be an IP or resolve to one
    host: String,
    /// Port to send the request too.
    /// This is only relevant when constructed by sender
    port: usize,
    /// Request path: e.g /home
    path: String,
    /// Request headers
    headers: Headers,
}

/// Builds an HTTP request
pub struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    pub fn new(request_type: RequestType, host: &str) -> Self {
        Self {
            request: Request {
                request_type,
                host: host.to_string(),
                port: 80,
                path: "/".to_string(),
                headers: Headers::default(),
            },
        }
    }

    pub fn port(&mut self, port: usize) -> &mut Self {
        self.request.port = port;
        self
    }

    pub fn path(&mut self, path: &str) -> &mut Self {
        self.request.path = path.to_string();
        self
    }

    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        self.request
            .headers
            .headers
            .insert(name.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> Request {
        self.request
    }
}

/// HTTP response
#[derive(Default, Debug)]
pub struct Response {
    headers: Headers,
    body: Vec<u8>,
}

impl Response {
    pub fn body(&self) -> &Vec<u8> {
        &self.body
    }
}

#[derive(Default)]
pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        self.response
            .headers
            .headers
            .insert(name.to_string(), value.to_string());
        self
    }

    pub fn body(&mut self, body: Vec<u8>) -> &mut Self {
        self.response.body = body;
        self
    }

    pub fn build(self) -> Response {
        self.response
    }
}
