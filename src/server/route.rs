use std::sync::Arc;

use crate::endpoint::MiddlewareEndpoint;
use crate::utils::BoxFuture;
use crate::{router::Router, Endpoint, Middleware, Response};
use hyper::{Method, Uri};

/// A handle to a route.
///
/// All HTTP requests are made against resources. After using [`Server::at`] (or
/// [`Route::at`]) to establish a route, the `Route` type can be used to
/// establish endpoints for various HTTP methods at that path. Also, using
/// `nest`, it can be used to set up a subrouter.
///
/// [`Server::at`]: ./struct.Server.html#method.at
#[allow(missing_debug_implementations)]
pub struct Route<'a, State> {
    router: &'a mut Router<State>,
    path: String,
    middleware: Vec<Arc<dyn Middleware<State>>>,
    /// Indicates whether the path of current route is treated as a prefix. Set by
    /// [`strip_prefix`].
    ///
    /// [`strip_prefix`]: #method.strip_prefix
    prefix: bool,
}

impl<'a, State: 'static> Route<'a, State> {
    pub(crate) fn new(router: &'a mut Router<State>, path: String) -> Route<'a, State> {
        Route {
            router,
            path,
            middleware: Vec::new(),
            prefix: false,
        }
    }

    /// Extend the route with the given `path`.
    pub fn at<'b>(&'b mut self, path: &str) -> Route<'b, State> {
        let mut p = self.path.clone();

        if !p.ends_with('/') && !path.starts_with('/') {
            p.push_str("/");
        }

        if path != "/" {
            p.push_str(path);
        }

        Route {
            router: &mut self.router,
            path: p,
            middleware: self.middleware.clone(),
            prefix: false,
        }
    }

    /// Treat the current path as a prefix, and strip prefixes from requests.
    ///
    /// This method is marked unstable as its name might change in the near future.
    ///
    /// Endpoints will be given a path with the prefix removed.
    #[cfg(any(feature = "unstable", feature = "docs"))]
    #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
    pub fn strip_prefix(&mut self) -> &mut Self {
        self.prefix = true;
        self
    }

    /// Apply the given middleware to the current route.
    pub fn middleware(&mut self, middleware: impl Middleware<State>) -> &mut Self {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Reset the middleware chain for the current route, if any.
    pub fn reset_middleware(&mut self) -> &mut Self {
        self.middleware.clear();
        self
    }

    /// Nest a [`Server`] at the current path.
    ///
    /// [`Server`]: struct.Server.html
    pub fn nest<InnerState>(&mut self, service: crate::Server<InnerState>) -> &mut Self
    where
        State: Send + Sync + 'static,
        InnerState: Send + Sync + 'static,
    {
        self.prefix = true;
        self.all(service.into_http_service());
        self.prefix = false;
        self
    }

    /// Add an endpoint for the given HTTP method
    pub fn method(&mut self, method: Method, ep: impl Endpoint<State>) -> &mut Self {
        if self.prefix {
            let ep = StripPrefixEndpoint::new(ep);
            let (ep1, ep2): (Box<dyn Endpoint<_>>, Box<dyn Endpoint<_>>) =
                if self.middleware.is_empty() {
                    let ep = Box::new(ep);
                    (ep.clone(), ep)
                } else {
                    let ep = Box::new(MiddlewareEndpoint::wrap_with_middleware(
                        ep,
                        &self.middleware,
                    ));
                    (ep.clone(), ep)
                };
            self.router.add(&self.path, method.clone(), ep1);
            let wildcard = self.at("*--tide-path-rest");
            wildcard.router.add(&wildcard.path, method, ep2);
        } else {
            let ep: Box<dyn Endpoint<_>> = if self.middleware.is_empty() {
                Box::new(ep)
            } else {
                Box::new(MiddlewareEndpoint::wrap_with_middleware(
                    ep,
                    &self.middleware,
                ))
            };
            self.router.add(&self.path, method, ep);
        }
        self
    }

    /// Add an endpoint for all HTTP methods, as a fallback.
    ///
    /// Routes with specific HTTP methods will be tried first.
    pub fn all(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        if self.prefix {
            let ep = StripPrefixEndpoint::new(ep);
            let (ep1, ep2): (Box<dyn Endpoint<_>>, Box<dyn Endpoint<_>>) =
                if self.middleware.is_empty() {
                    let ep = Box::new(ep);
                    (ep.clone(), ep)
                } else {
                    let ep = Box::new(MiddlewareEndpoint::wrap_with_middleware(
                        ep,
                        &self.middleware,
                    ));
                    (ep.clone(), ep)
                };
            self.router.add_all(&self.path, ep1);
            let wildcard = self.at("*--tide-path-rest");
            wildcard.router.add_all(&wildcard.path, ep2);
        } else {
            let ep: Box<dyn Endpoint<_>> = if self.middleware.is_empty() {
                Box::new(ep)
            } else {
                Box::new(MiddlewareEndpoint::wrap_with_middleware(
                    ep,
                    &self.middleware,
                ))
            };
            self.router.add_all(&self.path, ep);
        }
        self
    }

    /// Add an endpoint for `GET` requests
    pub fn get(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::GET, ep);
        self
    }

    /// Add an endpoint for `HEAD` requests
    pub fn head(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::HEAD, ep);
        self
    }

    /// Add an endpoint for `PUT` requests
    pub fn put(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::PUT, ep);
        self
    }

    /// Add an endpoint for `POST` requests
    pub fn post(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::POST, ep);
        self
    }

    /// Add an endpoint for `DELETE` requests
    pub fn delete(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::DELETE, ep);
        self
    }

    /// Add an endpoint for `OPTIONS` requests
    pub fn options(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::OPTIONS, ep);
        self
    }

    /// Add an endpoint for `CONNECT` requests
    pub fn connect(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::CONNECT, ep);
        self
    }

    /// Add an endpoint for `PATCH` requests
    pub fn patch(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::PATCH, ep);
        self
    }

    /// Add an endpoint for `TRACE` requests
    pub fn trace(&mut self, ep: impl Endpoint<State>) -> &mut Self {
        self.method(Method::TRACE, ep);
        self
    }
}

#[derive(Debug)]
struct StripPrefixEndpoint<E>(std::sync::Arc<E>);

impl<E> StripPrefixEndpoint<E> {
    fn new(ep: E) -> Self {
        Self(std::sync::Arc::new(ep))
    }
}

impl<E> Clone for StripPrefixEndpoint<E> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<State, E: Endpoint<State>> Endpoint<State> for StripPrefixEndpoint<E> {
    fn call<'a>(&'a self, mut req: crate::Request<State>) -> BoxFuture<'a, Response> {
        let rest = req.rest().unwrap_or("");
        let mut path_and_query = format!("/{}", rest);
        let uri = req.uri();
        if let Some(query) = uri.query() {
            path_and_query.push('?');
            path_and_query.push_str(query);
        }
        let mut new_uri = Uri::builder();
        if let Some(scheme) = uri.scheme() {
            new_uri = new_uri.scheme(scheme.clone());
        }
        if let Some(authority) = uri.authority() {
            new_uri = new_uri.authority(authority.clone());
        }
        let new_uri = new_uri.path_and_query(path_and_query.as_str()).build().unwrap();
        *req.request.uri_mut() = new_uri;

        self.0.call(req)
    }
}
