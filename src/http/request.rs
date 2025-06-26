use crate::http::{response::Response, Header, Method, Version};
use crate::DIRECTORY;
use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader},
    net::TcpStream,
};

#[derive(Debug)]
pub struct Request {
    method: Method,
    target: Box<[u8]>,
    _http_version: Version,
    headers: HashMap<Box<str>, Box<str>>,
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

    log::trace!("received request {request:?}");

    let mut delimited = request.target.splitn(3, |c| *c == b'/');

    // first in iterator should be "" because target should start with '/'
    match delimited.next() {
        Some(b"") => {} // proceed
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
        b"" => Response::default(),
        b"echo" => Response::plain_text(remainder),
        b"user-agent" if remainder.is_empty() => {
            let Some(user_agent) = request.headers.get("User-Agent") else {
                return Response::bad_request("no user agent");
            };
            Response::plain_text(user_agent.as_bytes())
        }
        b"files" => {
            let Ok(file_name) = String::try_from(remainder.to_vec()) else {
                return Response::not_found(); // could use bad_request instead
            };

            let path = {
                let Some(d) = DIRECTORY.get() else {
                    return Response::new_server_error();
                };
                d.join(file_name)
            };

            match std::fs::read(path) {
                Ok(content) => Response::octet_stream(content),
                Err(io_err) => Response::from(io_err),
            }
        }
        _other => Response::not_found(),
    }
}

impl TryFrom<BufReader<&mut TcpStream>> for Request {
    type Error = Response;

    fn try_from(buf: BufReader<&mut TcpStream>) -> Result<Request, Response> {
        let mut sbb = split_by_bytes(buf, *b"\r\n");
        let request_line = match sbb.next() {
            Some(Ok(bytes)) => bytes,
            Some(Err(e)) => {
                log::error!("{e}");
                return Err(Response::new_server_error());
            }
            None => return Err(Response::bad_request("missing HTTP request line")),
        };

        let (method, target, http_version) = parse_request_line(&request_line)?;

        let mut headers = HashMap::new();
        loop {
            match sbb
                .next()
                .expect("TcpStream can't return None unless stream is closed")
            {
                Ok(bytes) if bytes.is_empty() => break, // found \r\n\r\n (marking end of headers)
                Ok(bytes) => {
                    let Header { key, value } =
                        Header::try_from(bytes).map_err(Response::bad_request)?;
                    headers.insert(key, value);
                }
                Err(io_error) => {
                    eprintln!("{io_error}");
                    return Err(Response::new_server_error());
                }
            }
        }

        let body = Box::new([]);

        Ok(Request {
            method,
            target,
            _http_version: http_version,
            headers,
            _body: body,
        })
    }
}

fn parse_request_line(request_line: &[u8]) -> Result<(Method, Box<[u8]>, Version), Response> {
    let mut request_line_split = request_line.split(|b| *b == b' ');
    let method: Method = request_line_split
        .next()
        .ok_or(Response::bad_request("missing HTTP method"))? // no method given
        .try_into()
        .map_err(|()| Response::bad_request("unrecognized method"))?;
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
