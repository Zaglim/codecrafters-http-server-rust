use std::fmt::Display;

use crate::http::{Header, ResponseStatus, Version};
use std::io::{self, BufWriter, Write};

pub struct Response {
    version: Version,
    status: ResponseStatus, // serialization includes code and message
    headers: Vec<Header>,
    body: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            version: Version::Ver1_1,
            status: ResponseStatus::Ok,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }
}

impl Response {
    pub fn write_to(self, stream: impl Write) -> io::Result<()> {
        let mut writer = BufWriter::new(stream);

        // Write first line
        self.version.write_to(&mut writer)?;
        writer.write_all(b" ")?;
        self.status.write_to(&mut writer)?;
        writer.write_all(b"\r\n")?;

        // Write headers
        for header in &self.headers {
            writer.write_all(header.key.as_bytes())?;
            writer.write_all(b": ")?;
            writer.write_all(header.value.as_bytes())?;
            writer.write_all(b"\r\n")?;
        }

        // Signal end of headers
        writer.write_all(b"\r\n")?;

        writer.write_all(&self.body)?;

        writer.flush()
    }
}


/// resopnse creation
impl Response {
    pub fn bad_request(cause: impl Display) -> Response {
        Response {
            status: ResponseStatus::BadRequest,
            headers: [Header {
                key: "cause".into(),
                value: cause.to_string().into(),
            }]
            .into(),
            ..Default::default()
        }
    }
    pub fn new_server_error() -> Response {
        Response {
            status: ResponseStatus::ServerError,
            ..Default::default()
        }
    }

    pub fn not_found() -> Response {
        Response {
            status: ResponseStatus::NotFound,
            ..Default::default()
        }
    }

    pub fn echo(str: &[u8]) -> Response {
        let headers = Vec::from_iter([
            Header {
                key: "Content-Type".into(),
                value: "text/plain".into(),
            },
            Header {
                key: "Content-Length".into(),
                value: str.len().to_string().into(),
            },
        ]);
        let body = str.to_vec();

        Response {
            headers,
            body,
            ..Default::default()
        }
    }
}
