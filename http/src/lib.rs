#![feature(try_from)]

#[macro_use]
pub mod response;
pub mod request;

pub use self::request::*;
pub use self::response::*;

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
