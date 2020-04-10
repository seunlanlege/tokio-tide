use crate::{Request, Response};
use hyper::StatusCode;

/// Conversion into a `Response`.
pub trait IntoResponse: Send + Sized {
    /// Convert the value into a `Response`.
    fn into_response(self) -> Response;

    /// Create a new `IntoResponse` value that will respond with the given status code.
    ///
    /// ```
    /// # use tide::IntoResponse;
    /// let resp = "Hello, 404!".with_status(hyper::status::StatusCode::NOT_FOUND).into_response();
    /// assert_eq!(resp.status(), hyper::status::StatusCode::NOT_FOUND);
    /// ```
    fn with_status(self, status: StatusCode) -> WithStatus<Self> {
        WithStatus {
            inner: self,
            status,
        }
    }
}

// impl IntoResponse for () {
//     fn into_response(self) -> Response {
//         hyper::Response::builder()
//             .status(hyper::status::StatusCode::NO_CONTENT)
//             .body(Body::empty())
//             .unwrap()
//     }
// }

// impl IntoResponse for Vec<u8> {
//     fn into_response(self) -> Response {
//         hyper::Response::builder()
//             .status(hyper::status::StatusCode::OK)
//             .header("Content-Type", "application/octet-stream")
//             .body(Body::from(self))
//             .unwrap()
//     }
// }

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::new(200)
            .set_header("Content-Type", "text/plain; charset=utf-8")
            .body_string(self)
    }
}

impl<State: Send + Sync + 'static> IntoResponse for Request<State> {
    fn into_response(mut self) -> Response {
        Response::new(200)
            .body(self.body_raw())
    }
}

impl IntoResponse for &'_ str {
    fn into_response(self) -> Response {
        self.to_string().into_response()
    }
}

// impl IntoResponse for hyper::status::StatusCode {
//     fn into_response(self) -> Response {
//         hyper::Response::builder()
//             .status(self)
//             .body(Body::empty())
//             .unwrap()
//     }
// }

// impl<T: IntoResponse, U: IntoResponse> IntoResponse for Result<T, U> {
//     fn into_response(self) -> Response {
//         match self {
//             Ok(r) => r.into_response(),
//             Err(r) => {
//                 let res = r.into_response();
//                 if res.status().is_success() {
//                     panic!(
//                         "Attempted to yield error response with success code {:?}",
//                         res.status()
//                     )
//                 }
//                 res
//             }
//         }
//     }
// }

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

/// A response type that modifies the status code.
#[derive(Debug)]
pub struct WithStatus<R> {
    inner: R,
    status: hyper::StatusCode,
}

impl<R: IntoResponse> IntoResponse for WithStatus<R> {
    fn into_response(self) -> Response {
        self.inner.into_response().set_status(self.status)
    }
}
