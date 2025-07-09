use crate::encoding::Encoding;
use crate::http::response::content_type::ContentType;
use crate::http::{error, Version, WriteHeader};
use std::collections::HashMap;
use std::io::{self, BufWriter, Write};

pub struct Response {
    version: Version,
    status: ResponseStatus, // serialization includes code and message
    dyn_headers: HashMap<Box<str>, Box<str>>,
    body_data: Option<BodyData>,
}

pub struct BodyData {
    content_type: ContentType,
    opt_encoding: Option<Encoding>,
    body: Vec<u8>, // "Content-Length" header is generated from this
}

mod content_type {
    pub enum ContentType {
        Application(Application),
        Text(Text),
    }

    impl ContentType {
        pub(crate) fn as_text(&self) -> &[u8] {
            match self {
                ContentType::Application(a) => match a {
                    Application::OctetStream => b"application/octet-stream",
                },
                ContentType::Text(t) => match t {
                    Text::Plain => b"text/plain",
                },
            }
        }
    }

    pub enum Application {
        OctetStream,
    }
    pub enum Text {
        Plain,
    }
}

pub enum ResponseStatus {
    BadRequest,
    NotFound,
    Ok,
    ServerError,
    Created,
}

impl ResponseStatus {
    pub fn write_to(self, writer: &mut BufWriter<impl Write>) -> io::Result<()> {
        writer.write_all(self.serialize().as_bytes())
    }

    fn serialize(self) -> &'static str {
        match self {
            Self::Ok => "200 OK",
            Self::Created => "201 Created",
            Self::BadRequest => "400 Bad Request",
            Self::NotFound => "404 Not Found",
            Self::ServerError => "500 Internal Server Error",
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Response {
            version: Version::Ver1_1,
            status: ResponseStatus::Ok,
            dyn_headers: HashMap::new(),
            body_data: None,
        }
    }
}

pub const CRLF: [u8; 2] = [b'\r', b'\n'];

impl Response {
    pub fn write_to(self, stream: impl Write) -> io::Result<()> {
        let Response {
            version,
            status,
            dyn_headers,
            body_data,
        } = self;

        let mut writer = BufWriter::new(stream);

        // Write first line
        version.write_to(&mut writer)?;
        writer.write_all(b" ")?;
        status.write_to(&mut writer)?;
        writer.write_all(&CRLF)?;

        // Write headers (excluding body related headers)
        for (key, value) in dyn_headers {
            writer.write_header(key.as_bytes(), value.as_bytes())?;
        }

        if let Some(BodyData {
            content_type,
            opt_encoding,
            body,
        }) = body_data
        {
            // add body related headers
            writer.write_header("Content-Type", content_type.as_text())?;
            writer.write_header("Content-Length", body.len().to_string().as_bytes())?;
            if let Some(encoding) = opt_encoding {
                writer.write_header(
                    b"Content-Encoding",
                    match encoding {
                        Encoding::Gzip => b"gzip",
                    },
                )?;
            }
            // Signal end of headers
            writer.write_all(&CRLF)?;

            writer.write_all(&body)?;
        } else {
            // Signal end of headers
            writer.write_all(&CRLF)?;
        }
        Ok(())
    }
}

impl From<error::BadRequest> for Response {
    fn from(error: error::BadRequest) -> Self {
        Response {
            status: ResponseStatus::BadRequest,
            dyn_headers: HashMap::from([("Cause".into(), error.to_string().into())]),
            ..Self::default()
        }
    }
}

pub mod success {
    use crate::{
        encoding::{read_and_encode, Encoding},
        http::{
            response::{
                content_type::{Application::OctetStream, ContentType, Text::Plain},
                BodyData, Response, ResponseStatus,
            },
            READING_MEMORY,
        },
    };

    pub fn plain_text(str: String, opt_encoding: Option<Encoding>) -> Response {
        let body = if let Some(encoding) = opt_encoding {
            read_and_encode(str.as_bytes(), encoding).expect(READING_MEMORY)
        } else {
            str.into_bytes()
        };

        let body_data = BodyData {
            content_type: ContentType::Text(Plain),
            opt_encoding,
            body,
        };

        Response {
            body_data: Some(body_data),
            ..Default::default()
        }
    }
    pub fn octet_stream(unencoded_body: Vec<u8>, opt_encoding: Option<Encoding>) -> Response {
        // let mut headers = HashM[
        //     Header::content_type("application/octet-stream"),
        //     Header::content_length(&body),
        // ];

        let body = if let Some(encoding) = opt_encoding {
            read_and_encode(unencoded_body.as_slice(), encoding).expect(READING_MEMORY)
        } else {
            unencoded_body
        };

        let body_data = BodyData {
            content_type: ContentType::Application(OctetStream),
            opt_encoding,
            body,
        };

        Response {
            body_data: Some(body_data),
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
    use crate::http::response::{Response, ResponseStatus};

    pub fn generic() -> Response {
        Response {
            status: ResponseStatus::ServerError,
            ..Default::default()
        }
    }
}

pub mod client_error {
    use crate::http::response::{Response, ResponseStatus};

    pub fn not_found() -> Response {
        Response {
            status: ResponseStatus::NotFound,
            ..Default::default()
        }
    }
}
