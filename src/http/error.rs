use crate::http::response::{client_error, server_error, Response};
use std::{io, string::FromUtf8Error};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BadRequest {
    #[error("Request line did not have a method")]
    MissingMethod,
    #[error("Unsupported Method")]
    UnsupportedMethod,
    #[error("Invalid utf-8")]
    NotUTF8,
    #[error(transparent)]
    BadTarget(#[from] InvalidTargetError),
    #[error("Missing HTTP version")]
    MissingHTTPVersion,
    #[error("Unsupported HTTP version")]
    UnsupportedHTTPVersion,
    #[error("A CRLF is missing")]
    MissingCRLF,
    #[error("Missing header: {0}")]
    MissingHeader(&'static str),
    #[error("Malformed header. Recquires delimiting ': '")]
    MalformedHeader,
    #[error("missing a target")]
    MissingTarget,
}

#[derive(Error, Debug)]
pub enum InvalidTargetError {
    #[error("Malformed target: does not start with '/'")]
    DoesNotStartWithSlash,
}

impl From<FromUtf8Error> for BadRequest {
    fn from(_: FromUtf8Error) -> Self {
        BadRequest::NotUTF8
    }
}

impl From<io::Error> for Response {
    fn from(io_err: io::Error) -> Response {
        use io::ErrorKind as EK;
        match io_err.kind() {
            EK::NotFound | EK::PermissionDenied | EK::IsADirectory => client_error::not_found(),
            _ => server_error::generic(),
        }
    }
}
