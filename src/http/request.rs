use std::{
    io::{self, BufRead, BufReader},
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

impl TryFrom<BufReader<&mut TcpStream>> for Request {
    type Error = Response;

    fn try_from(buf: BufReader<&mut TcpStream>) -> Result<Request, Response> {
        let mut sbb = split_by_bytes(buf, *b"\r\n");
        let request_line = match sbb.next() {
            Some(Ok(bytes)) => bytes,
            Some(Err(e)) => {
                eprintln!("{e}");
                return Err(Response::new_server_error());
            }
            None => return Err(Response::bad_request("missing HTTP request line")),
        };

        let mut request_line_split = request_line.split(|b| *b == b' ');

        let method: Method = request_line_split
            .next()
            .ok_or(Response::bad_request("missing HTTP method"))? // no method given
            .try_into()
            .map_err(Response::bad_request)?;

        let target: Box<[u8]> = request_line_split
            .next()
            .ok_or(Response::bad_request("missing HTTP target URL"))? // no target given
            .into();

        let http_version: Version = request_line_split
            .next()
            .ok_or(Response::bad_request("missing HTTP version"))? // no version given
            .try_into()
            .map_err(Response::bad_request)?;

        if let Some(bad) = request_line_split.next() {
            return Err(Response::bad_request(format!(
                "expected \\r\\n, found {}",
                String::from_utf8_lossy(bad)
            )));
        }

        let mut headers = Vec::new();
        loop {
            match sbb.next() {
                Some(Ok(bytes)) if bytes.is_empty() => break, // found \r\n\r\n
                Some(Ok(bytes)) => {
                    headers.push(Header::try_from(bytes).map_err(Response::bad_request)?);
                }
                Some(Err(io_error)) => {
                    eprintln!("{io_error}");
                    return Err(Response::new_server_error());
                }
                None => return Err(Response::bad_request("")),
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

/// An iterator over an instance of `BufRead` seperated by a given multi-byte delimiter
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
