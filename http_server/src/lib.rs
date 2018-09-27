#![feature(try_from)]

extern crate http;
#[macro_use]
extern crate failure;
extern crate core;
extern crate pool;

use failure::Error;
use http::RequestBuilder;
use http::RequestType;
use std::convert::TryFrom;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::net::TcpStream;

pub struct HttpServer {
    listener: TcpListener,
}

#[derive(Debug, Fail)]
pub enum HttpServerError {
    #[fail(display = "IO error: {:?}", 0)]
    IoError(std::io::Error),
    #[fail(display = "Http Method not present in request line")]
    HttpMethodNotPresent,
    #[fail(display = "Path not present in request line")]
    PathNotPresent,
}

impl From<std::io::Error> for HttpServerError {
    fn from(err: std::io::Error) -> Self {
        HttpServerError::IoError(err)
    }
}

impl HttpServer {
    /// Create an http server on the specified port
    /// `valid` valid port. Should be 80 for http
    pub fn create(port: usize) -> Result<Self, Error> {
        Ok(Self {
            listener: TcpListener::bind(&format!("0.0.0.0:{}", port))?,
        })
    }

    /// Listen and respond to incoming http requests
    pub fn listen(&mut self) -> Result<(), Error> {
        let workers = pool::ThreadPool::new(10);

        for stream in self.listener.incoming() {
            let stream = stream?;
            workers.do_work(move || {
                if let Err(err) = Self::handle_connection(stream) {
                    println!("Error in request: {:?}", err);
                }
            });
        }
        Ok(())
    }

    /// Handles an incoming connection
    /// Parses the request and responds
    fn handle_connection(mut stream: TcpStream) -> Result<(), HttpServerError> {
        let mut buffered_stream = BufReader::new(stream.try_clone()?);

        // First line of a request, normally in the format "GET / HTTP/1.1"
        let mut request_line = String::new();
        buffered_stream.read_line(&mut request_line)?;

        let mut parts = request_line.split_whitespace();
        let request_type = parts.next().ok_or(HttpServerError::HttpMethodNotPresent)?;
        let path = parts.next().ok_or(HttpServerError::PathNotPresent)?;

        let mut request = RequestBuilder::new(
            RequestType::try_from(request_type).unwrap_or(RequestType::GET),
            "localhost",
        );
        request.path(path);

        // Parse all the headers
        let mut line = String::new();
        loop {
            buffered_stream.read_line(&mut line)?;

            if line.trim().len() == 0 {
                break;
            }

            if let Some(header_split_index) = line.find(":") {
                let (name, value) = line.split_at(header_split_index);
                let value = value[1..].trim();

                request.header(name, value);
            }

            // We reuse the line buffer, so we need to clear it every time
            line.clear();
        }

        let _request = request.build();

        //TODO remove hard-coded thing and move them to the api
        //TODO Once more pages are created, a way to specify different pages should be available
        let content = include_str!("../../static/landing_page.html");

        stream.write(b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nConnection: close\r\n\r\n")?;
        stream.write(content.as_bytes())?;
        stream.write(b"\r\n")?;
        Ok(())
    }
}
