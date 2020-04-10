use cookie::Cookie;
use hyper::{HeaderMap, Method, Uri, Version, Body};
use route_recognizer::Params;
use serde::Deserialize;

use std::{str::FromStr, sync::Arc};

use crate::middleware::cookies::CookieData;
use crate::error::Error;
use bytes::Buf;

/// An HTTP request.
///
/// The `Request` gives endpoints access to basic information about the incoming
/// request, route parameters, and various ways of accessing the request's body.
///
/// Requests also provide *extensions*, a type map primarily used for low-level
/// communication between middleware and endpoints.
#[derive(Debug)]
pub struct Request<State> {
    pub(crate) state: Arc<State>,
    pub(crate) request: hyper::Request<Body>,
    pub(crate) route_params: Vec<Params>,
}

impl<State> Request<State> {
    pub fn new(
        state: Arc<State>,
        request: hyper::Request<Body>,
        route_params: Vec<Params>,
    ) -> Request<State> {
        Request {
            state,
            request,
            route_params,
        }
    }

    /// Access the request's HTTP method.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use futures::executor::block_on;
    /// # fn main() -> Result<(), std::io::Error> { block_on(async {
    /// #
    /// use tide::Request;
    ///
    /// let mut app = tide::new();
    /// app.at("/").get(|req: Request<()>| async move {
    ///     assert_eq!(req.method(), hyper::Method::GET);
    ///     ""
    /// });
    /// app.listen("127.0.0.1:8080").await?;
    /// #
    /// # Ok(()) })}
    /// ```
    pub fn method(&self) -> &Method {
        self.request.method()
    }

