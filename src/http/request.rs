use std::{
    io::{self, BufRead, BufReader},
    net::TcpStream,
};

use crate::http::*;

#[derive(Debug)]
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

    dbg!(&request);

    let mut delimited = request.target.splitn(3, |c| *c == b'/');

    // first in iterator should be "" because target should start with '/'
    match delimited.next() {
        Some(b"") => {}
        Some(not_empty) => {
            return Response::bad_request(format!(
                "malformed taget: {}",
                String::from_utf8_lossy(not_empty)
            ));
        }
        None => return Response::bad_request("empty target value"),
    }

    let endpoint = delimited.next().unwrap_or(b"");
    let remainder = delimited.next().unwrap_or(b"");
    match endpoint {
        b"" => return Response::default(),
        b"echo" | b"user-agent" => return Response::echo(remainder),
        _other => {
            return Response {
                status: ResponseStatus::NotFound,
                ..Default::default()
            }
        }
    }
}

impl TryFrom<BufReader<&mut TcpStream>> for Request {
    type Error = Response;

    fn try_from(buf: BufReader<&mut TcpStream>) -> Result<Request, Response> {
        let mut sbb = split_by_bytes(buf, *b"\r\n");
        let request_line = match sbb.next() {
            Some(Ok(bytes)) => bytes,
            Some(Err(e)) => {
                error!("{e}");
                return Err(Response::new_server_error());
            }
            None => return Err(Response::bad_request("missing HTTP request line")),
        };

        let (method, target, http_version) = parse_request_line(request_line)?;

        let mut headers = Vec::new();
        loop {
            match sbb
                .next()
                .expect("TcpStream can't return None unless stream is closed")
            {
                Ok(bytes) if bytes.is_empty() => break, // found \r\n\r\n (marking end of headers)
                Ok(bytes) => {
                    headers.push(Header::try_from(bytes).map_err(Response::bad_request)?);
                }
                Err(io_error) => {
                    eprintln!("{io_error}");
                    return Err(Response::new_server_error());
                }
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

fn parse_request_line(request_line: Vec<u8>) -> Result<(Method, Box<[u8]>, Version), Response> {
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
    Ok((method, target, http_version))
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
