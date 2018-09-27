#![feature(nll)]

extern crate core;
extern crate http;
extern crate http_server;

use core::time::Duration;
use std::thread;

fn main() {

	println!("Server started...");

    loop {
        let mut server = http_server::HttpServer::create(80).unwrap();

        let result = server.listen();

        if let Err(err) = result {
            println!(
                "Server ended in error, starting it up again in 5 seconds. Error: {:?}",
                err
            );
            thread::sleep(Duration::from_secs(5));
        } else {
            break;
        }
    }

    println!("Server has been closed.");
}