    /// Access the request's full URI method.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use futures::executor::block_on;
    /// # fn main() -> Result<(), std::io::Error> { block_on(async {
    /// #
    /// use tide::Request;
    ///
    /// let mut app = tide::new();
    /// app.at("/").get(|req: Request<()>| async move {
    ///     assert_eq!(req.uri(), &"/".parse::<tide::Uri>().unwrap());
    ///     ""
    /// });
    /// app.listen("127.0.0.1:8080").await?;
    /// #
    /// # Ok(()) })}
    /// ```
    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }

    /// Access the request's HTTP version.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use futures::executor::block_on;
    /// # fn main() -> Result<(), std::io::Error> { block_on(async {
    /// #
    /// use tide::Request;
    ///
    /// let mut app = tide::new();
    /// app.at("/").get(|req: Request<()>| async move {
    ///     assert_eq!(req.version(), tide::Version::HTTP_11);
    ///     ""
    /// });
    /// app.listen("127.0.0.1:8080").await?;
    /// #
    /// # Ok(()) })}
    /// ```
    pub fn version(&self) -> Version {
        self.request.version()
    }

    /// Access the request's headers.
    pub fn headers(&self) -> &HeaderMap {
        self.request.headers()
    }

    /// Get an HTTP header.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use futures::executor::block_on;
    /// # fn main() -> Result<(), std::io::Error> { block_on(async {
    /// #
    /// use tide::Request;
    ///
    /// let mut app = tide::new();
    /// app.at("/").get(|req: Request<()>| async move {
    ///     assert_eq!(req.header("X-Forwarded-For"), Some("127.0.0.1"));
    ///     ""
    /// });
    /// app.listen("127.0.0.1:8080").await?;
    /// #
    /// # Ok(()) })}
    /// ```
    pub fn header(&self, key: &'static str) -> Option<&'_ str> {
        self.request.headers().get(key).map(|h| h.to_str().unwrap())
    }

    /// Get a local value.
    pub fn local<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.request.extensions().get()
    }

    /// Set a local value.
    pub fn set_local<T: Send + Sync + 'static>(mut self, val: T) -> Self {
        self.request.extensions_mut().insert(val);
        self
    }

    ///  Access app-global state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Extract and parse a route parameter by name.
    ///
    /// Returns the results of parsing the parameter according to the inferred
    /// output type `T`.
    ///
    /// The name should *not* include the leading `:` or the trailing `*` (if
    /// any).
    ///
    /// # Errors
    ///
    /// Yields an `Err` if the parameter was found but failed to parse as an
    /// instance of type `T`.
    ///
    /// # Panics
    ///
    /// Panic if `key` is not a parameter for the route.
    pub fn param<T: FromStr>(&self, key: &str) -> Result<T, T::Err> {
        self.route_params
            .iter()
            .rev()
            .filter_map(|params| params.find(key))
            .next()
            .unwrap()
            .parse()
    }

    pub(crate) fn rest(&self) -> Option<&str> {
        self.route_params
            .last()
            .and_then(|params| params.find("--tide-path-rest"))
    }

    /// Reads the entire request body into a byte buffer.
    ///
    /// This method can be called after the body has already been read, but will
    /// produce an empty buffer.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use futures::executor::block_on;
    /// # fn main() -> Result<(), std::io::Error> { block_on(async {
    /// #
    /// use tide::Request;
    ///
    /// let mut app = tide::new();
    /// app.at("/").get(|mut req: Request<()>| async move {
    ///     let _body: Vec<u8> = req.body_bytes().await.unwrap();
    ///     ""
    /// });
    /// app.listen("127.0.0.1:8080").await?;
    /// #
    /// # Ok(()) })}
    /// ```
    pub async fn body_bytes(&mut self) -> Result<Vec<u8>, Error> {
        let body = std::mem::replace(self.request.body_mut(), Body::empty());
        // todo: not a fan of these extra allocations, getting a Vec<u8> out of the body shouldn't be this hard.
        let bytes = hyper::body::aggregate(body).await?.to_bytes();
        Ok(bytes.to_vec())
    }

    /// Reads the entire request body into a string.
    ///
    /// This method can be called after the body has already been read, but will
    /// produce an empty buffer.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// If the body cannot be interpreted as valid UTF-8, an `Err` is returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use futures::executor::block_on;
    /// # fn main() -> Result<(), std::io::Error> { block_on(async {
    /// #
    /// use tide::Request;
    ///
    /// let mut app = tide::new();
    /// app.at("/").get(|mut req: Request<()>| async move {
    ///     let _body: String = req.body_string().await.unwrap();
    ///     ""
    /// });
    /// app.listen("127.0.0.1:8080").await?;
    /// #
    /// # Ok(()) })}
    /// ```
    pub async fn body_string(&mut self) -> Result<String, Error> {
        let body_bytes = self.body_bytes().await?;
        Ok(String::from_utf8(body_bytes).map_err(|_| {
            Error::IO(std::io::ErrorKind::InvalidData.into())
        })?)
    }

    pub fn body_raw(&mut self) -> Body {
        std::mem::replace(self.request.body_mut(), Body::empty())
    }

    /// Reads and deserialized the entire request body via json.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// If the body cannot be interpreted as valid json for the target type `T`,
    /// an `Err` is returned.
    pub async fn body_json<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body_bytes = self.body_bytes().await?;
        Ok(serde_json::from_slice(&body_bytes).map_err(|_| {
            Error::IO(std::io::ErrorKind::InvalidData.into())
        })?)
    }

    /// Get the URL querystring.
    pub fn query<'de, T: Deserialize<'de>>(&'de self) -> Result<T, crate::Error> {
        // Default to an empty query string if no query parameter has been specified.
        // This allows successful deserialisation of structs where all fields are optional
        // when none of those fields has actually been passed by the caller.
        let query = self.uri().query().unwrap_or("");
        serde_qs::from_str(query).map_err(|e| {
            // Return the displayable version of the deserialisation error to the caller
            // for easier debugging.
            let response = crate::Response::new(400).body_string(format!("{}", e));
            crate::Error::Response(response)
        })
    }

    /// Parse the request body as a form.
    pub async fn body_form<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body = self.body_bytes().await?;
        let res = serde_qs::from_bytes(&body).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("could not decode form: {}", e),
            )
        })?;
        Ok(res)
    }

    /// returns a `Cookie` by name of the cookie.
    pub fn cookie(&self, name: &str) -> Option<Cookie<'static>> {
        let cookie_data = self
            .local::<CookieData>()
            .expect("should always be set by the cookies middleware");

        let locked_jar = cookie_data.content.read().unwrap();
        locked_jar.get(name).cloned()
    }
}
