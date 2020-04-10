use cookie::Cookie;
use hyper::{Body, body};

use tide::{Request, Response, Server, Endpoint};
use std::sync::Arc;
use bytes::Buf;

static COOKIE_NAME: &str = "testCookie";

async fn retrieve_cookie(cx: Request<()>) -> String {
    cx.cookie(COOKIE_NAME).unwrap().value().to_string()
}

async fn set_cookie(_req: Request<()>) -> Response {
    let mut res = Response::new(200);
    res.set_cookie(Cookie::new(COOKIE_NAME, "NewCookieValue"));
    res
}

async fn remove_cookie(_req: Request<()>) -> Response {
    let mut res = Response::new(200);
    res.remove_cookie(Cookie::named(COOKIE_NAME));
    res
}

async fn set_multiple_cookie(_req: Request<()>) -> Response {
    let mut res = Response::new(200);
    res.set_cookie(Cookie::new("C1", "V1"));
    res.set_cookie(Cookie::new("C2", "V2"));
    res
}

fn app() -> crate::Server<()> {
    let mut app = tide::new();

    app.at("/get").get(retrieve_cookie);
    app.at("/set").get(set_cookie);
    app.at("/remove").get(remove_cookie);
    app.at("/multi").get(set_multiple_cookie);
    app
}

async fn make_request(endpoint: &str) -> Response {
    let app = app().into_http_service();
    let req = hyper::Request::get(endpoint)
        .header(hyper::header::COOKIE, "testCookie=RequestCookieValue")
        .body(Body::empty())
        .unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    app.call(req).await
}

#[tokio::test]
async fn successfully_retrieve_request_cookie() {
    let mut res = make_request("/get").await;
    assert_eq!(res.status(), 200);

    let body = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();

    assert_eq!(&body[..], &*b"RequestCookieValue");
}

#[tokio::test]
async fn successfully_set_cookie() {
    let res = make_request("/set").await;
    assert_eq!(res.status(), 200);
    let test_cookie_header = res.headers().get(hyper::header::SET_COOKIE).unwrap();
    assert_eq!(
        test_cookie_header.to_str().unwrap(),
        "testCookie=NewCookieValue"
    );
}

#[tokio::test]
async fn successfully_remove_cookie() {
    let res = make_request("/remove").await;
    assert_eq!(res.status(), 200);
    let test_cookie_header = res.headers().get(hyper::header::SET_COOKIE).unwrap();
    assert!(test_cookie_header
        .to_str()
        .unwrap()
        .starts_with("testCookie=;"));
    let cookie = Cookie::parse_encoded(test_cookie_header.to_str().unwrap()).unwrap();
    assert_eq!(cookie.name(), COOKIE_NAME);
    assert_eq!(cookie.value(), "");
    assert_eq!(cookie.http_only(), None);
    assert_eq!(cookie.max_age().unwrap().whole_nanoseconds(), 0);
}

#[tokio::test]
async fn successfully_set_multiple_cookies() {
    let res = make_request("/multi").await;
    assert_eq!(res.status(), 200);
    let cookie_header = res.headers().get_all(hyper::header::SET_COOKIE);
    let mut iter = cookie_header.iter();

    let cookie1 = iter.next().unwrap();
    let cookie2 = iter.next().unwrap();

    //Headers can be out of order
    if cookie1.to_str().unwrap().starts_with("C1") {
        assert_eq!(cookie1, "C1=V1");
        assert_eq!(cookie2, "C2=V2");
    } else {
        assert_eq!(cookie2, "C1=V1");
        assert_eq!(cookie1, "C2=V2");
    }

    assert!(iter.next().is_none());
}
