use crate::{
    encoding::{self, Encoding},
    http::{
        error::{BadRequest, InvalidTargetError},
        response::{client_error, server_error, success, Response, CRLF},
        Header, Method, Version,
    },
    DIRECTORY,
};
use std::{
    collections::HashMap,
    fmt::{self, Formatter},
    fs::{self, File},
    io::{self, BufRead, BufReader, Read},
    net::TcpStream,
    path::{Path, PathBuf},
};

pub struct Request {
    method: Method,
    target: Target,
    http_version: Version,
    headers: HashMap<Box<str>, Box<str>>,
    body: Box<[u8]>,
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Request {
            method,
            target,
            http_version,
            headers,
            body,
        } = self;
        write!(
            f,
            "Request: {{\n\
            method: {method:?},\n\
            target: {target:?},\n\
            http_version: {http_version:?},\n\
            headers: {headers:?}\n\
            body: {:?},\n\
            }}",
            String::from_utf8_lossy(body)
        )
    }
}

impl Request {
    pub fn handle(self) -> Response {
        log::trace!("received {self:?}");
        let Request {
            target: Target { path_str },
            body,
            method,
            mut headers,
            ..
        } = self;

        let (endpoint, remainder) = path_str.split_once('/').unwrap_or((&path_str, ""));

        let encoding_options_str = headers.remove("Accept-Encoding").unwrap_or("".into());
        let response_encoding = get_first_supported_encoding(&encoding_options_str);
        dbg!(&response_encoding);

        let result = match (method, endpoint) {
            (Method::Get, "") => Ok(Response::default()),
            (Method::Get, "files") => handle_get_file(remainder, response_encoding),
            (Method::Post, "files") => handle_post_file(remainder, body),
            (Method::Get, "echo") => Ok(success::plain_text(
                remainder.to_string(),
                response_encoding,
            )),
            (Method::Get, "user-agent") if remainder.is_empty() => handle_get_user_agent(headers),
            (Method::Get, other) => {
                log::debug!("request to unimplemented endpoint: {other}");
                Err(client_error::not_found())
            }
            (Method::Post, _) => todo!("handle invalid post targets"),
        };
        result.unwrap_or_else(Response::from)
    }
}

fn handle_post_file(file_path: &str, content: Box<[u8]>) -> Result<Response, Response> {
    let path = try_create_path(file_path)?;
    write_creating_parents(path, content)?;

    Ok(success::created())
}

fn handle_get_file(file_path: &str, opt_encoding: Option<Encoding>) -> Result<Response, Response> {
    log::debug!("retreiving file...");
    let path = try_create_path(file_path)?;

    let mut file = File::open(path)?;
    let body = if let Some(encoding) = opt_encoding {
        encoding::read_and_encode(file, encoding)?
    } else {
        let mut vec = Vec::new();
        file.read_to_end(&mut vec)?;
        vec
    };

    Ok(success::octet_stream(body, opt_encoding))
}

fn get_first_supported_encoding(supported_encodings_str: &str) -> Option<Encoding> {
    dbg!(supported_encodings_str);
    supported_encodings_str
        .split_ascii_whitespace()
        .find_map(|str| Encoding::try_from(str).ok())
}

fn handle_get_user_agent(mut headers: HashMap<Box<str>, Box<str>>) -> Result<Response, Response> {
    let user_agent = headers
        .remove("User-Agent")
        .ok_or(BadRequest::MissingHeader("User-Agent"))?;
    Ok(success::plain_text(user_agent.into_string(), None))
}

/// On some OS, creating a file won't work unless it's parents already exist
fn write_creating_parents<P, C>(path: P, contents: C) -> io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn try_create_path(file_name: &str) -> Result<PathBuf, Response> {
    let path = DIRECTORY
        .get()
        .ok_or(server_error::generic())
        .inspect_err(|_| log::error!("DIRECTORY is not set!"))?
        .join(file_name);
    Ok(path)
}
pub trait RequestSource {
    fn read_request(self) -> Result<Request, Response>;
}

impl RequestSource for &mut TcpStream {
    fn read_request(self) -> Result<Request, Response> {
        let mut buf = BufReader::new(self);

        let mut sbb = split_by_bytes(&mut buf, CRLF);
        let request_line = match sbb.next() {
            Some(Ok(bytes)) => bytes,
            Some(Err(e)) => {
                log::error!("{e}");
                return Err(server_error::generic());
            }
            None => return Err(client_error::bad_request::missing_method()),
        };

        let (method, target, http_version) = parse_request_line(request_line)?;

        let mut headers = HashMap::new();
        loop {
            match sbb
                .next()
                .expect("TcpStream can't return None unless stream is closed")
            {
                Ok(bytes) if bytes.is_empty() => break, // found \r\n\r\n (marking end of headers)
                Ok(bytes) => {
                    let Header { key, value } = Header::try_from(bytes)?;
                    headers.insert(key, value);
                }
                Err(io_error) => {
                    log::error!("System error: {io_error}");
                    return Err(server_error::generic());
                }
            }
        }

        let body = match method {
            Method::Get => Box::new([]),
            Method::Post => {
                let count: usize = headers
                    .remove("Content-Length")
                    .ok_or(BadRequest::MissingHeader("Content-Length"))?
                    .parse()
                    .map_err(|_| BadRequest::MalformedHeader)?;
                let mut vec = vec![0; count];
                buf.read_exact(&mut vec)?;
                vec.into_boxed_slice()
            }
        };

        let request = Request {
            method,
            target,
            http_version,
            headers,
            body,
        };
        log::trace!("parsed request: {request:?}");
        Ok(request)
    }
}

fn parse_request_line(request_line: Vec<u8>) -> Result<(Method, Target, Version), BadRequest> {
    let request_line = String::try_from(request_line)?;

    let mut request_line_split = request_line.split(' ');

    // let mut request_line_split = request_line.split(|b| *b == b' ');
    let method: Method = request_line_split
        .next()
        .ok_or(BadRequest::MissingMethod)?
        .try_into()?;

    let target: Target = request_line_split
        .next()
        .ok_or(BadRequest::MissingTarget)?
        .try_into()?;

    let http_version: Version = request_line_split
        .next()
        .ok_or(BadRequest::MissingHTTPVersion)? // no version given
        .try_into()?;

    if request_line_split.next().is_some() {
        // expected \\r\\n
        return Err(BadRequest::MissingCRLF);
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

        let last_of_delimiter = self.delimiter[N - 1];
        loop {
            match self.buf.read_until(last_of_delimiter, &mut vec) {
                Ok(0) => return None, // End of reader
                Err(e) => return Some(Err(e)),
                Ok(_) if vec.ends_with(&self.delimiter) => {
                    for _ in 0..N {
                        vec.pop();
                    }
                    return Some(Ok(vec));
                }
                Ok(_) => {} // continue
            }
        }
    }
}

#[derive(Debug)]
pub struct Target {
    path_str: String,
    // query component(s)
}

impl TryFrom<&'_ str> for Target {
    type Error = InvalidTargetError;
    fn try_from(str: &str) -> Result<Target, InvalidTargetError> {
        let (prefix, relevant) = str.split_at(1);

        if prefix != "/" {
            log::trace!("target deemed invalid: does not start with '/': {str:?}");
            return Err(InvalidTargetError::DoesNotStartWithSlash);
        }

        let mut split = relevant.split('?'); // todo check the rules of URLs and consider TryFrom instead. This split might not be enough?
        let path = split.next().unwrap_or("").to_string();

        // todo add query parameters

        Ok(Target { path_str: path })
    }
}
