use futures::future::BoxFuture;
use tide::{Middleware, Request, Endpoint};
use std::sync::Arc;
use hyper::Body;

struct TestMiddleware(&'static str, &'static str);

impl TestMiddleware {
    fn with_header_name(name: &'static str, value: &'static str) -> Self {
        Self(name, value)
    }
}

impl<State: Send + Sync + 'static> Middleware<State> for TestMiddleware {
    fn handle<'a>(
        &'a self,
        req: tide::Request<State>,
        next: tide::Next<'a, State>,
    ) -> BoxFuture<'a, tide::Response> {
        Box::pin(async move {
            let res = next.run(req).await;
            res.set_header(self.0, self.1)
        })
    }
}

async fn echo_path<State>(req: tide::Request<State>) -> String {
    req.uri().path().to_string()
}

#[tokio::test]
async fn route_middleware() {
    let mut app = tide::new();
    let mut foo_route = app.at("/foo");
    foo_route // /foo
        .middleware(TestMiddleware::with_header_name("X-Foo", "foo"))
        .get(echo_path);
    foo_route
        .at("/bar") // nested, /foo/bar
        .middleware(TestMiddleware::with_header_name("X-Bar", "bar"))
        .get(echo_path);
    foo_route // /foo
        .post(echo_path)
        .reset_middleware()
        .put(echo_path);
    let app = app.into_http_service();

    let req = hyper::Request::get("/foo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Foo"), Some(&"foo".parse().unwrap()));

    let req = hyper::Request::post("/foo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Foo"), Some(&"foo".parse().unwrap()));

    let req = hyper::Request::put("/foo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Foo"), None);

    let req = hyper::Request::get("/foo/bar").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Foo"), Some(&"foo".parse().unwrap()));
    assert_eq!(res.headers().get("X-Bar"), Some(&"bar".parse().unwrap()));
}

#[tokio::test]
async fn app_and_route_middleware() {
    let mut app = tide::new();
    app.middleware(TestMiddleware::with_header_name("X-Root", "root"));
    app.at("/foo")
        .middleware(TestMiddleware::with_header_name("X-Foo", "foo"))
        .get(echo_path);
    app.at("/bar")
        .middleware(TestMiddleware::with_header_name("X-Bar", "bar"))
        .get(echo_path);
    let app = app.into_http_service();

    let req = hyper::Request::get("/foo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Root"), Some(&"root".parse().unwrap()));
    assert_eq!(res.headers().get("X-Foo"), Some(&"foo".parse().unwrap()));
    assert_eq!(res.headers().get("X-Bar"), None);

    let req = hyper::Request::get("/bar").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Root"), Some(&"root".parse().unwrap()));
    assert_eq!(res.headers().get("X-Foo"), None);
    assert_eq!(res.headers().get("X-Bar"), Some(&"bar".parse().unwrap()));
}

#[tokio::test]
async fn nested_app_with_route_middleware() {
    let mut inner = tide::new();
    inner.middleware(TestMiddleware::with_header_name("X-Inner", "inner"));
    inner
        .at("/baz")
        .middleware(TestMiddleware::with_header_name("X-Baz", "baz"))
        .get(echo_path);

    let mut app = tide::new();
    app.middleware(TestMiddleware::with_header_name("X-Root", "root"));
    app.at("/foo")
        .middleware(TestMiddleware::with_header_name("X-Foo", "foo"))
        .get(echo_path);
    app.at("/bar")
        .middleware(TestMiddleware::with_header_name("X-Bar", "bar"))
        .nest(inner);
    let app = app.into_http_service();

    let req = hyper::Request::get("/foo").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Root"), Some(&"root".parse().unwrap()));
    assert_eq!(res.headers().get("X-Inner"), None);
    assert_eq!(res.headers().get("X-Foo"), Some(&"foo".parse().unwrap()));
    assert_eq!(res.headers().get("X-Bar"), None);
    assert_eq!(res.headers().get("X-Baz"), None);

    let req = hyper::Request::get("/bar/baz").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Root"), Some(&"root".parse().unwrap()));
    assert_eq!(
        res.headers().get("X-Inner"),
        Some(&"inner".parse().unwrap())
    );
    assert_eq!(res.headers().get("X-Foo"), None);
    assert_eq!(res.headers().get("X-Bar"), Some(&"bar".parse().unwrap()));
    assert_eq!(res.headers().get("X-Baz"), Some(&"baz".parse().unwrap()));
}

#[tokio::test]
async fn subroute_not_nested() {
    let mut app = tide::new();
    app.at("/parent") // /parent
        .middleware(TestMiddleware::with_header_name("X-Parent", "Parent"))
        .get(echo_path);
    app.at("/parent/child") // /parent/child, not nested
        .middleware(TestMiddleware::with_header_name("X-Child", "child"))
        .get(echo_path);
    let app = app.into_http_service();

    let req = hyper::Request::get("/parent/child").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = app.call(req).await;
    assert_eq!(res.headers().get("X-Parent"), None);
    assert_eq!(
        res.headers().get("X-Child"),
        Some(&"child".parse().unwrap())
    );
}
