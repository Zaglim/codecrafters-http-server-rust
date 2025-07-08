use flate2::Compression;
use std::io::{self, Read};

/// Read uncompressed data and returns compressed copy of the data in the given encoding.
///
/// # Errors
/// Errors on any `io::Error` except `ErrorKind::Interruped`. Interruptions are ignored and the read will continue
pub fn read_and_encode(readable: impl Read, encoding: Encoding) -> io::Result<Vec<u8>> {
    let mut encoder = match encoding {
        Encoding::Gzip => flate2::read::GzEncoder::new(readable, Compression::default()),
    };

    let compressed = {
        let mut vec = Vec::new();
        encoder.read_to_end(&mut vec)?;
        vec
    };
    Ok(compressed)
}

#[derive(Debug, Copy, Clone)]
pub enum Encoding {
    Gzip,
}

pub struct UnsupportedEncodingError;

impl TryFrom<&str> for Encoding {
    type Error = UnsupportedEncodingError;
    fn try_from(str: &str) -> Result<Self, UnsupportedEncodingError> {
        match str {
            "gzip" => Ok(Encoding::Gzip),
            unsupported => {
                log::trace!("unsupported encoding: {unsupported}");
                Err(UnsupportedEncodingError)
            }
        }
    }
}

impl From<Encoding> for Box<str> {
    fn from(value: Encoding) -> Self {
        <&str>::from(value).into()
    }
}

impl From<Encoding> for &'static str {
    fn from(value: Encoding) -> Self {
        match value {
            Encoding::Gzip => "gzip",
        }
    }
}
