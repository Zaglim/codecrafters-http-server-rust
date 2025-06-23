use core::str;
use std::fmt::Display;

use log::error;

pub mod request;

#[derive(PartialEq, Eq, Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    // ...
}

impl TryFrom<&'_ [u8]> for Method {
    type Error = String;

    fn try_from(value: &'_ [u8]) -> Result<Self, String> {
        match value {
            b"GET" => Ok(Self::Get),
            b"POST" => Ok(Self::Post),
            b"PUT" => Ok(Self::Put),
            bytes => {
                let error = format!(
                    "\"{}\" is not a recognised method",
                    String::from_utf8_lossy(bytes)
                );
                Err(error)
            }
        }
    }
}

#[derive(Debug)]
pub enum Version {
    // ..
    Ver1_1,
    Ver2_0,
    // ..
}
impl Version {
    fn serialize(&self) -> &[u8] {
        match self {
            Self::Ver1_1 => b"HTTP/1.1",
            Self::Ver2_0 => b"HTTP/2.0",
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for Version {
    type Error = String;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        match value {
            b"HTTP/1.1" => Ok(Self::Ver1_1),
            b"HTTP/2.0" => Ok(Self::Ver2_0),
            bytes => {
                let e = format!(
                    "\"{}\" is not a recognised HTTP version",
                    String::from_utf8_lossy(bytes)
                );
                error!("{e}");
                Err(e)
            }
        }
    }
}
#[derive(Debug)]
pub struct Header {
    key: Box<str>,
    value: Box<str>,
}

impl TryFrom<Vec<u8>> for Header {
    type Error = String;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let split_pos = bytes
            .windows(2)
            .position(|b| b == b": ")
            .ok_or("expecting key and value seperated by \": \"")?;
        let key: Box<str> = String::from_utf8_lossy(&bytes[..split_pos]).into();
        let value: Box<str> = String::from_utf8_lossy(&bytes[split_pos + 2..]).into();

        Ok(Header { key, value })
    }
}

pub enum ResponseStatus {
    BadRequest, // 400
    NotFound,
    Ok,          // 200
    ServerError, // 300?
}
impl ResponseStatus {
    fn serialize(&self) -> &'static [u8] {
        match self {
            Self::BadRequest => b"400 Bad Request",
            Self::NotFound => b"404 Not Found",
            Self::Ok => b"200 OK",
            Self::ServerError => b"500 Internal Server Error",
        }
    }
}

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
    fn new_server_error() -> Response {
        Response {
            status: ResponseStatus::ServerError,
            ..Default::default()
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let Response {
            version,
            status,
            headers,
            body,
        } = self;
        let mut vec = Vec::new();

        // first line
        vec.extend(version.serialize());
        vec.push(b' ');
        vec.extend(status.serialize());
        vec.extend(b"\r\n");

        // headers
        for header in headers {
            vec.extend(header.key.bytes());
            vec.extend(b": ");
            vec.extend(header.value.bytes());
            vec.extend(b"\r\n");
        }
        // signal end of headers
        vec.extend(b"\r\n");
        vec.extend(body);
        vec
    }

    fn echo(str: &[u8]) -> Response {
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
