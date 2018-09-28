#![feature(nll)]

extern crate chrono;
extern crate core;
extern crate http;
extern crate http_server;

#[macro_use]
extern crate log;

use core::time::Duration;
use log::{Level, LevelFilter, Metadata, Record};
use std::thread;

use chrono::prelude::*;

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

fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Info))
        .unwrap();

    info!("Server started...");

    loop {
        let mut server = http_server::HttpServer::create(80).unwrap();

        let result = server.listen();

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
