use std::{
    io::{self, BufRead, Read},
    net::TcpStream,
};

use crate::http::*;

pub struct Request {
    method: Method,
    target: Box<[u8]>,
    _http_version: Version,
    _headers: Box<[Header]>,
    _body: Box<[u8]>,
}
impl Request {
    /// TODO try replace with stream.bytes() implementation? (To process as it is coming in)
    pub fn try_read_new<'a>(stream: &mut TcpStream) -> Result<Request, Response> {
        let bytes = {
            let mut vec = Vec::new();
            let mut buffer = [0; 1024];

            // Read until we find the end of headers (\r\n\r\n)
            loop {
                let bytes_read = stream.read(&mut buffer).map_err(|e| {
                    eprintln!("{e}");
                    Response::new_bad_request()
                })?;

                if bytes_read == 0 {
                    break; // Connection closed
                }

                vec.extend(&buffer[..bytes_read]);

                if vec.windows(4).any(|window| window == b"\r\n\r\n") {
                    // reached the end of headers.
                    // todo: Once bodies become a thing in the codecrafters challenge, then this will cause an issue
                    break;
                }
            }

            vec
        };
        Request::try_from(bytes.as_slice())
    }

    pub fn make_response(&self) -> Response {
        match self.method {
            Method::Get => handle_get(self),
            Method::Post => todo!(),
            Method::Put => todo!(),
        }
    }
}

fn handle_get(request: &Request) -> Response {
    debug_assert_eq!(request.method, Method::Get);

    match request.target.as_ref() {
        b"/" => Response::default(),
        _ => Response {
            status: ResponseStatus::NotFound,
            ..Default::default()
        },
    }
}

impl<'a> TryFrom<&'a [u8]> for Request {
    type Error = Response;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let mut sbb = split_by_bytes(value, *b"\r\n");
        let request_line = match sbb.next() {
            Some(Ok(bytes)) => bytes,
            Some(Err(e)) => {
                eprintln!("{e}");
                return Err(Response::new_server_error());
            }
            None => return Err(Response::new_bad_request()),
        };

        let mut request_line_split = request_line.split(|b| *b == b' ');

        let method: Method = request_line_split
            .next()
            .ok_or(Response::new_bad_request())? // no method given
            .try_into()
            .map_err(|_| Response::new_bad_request())?;

        let target: Box<[u8]> = request_line_split
            .next()
            .ok_or(Response::new_bad_request())? // no target given
            .into();

        let http_version: Version = request_line_split
            .next()
            .ok_or(Response::new_bad_request())?
            .try_into()
            .map_err(|_| Response::new_bad_request())?;

        if request_line_split.next().is_some() {
            return Err(Response::new_bad_request());
        }

        let mut headers = Vec::new();
        loop {
            match sbb.next() {
                Some(Ok(bytes)) if bytes.is_empty() => break, // found \r\n\r\n
                Some(Ok(bytes)) => {
                    headers.push(Header::try_from(bytes).map_err(|_| Response::new_bad_request())?);
                }
                Some(Err(io_error)) => {
                    eprintln!("{io_error}");
                    return Err(Response::new_server_error());
                }
                None => return Err(Response::new_bad_request()),
            };
        }
        let headers = headers.into_boxed_slice();
        // let body = sbb.inn
        let body = Box::new([]);

        Ok(Request {
            method,
            target,
            _http_version: http_version,
            _headers: headers,
            _body: body,
        })
    }
}

/// An iterator over the lines of an instance of `BufRead`.
///
/// This struct is generally created by calling [`lines`] on a `BufRead`.
/// Please see the documentation of [`lines`] for more details.
///
/// [`lines`]: BufRead::lines
struct SplitByBytes<B, const N: usize> {
    buf: B,
    delimiter: [u8; N],
}

fn split_by_bytes<B: BufRead, const N: usize>(buf: B, delimiter: [u8; N]) -> SplitByBytes<B, N> {
    SplitByBytes { buf, delimiter }
}

impl<B: BufRead, const N: usize> Iterator for SplitByBytes<B, N> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<io::Result<Vec<u8>>> {
        let mut vec = Vec::new();

        loop {
            match self.buf.read_until(self.delimiter[N - 1], &mut vec) {
                Ok(0) => return None,
                Err(e) => return Some(Err(e)),
                Ok(_) => {}
            }

            if vec.ends_with(&self.delimiter) {
                for _ in 0..N {
                    vec.pop();
                }
                return Some(Ok(vec));
            }
        }
    }
}
