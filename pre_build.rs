#![feature(const_str_as_bytes)]

use flate2::write::GzEncoder;
use flate2::Compression;
use http::make_response;
use std::fs::{read_to_string, File, create_dir_all, remove_dir_all};
use std::io::{Write, Read};
use walkdir::WalkDir;

fn main() {

    remove_dir_all("./static_out");
    create_dir_all("./static_out");

    for entry in WalkDir::new("./static") {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let name = entry.file_name().to_str().unwrap();
            if let Some(index) = name.rfind('.') {
				let (name, extension) = name.split_at(index);
                let output = match extension.trim() {
                    ".html" => {
                        let html = read_to_string(entry.path()).unwrap();
                        let response = make_response!(HTML: "202 OK", &html);
                        Some(response.to_vec())
                    }
                    ".ico" => {
                        let mut icon = Vec::new();
                        let mut file = File::open(entry.path()).unwrap();
                        file.read_to_end(&mut icon).unwrap();

                        let content = gzip(&icon);
                        let response = make_response!(ICON: "202 OK", content);

						Some(response.to_vec())
                    }
                    _ => None,
                };

				if let Some(output) = output {
                    let path = format!("./static_out/{}_{}.http", name, &extension[1..]);
					let mut file_out = File::create(path).unwrap();
                    file_out.write_all(&output).unwrap();
				} else {
                    panic!("Unsurported static file: {}", extension);
                }

            }
        }
    }
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
