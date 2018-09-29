#![feature(try_from)]

#[macro_use]
extern crate failure;

use flate2::write::GzEncoder;
use flate2::Compression;
use http::{Request, RequestBuilder, RequestType};
use router::{Endpoint, Router};
use std::convert::TryFrom;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

/// Minifies and gzips html
pub fn compress_html(html: &str) -> Vec<u8> {
    let minified_content = minify::html::minify(html);
    gzip(&minified_content.into_bytes())
}

pub fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// An http server that takes care of accepting connections and serving them with content
pub struct HttpServer {
    listener: TcpListener,
    router: Router<HttpRouteInfo, ()>,
}

/// Info that needs to be routed to an endpoint
#[derive(Debug)]
pub struct HttpRouteInfo {
    request: Request,
    writer: TcpStream,
}

impl HttpRouteInfo {
    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn writer(&mut self) -> &mut impl Write {
        &mut self.writer
    }

    /// Respond with a 202 ok with the given body of content
    pub fn ok(mut self, content: &[u8]) -> Result<(), HttpServerError> {
        self.writer.write(b"HTTP/1.1 200 OK\r\nContent-Type: text/html charset=UTF-8\r\nContent-Encoding: gzip\r\nConnection: close\r\n\r\n")?;
        self.writer.write(content)?;
        self.writer.write(b"\r\n")?;

        Ok(())
    }

    pub fn icon(mut self, content: &[u8]) -> Result<(), HttpServerError> {

        self.writer.write(b"HTTP/1.1 200 OK\r\nContent-Type: image/x-icon\r\nContent-Encoding: gzip\r\nConnection:Close\r\n\r\n")?;
        self.writer.write(content)?;
        Ok(())
    }
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
    pub fn create(port: usize) -> Result<Self, HttpServerError> {
        Ok(Self {
            listener: TcpListener::bind(&format!("0.0.0.0:{}", port))?,
            router: Router::default(),
        })
    }

    /// Listen and respond to incoming http requests
    pub fn listen(self) -> Result<(), HttpServerError> {
        let HttpServer { listener, router } = self;
        let router = Arc::new(router);

        let workers = pool::ThreadPool::new(10);

        for stream in listener.incoming() {
            let stream = stream?;
            let router = router.clone();
            workers.do_work(move || {
                if let Err(err) = Self::handle_connection(stream, router) {
                    println!("Error in request: {:?}", err);
                }
            });
        }
        Ok(())
    }

    pub fn add_route(
        &mut self,
        path: impl Into<router::RouterPath>,
        endpoint: impl Endpoint<HttpRouteInfo, ()> + 'static,
    ) {
        self.router.add_path(path, endpoint);
    }

    /// Handles an incoming connection
    /// Parses the request and responds
    fn handle_connection(
        mut stream: TcpStream,
        router: Arc<Router<HttpRouteInfo, ()>>,
    ) -> Result<(), HttpServerError> {
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

            if line.trim().is_empty() {
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

        let request = request.build();

        // If no route is found, server with a generic 404 page.
        if let None = router.route(
            path,
            HttpRouteInfo {
                writer: stream.try_clone()?,
                request,
            },
        ) {
            stream.write(b"HTTP/1.1 404 NOT FOUND\r\nContent-Type: text/html charset=UTF-8\r\nContent-Encoding: gzip\r\nConnection: close\r\n\r\n")?;
            stream.write(&compress_html("Could not find resource"))?;
            stream.write(b"\r\n")?;
        }

        Ok(())
    }
}
