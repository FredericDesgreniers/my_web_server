use crate::Headers;

/// Generate an HTTP response header at compile time.
/// Input takes the form
/// ```
/// use http::response_head;
///
/// response_head!("101 SOME CODE",
///     header("Content-Type", "text/html"),
///     header("Content-Encoding", "gzip")
/// );
/// ```
/// and returns a fully formed string containing all the information provided.
///
/// Using nightly const `as_bytes()` on the string allows a 100% compile time generation of headers
///
#[macro_export]
macro_rules! response_head {
    ($code: expr, $(header($key:expr, $value: expr)),*) => {
        concat!("HTTP/1.1 ",$code,"\r\n", $($key,":",$value,"\r\n",)*)
    }

}

/// HTTP response
#[derive(Debug)]
pub struct Response {
    headers: Headers,
    /// Code such as 404 or 200
    code: String,
    /// Response body
    body: Vec<u8>,
}

impl Response {
    pub fn with_code(code: &str) -> Self {
        Self {
            headers: Headers::default(),
            code: code.to_string(),
            body: Vec::new(),
        }
    }

    pub fn body(&self) -> &Vec<u8> {
        &self.body
    }
    pub fn head_bytes(&self) -> Vec<u8> {
        let mut head = Vec::new();

        head.extend_from_slice(&format!("HTTP/1.1 {}\r\n", self.code).into_bytes());

        for (name, value) in self.headers.iter() {
            head.extend_from_slice(
                &format!("{name}:{value}\r\n", name = name, value = value).into_bytes(),
            );
        }

        head
    }
}

pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    pub fn ok_200() -> Self {
        Self {
            response: Response::with_code("200 OK"),
        }
    }

    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        self.response
            .headers
            .add(name.to_string(), value.to_string());
        self
    }

    pub fn code(&mut self, code: &str) -> &mut Self {
        self.response.code = code.to_string();
        self
    }
    pub fn body(&mut self, body: Vec<u8>) -> &mut Self {
        self.response.body = body;
        self
    }

    pub fn build(self) -> Response {
        self.response
    }
}