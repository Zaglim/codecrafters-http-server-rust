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
    type Error = ();

    fn try_from(value: &'_ [u8]) -> Result<Self, ()> {
        match value {
            b"GET" => Ok(Self::Get),
            b"POST" => Ok(Self::Post),
            b"PUT" => Ok(Self::Put),
            bytes => {
                error!(
                    "\"{}\" is not a recognised method",
                    String::from_utf8_lossy(bytes)
                );
                Err(())
            }
        }
    }
}

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
    type Error = ();

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        match value {
            b"HTTP/1.1" => Ok(Self::Ver1_1),
            b"HTTP/2.0" => Ok(Self::Ver2_0),
            bytes => {
                error!(
                    "\"{}\" is not a recognised version",
                    String::from_utf8_lossy(bytes)
                );
                Err(())
            }
        }
    }
}

pub struct Header {
    key: Box<[u8]>,
    value: Box<[u8]>,
}

impl TryFrom<Vec<u8>> for Header {
    type Error = ();

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let split_pos = bytes.windows(2).position(|b| b == b": ").ok_or(())?;
        let key = &bytes[..split_pos];
        let value = &bytes[split_pos + 2..];

        Ok(Header {
            key: key.into(),
            value: value.into(),
        })
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
    pub fn new_bad_request() -> Response {
        Response {
            status: ResponseStatus::BadRequest,
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
            vec.extend(&header.key);
            vec.extend(b": ");
            vec.extend(&header.value);
            vec.extend(b"\r\n");
        }
        // signal end of headers
        vec.extend(b"\r\n");
        vec.extend(body);
        vec
    }
}
