use crate::http::error::BadRequest;
use crate::http::response::{Response, CRLF};
use std::io::{self, Write};
use std::net::TcpStream;

pub mod error;
pub mod request;
pub mod response;

#[derive(PartialEq, Eq, Debug)]
pub enum Method {
    Get,
    Post,
    // Put,
    // ...
}

impl TryFrom<&'_ str> for Method {
    type Error = BadRequest;

    fn try_from(value: &'_ str) -> Result<Self, BadRequest> {
        match value {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            // "PUT" => Ok(Self::Put),
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
    pub fn write_to(self, mut writer: impl Write) -> io::Result<()> {
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

pub const READING_MEMORY: &str = "Reading a slice is infallable";

pub trait HTTPCarrier {
    fn respond(&mut self, response: Response) -> io::Result<()>;
}

impl HTTPCarrier for TcpStream {
    fn respond(&mut self, response: Response) -> io::Result<()> {
        response.write_to(&mut *self)?;
        self.flush()
    }
}

pub trait WriteHeader: Write {
    fn write_header(&mut self, key: impl AsRef<[u8]>, value: &[u8]) -> io::Result<()> {
        self.write_all(key.as_ref())?;
        self.write_all(b": ")?;
        self.write_all(value)?;
        self.write_all(&CRLF)?;
        Ok(())
    }
}

impl<T: Write> WriteHeader for T {}
