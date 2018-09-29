#![feature(try_from)]

use core::convert::TryFrom;
use std::collections::HashMap;
use std::fmt::{self, Display};

/// Contains (key, value) headers
#[derive(Default, Debug)]
pub struct Headers {
    headers: Vec<Header>,
}

pub type Header = (String, String);

impl Headers {
    pub fn add(&mut self, key: String, value: String) {
        self.headers.push((key, value));
    }

    pub fn iter(&self) -> impl Iterator<Item=&Header> {
        self.headers.iter()
    }
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
            .add(name.to_string(), value.to_string());
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
    /// Code such as 404 or 200
    code: String,
    /// Response body
    body: Vec<u8>,
}

impl Response {
    pub fn body(&self) -> &Vec<u8> {
        &self.body
    }
    pub fn head_bytes(&self) -> Vec<u8> {
        let mut head = Vec::new();

        head.extend_from_slice(&format!("HTTP/1.1 {}\r\n", self.code).into_bytes());

        for (name, value) in self.headers.iter() {
            head.extend_from_slice(
                &format!("{name}:{value}\r\n", name = name, value = value).into_bytes(),
            );
        }

        head.extend_from_slice(b"\r\n");

        head
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
            .add(name.to_string(), value.to_string());
        self
    }

    pub fn code(&mut self, code: &str) -> &mut Self {
        self.response.code = code.to_string();
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
