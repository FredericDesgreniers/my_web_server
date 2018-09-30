#![feature(const_str_as_bytes)]

use flate2::write::GzEncoder;
use flate2::Compression;
use http::make_response;
use std::fs::{read_to_string, File, create_dir_all, remove_dir_all};
use std::io::{Write, Read};
use walkdir::WalkDir;

fn main() {

    // Clean up from last build
    remove_dir_all("./static_out");
    create_dir_all("./static_out");

    // Currently, this does not respect sub-directory paths, so everything is outputed to the same directory level.
    // Eventually, this will need to be changed to support more complex static file organization

    for entry in WalkDir::new("./static") {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            let name = entry.file_name().to_str().unwrap();

            // The file extension tells us how to handle the files
            // For example, html is minified and then gzipped, while icons are straight up gzipped
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
                    //TODO Change this to not map out everything in the same directory
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

/// Compress html to the minimum possible size
pub fn compress_html(html: &str) -> Vec<u8> {
    let minified_content = minify::html::minify(html);
    gzip(&minified_content.into_bytes())
}

pub fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}
