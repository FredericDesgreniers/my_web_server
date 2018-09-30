#![feature(try_from)]
#![feature(const_str_as_bytes)]

#[macro_use]
extern crate failure;

#[macro_use]
extern crate http;

use flate2::write::GzEncoder;
use flate2::Compression;
use http::{Request, RequestBuilder, RequestType};
use router::{Endpoint, Router};
use std::convert::TryFrom;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

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

const TEXT_HEADER: &[u8] = response_head!(
    "200 OK",
    header("Content-Type", "text/html charset=UTF-8"),
    header("Content-Encoding", "gzip"),
    header("Cache-Control", "max-age=1800"),
    header("Cache-Control", "public")
).as_bytes();

const ICON_HEADER: &[u8] = response_head!(
    "200 OK",
    header("Content-Type", "image/x-icon"),
    header("Content-Encoding", "gzip"),
    header("Cache-Control", "max-age=1800"),
    header("Cache-Control", "public")
).as_bytes();

impl HttpRouteInfo {
    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn writer(&mut self) -> &mut impl Write {
        &mut self.writer
    }

    /// Respond with a 202 ok with the given body of content
    pub fn ok(mut self, content: &[u8]) -> Result<(), HttpServerError> {
        self.writer.write(TEXT_HEADER)?;
        self.writer
            .write(&format!("Content-Length: {}\r\n", content.len()).into_bytes())?;
        self.writer.write(b"\r\n")?;
        self.writer.write(content)?;

        Ok(())
    }

    pub fn icon(mut self, content: &[u8]) -> Result<(), HttpServerError> {
        self.writer.write(ICON_HEADER)?;
        self.writer
            .write(&format!("Content-Length: {}\r\n", content.len()).into_bytes())?;
        self.writer.write(b"\r\n")?;
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
    pub fn listen(self, worker_num: usize) -> Result<(), HttpServerError> {
        let HttpServer { listener, router } = self;
        let router = Arc::new(router);

        let workers = pool::ThreadPool::new(worker_num, router);

        for stream in listener.incoming() {
            let stream = stream?;

            workers.do_work(move |router: &Arc<Router<HttpRouteInfo, ()>>| {
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
        router: &Arc<Router<HttpRouteInfo, ()>>,
    ) -> Result<(), HttpServerError> {
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;

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

        let mut persist = true;

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

                match name.to_lowercase().trim() {
                    "connection" => match value.to_lowercase().trim() {
                        "close" => {
                            persist = false;
                        }
                        _ => (),
                    },
                    _ => (),
                }

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

        if persist {
            Self::handle_connection(stream, router)?;
        }

        Ok(())
    }
}
