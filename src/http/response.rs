use crate::http::{error, Header, ResponseStatus, Version};
use std::io::{self, BufWriter, ErrorKind, Write};

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

impl From<error::BadRequest> for Response {
    fn from(error: error::BadRequest) -> Self {
        Response {
            status: ResponseStatus::BadRequest,
            headers: vec![Header {
                key: "Cause".into(),
                value: error.to_string().into(),
            }],
            ..Self::default()
        }
    }
}

pub mod success {
    use crate::http::response::Response;
    use crate::http::{Header, ResponseStatus};
    pub fn plain_text(str: String) -> Response {
        let headers = Vec::from_iter([
            Header::content_type("text/plain"),
            Header::content_length(&str),
        ]);
        let body = str.into_bytes();

        Response {
            headers,
            body,
            ..Default::default()
        }
    }
    pub fn octet_stream(body: Vec<u8>) -> Response {
        let headers = Vec::from_iter([
            Header::content_type("application/octet-stream"),
            Header::content_length(&body),
        ]);
        Response {
            headers,
            body,
            ..Default::default()
        }
    }

    pub fn created() -> Response {
        Response {
            status: ResponseStatus::Created,
            ..Default::default()
        }
    }
}

pub mod server_error {
    use crate::http::response::Response;
    use crate::http::ResponseStatus;

    pub fn generic() -> Response {
        Response {
            status: ResponseStatus::ServerError,
            ..Default::default()
        }
    }
}

pub mod client_error {
    use crate::http::response::Response;
    use crate::http::ResponseStatus;

    pub mod bad_request {
        use crate::http::response::Response;
        use crate::http::{Header, ResponseStatus};

        fn generic() -> Response {
            Response {
                status: ResponseStatus::BadRequest,
                ..Default::default()
            }
        }

        fn with_cause(cause: &'static str) -> Response {
            let mut response = generic();
            response.headers.push(Header {
                key: "Cause".into(),
                value: cause.into(),
            });
            response
        }

        pub(crate) fn missing_method() -> Response {
            with_cause("Missing method")
        }
    }

    pub(crate) fn not_found() -> Response {
        Response {
            status: ResponseStatus::NotFound,
            ..Default::default()
        }
    }
}

impl From<io::Error> for Response {
    fn from(io_err: io::Error) -> Response {
        use ErrorKind as EK;
        match io_err.kind() {
            EK::NotFound | EK::PermissionDenied | EK::IsADirectory => client_error::not_found(),
            _ => server_error::generic(),
        }
    }
}
