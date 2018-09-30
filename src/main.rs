#![feature(const_str_as_bytes)]
#![feature(nll)]

#[macro_use]
extern crate log;

#[macro_use]
extern crate http;

use chrono::prelude::*;
use core::time::Duration;
use http::{compress_html, gzip};
use http_server::HttpRouteInfo;
use log::{Level, LevelFilter, Metadata, Record};
use router::{Endpoint, RoutedInfo};
use std::io::Write;
use std::thread;

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let datetime = Local::now();
            println!("[{:?}] - {} - {}", datetime, record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

/// Endpoint to serve static content
struct StaticResource(Vec<u8>);

impl Endpoint<HttpRouteInfo, ()> for StaticResource {
    fn process(&self, mut route_info: RoutedInfo<HttpRouteInfo>) {
        route_info.data.writer().write_all(&self.0).unwrap();
    }
}

struct Page404(Vec<u8>);

impl Page404 {
    pub fn create() -> Self {
        Page404(compress_html("Could not find page"))
    }
}

impl Endpoint<HttpRouteInfo, ()> for Page404 {
    fn process(&self, route_info: RoutedInfo<HttpRouteInfo>) -> () {
        route_info.data.not_found_404(&self.0).unwrap();
    }
}

fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();

    info!("Server started...");

    loop {
        let mut server = http_server::HttpServer::create(80).unwrap();

        server.add_route(
            "/",
            StaticResource(include_bytes!("../static_out/landing_page_html.http").to_vec()),
        );
        server.add_route(
            "/favicon.ico",
            StaticResource(include_bytes!("../static_out/favicon_ico.http").to_vec()),
        );

        server.router_mut().set_endpoint_404(Page404::create());

        let result = server.listen(40);
        if let Err(err) = result {
            error!(
                "Server ended in error, starting it up again in 5 seconds. Error: {:?}",
                err
            );
            thread::sleep(Duration::from_secs(5));
        } else {
            break;
        }
    }

    info!("Server has been closed.");
}
