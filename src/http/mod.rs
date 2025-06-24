use core::str;
use log::error;
use std::io;
use std::io::{BufWriter, Write};

pub mod request;
pub mod response;

#[derive(PartialEq, Eq, Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    // ...
}

impl TryFrom<&'_ [u8]> for Method {
    type Error = ();

    fn try_from(value: &'_ [u8]) -> Result<Self, ()> {
        match value {
            b"GET" => Ok(Self::Get),
            b"POST" => Ok(Self::Post),
            b"PUT" => Ok(Self::Put),
            bytes => {
                error!(
                    "received unrecognised method: \"{}\"",
                    String::from_utf8_lossy(bytes)
                );
                Err(())
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

impl<'a> TryFrom<&'a [u8]> for Version {
    type Error = String;

    fn try_from(value: &'a [u8]) -> Result<Self, String> {
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

    fn try_from(bytes: Vec<u8>) -> Result<Self, String> {
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
    pub fn write_to(self, writer: &mut BufWriter<impl Write>) -> io::Result<()> {
        writer.write_all(self.serialize())
    }

    fn serialize(self) -> &'static [u8] {
        match self {
            Self::BadRequest => b"400 Bad Request",
            Self::NotFound => b"404 Not Found",
            Self::Ok => b"200 OK",
            Self::ServerError => b"500 Internal Server Error",
        }
    }
}
