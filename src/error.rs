//! Tide error types.
use hyper::StatusCode;

use crate::response::{IntoResponse, Response};
use hyper::Body;

/// A specialized Result type for Tide.
pub type Result<T = Response> = std::result::Result<T, Error>;

/// A generic error.
#[derive(Debug, derive_more::From)]
pub enum Error {
    Hyper(hyper::Error),
    Response(Response),
    IO(std::io::Error)
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::Response(r) => r,
            _ => unimplemented!(),
        }
    }
}

struct Cause(Box<dyn std::error::Error + Send + Sync>);

impl From<StatusCode> for Error {
    fn from(status: StatusCode) -> Error {
        Error::Response(Response::new(status.as_u16()))
    }
}

/// A simple error type that wraps a String
#[derive(Debug)]
pub struct StringError(pub String);
impl std::error::Error for StringError {}

impl std::fmt::Display for StringError {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(formatter)
    }
}

/// Extension methods for `Result`.
pub trait ResultExt<T>: Sized {
    /// Convert to an `Result`, treating the `Err` case as a client
    /// error (response code 400).
    fn client_err(self) -> Result<T> {
        self.with_err_status(StatusCode::BAD_REQUEST)
    }

    /// Convert to an `Result`, treating the `Err` case as a server
    /// error (response code 500).
    fn server_err(self) -> Result<T> {
        self.with_err_status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Convert to an `Result`, wrapping the `Err` case with a custom
    /// response status.
    fn with_err_status(self, status: impl Into<StatusCode>) -> Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ResultExt<T> for std::result::Result<T, E> {
    fn with_err_status(self, status: impl Into<StatusCode>) -> Result<T> {
        self.map_err(|e| {
            let res = hyper::Response::builder()
                .status(status.into())
                .extension(Cause(Box::new(e)))
                .body(Body::empty())
                .unwrap()
                .into();
            Error::Response(res)
        })
    }
}
