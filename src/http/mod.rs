use crate::http::error::BadRequest;
use core::str;
use std::io::{self, BufWriter, Write};

pub mod error;
pub mod request;
pub mod response;

#[derive(PartialEq, Eq, Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    // ...
}

impl TryFrom<&'_ str> for Method {
    type Error = BadRequest;

    fn try_from(value: &'_ str) -> Result<Self, BadRequest> {
        match value {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            other => {
                log::error!("received unrecognised method: \"{other}\"");
                Err(BadRequest::UnsupportedMethod)
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
    pub(crate) fn write_to<T: Write>(self, writer: &mut BufWriter<T>) -> io::Result<()> {
        writer.write_all(self.serialize())
    }
}

impl Version {
    fn serialize(self) -> &'static [u8] {
        match self {
            Self::Ver1_1 => b"HTTP/1.1",
            Self::Ver2_0 => b"HTTP/2.0",
        }
    }
}

impl<'a> TryFrom<&'a str> for Version {
    type Error = BadRequest;

    fn try_from(value: &'a str) -> Result<Self, BadRequest> {
        match value {
            "HTTP/1.1" => Ok(Self::Ver1_1),
            "HTTP/2.0" => Ok(Self::Ver2_0),
            _other => Err(BadRequest::UnsupportedHTTPVersion),
        }
    }
}
#[derive(Debug)]
pub struct Header {
    key: Box<str>,
    value: Box<str>,
}

impl Header {
    pub fn content_type(value: impl Into<Box<str>>) -> Header {
        Header {
            key: "Content-Type".into(),
            value: value.into(),
        }
    }

    pub fn content_length(content: impl AsRef<[u8]>) -> Header {
        Header {
            key: "Content-Length".into(),
            value: content.as_ref().len().to_string().into_boxed_str(),
        }
    }
}

impl TryFrom<Vec<u8>> for Header {
    type Error = BadRequest;

    fn try_from(bytes: Vec<u8>) -> Result<Self, BadRequest> {
        let split_pos = bytes
            .windows(2)
            .position(|b| b == b": ")
            .ok_or(BadRequest::MalformedHeader)?;
        let key: Box<str> = String::from_utf8_lossy(&bytes[..split_pos]).into();
        let value: Box<str> = String::from_utf8_lossy(&bytes[split_pos + 2..]).into();

        Ok(Header { key, value })
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
