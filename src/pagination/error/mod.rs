use std::{error, fmt, io, str::Utf8Error};

use mongodb::error::Error;


#[derive(Debug)]
pub enum CursorError {
    IoError(io::Error),
    InvalidCursor(String),
    Unknown(String),
    InvalidId(String),
    MongoError(Error),
}

impl From<io::Error> for CursorError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<Utf8Error> for CursorError {
    fn from(err: Utf8Error) -> Self {
        Self::InvalidCursor(err.to_string())
    }
}

impl From<base64::DecodeError> for CursorError {
    fn from(err: base64::DecodeError) -> Self {
        Self::InvalidCursor(err.to_string())
    }
}

impl fmt::Display for CursorError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::IoError(ref inner) => inner.fmt(fmt),
            Self::InvalidCursor(ref cursor) => {
                write!(fmt, "Invalid cursor - unable to parse: {cursor:?}")
            }
            Self::Unknown(ref inner) => inner.fmt(fmt),
            Self::InvalidId(ref id) => write!(fmt, "Invalid id - {id:?}"),
            Self::MongoError(ref inner ) => write!(fmt, "Mongo Error - {inner:?} "),
        }
    }
}


impl error::Error for CursorError {
    fn description(&self) -> &str {
        match *self {
            Self::IoError(_) => "IO Error",
            Self::InvalidCursor(_) => "Invalid cursor value",
            Self::Unknown(ref inner) => inner,
            Self::InvalidId(_) => "Invalid mongodbid",
            Self::MongoError(_) => "Momngo Error",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Self::IoError(ref inner) => Some(inner),
            _ => None,
        }
    }
}
