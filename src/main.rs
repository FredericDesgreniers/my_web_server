#![feature(nll)]

#[macro_use]
extern crate log;

use chrono::prelude::*;
use core::time::Duration;
use http_server::{compress_html, gzip, HttpRouteInfo};
use log::{Level, LevelFilter, Metadata, Record};
use router::{Endpoint, RoutedInfo};
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
struct StaticPage(Vec<u8>);

impl Endpoint<HttpRouteInfo, ()> for StaticPage {
    fn process(&self, route_info: RoutedInfo<HttpRouteInfo>) {
        route_info.data.ok(&self.0).unwrap();
    }
}

struct StaticIcon(Vec<u8>);

impl Endpoint<HttpRouteInfo, ()> for StaticIcon {
    fn process(&self, route_info: RoutedInfo<HttpRouteInfo>) -> () {
        route_info.data.icon(&self.0).unwrap();
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
            StaticPage(compress_html(include_str!("../static/landing_page.html"))),
        );
        server.add_route(
            "/favicon.ico",
            StaticIcon(gzip(include_bytes!("../static/favicon.ico"))),
        );

        server.router_mut().set_endpoint_404(Page404::create());

        let result = server.listen(20);
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
