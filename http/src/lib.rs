#![feature(const_str_as_bytes)]
#![feature(try_from)]

#[macro_use]
pub mod response;
pub mod request;

pub use self::request::*;
pub use self::response::*;

use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

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

    pub fn iter(&self) -> impl Iterator<Item = &Header> {
        self.headers.iter()
    }
}

pub fn compress_html_into(html: &str, buffer: &mut Vec<u8>) {
    let minified_html = minify::html::minify(html);
    gzip_into(minified_html.as_bytes(), buffer);
}

pub fn gzip_into(data: &[u8], buffer: &mut Vec<u8>) {
    let mut encoder = GzEncoder::new(buffer, Compression::default());
    encoder.write_all(data).unwrap();
    let _ = encoder.finish().unwrap();
}

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
