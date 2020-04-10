use serde::Deserialize;
use tide::{server::Service, IntoResponse, Request, Response, Server, Endpoint};
use hyper::{Body, body};
use std::sync::Arc;
use bytes::Buf;

#[derive(Deserialize)]
struct Params {
    msg: String,
}

#[derive(Deserialize)]
struct OptionalParams {
    _msg: Option<String>,
    _time: Option<u64>,
}

async fn handler(cx: Request<()>) -> Response {
    let p = cx.query::<Params>();
    match p {
        Ok(params) => params.msg.into_response(),
        Err(error) => error.into_response(),
    }
}

async fn optional_handler(cx: Request<()>) -> Response {
    let p = cx.query::<OptionalParams>();
    match p {
        Ok(_) => Response::new(200),
        Err(error) => error.into_response(),
    }
}

fn get_server() -> Service<()> {
    let mut app = Server::new();
    app.at("/").get(handler);
    app.at("/optional").get(optional_handler);
    app.into_http_service()
}

#[tokio::test]
async fn successfully_deserialize_query() {
    let server = get_server();
    let req = hyper::Request::get("/?msg=Hello")
        .body(Body::empty())
        .unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = server.call(req).await;
    assert_eq!(res.status(), 200);
    let body = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&body[..], "Hello".as_bytes());
}

#[tokio::test]
async fn unsuccessfully_deserialize_query() {
    let server = get_server();
    let req = hyper::Request::get("/").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = server.call(req).await;
    assert_eq!(res.status(), 400);

    let body = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&body[..], "failed with reason: missing field `msg`".as_bytes());
}

#[tokio::test]
async fn malformatted_query() {
    let server = get_server();
    let req = hyper::Request::get("/?error=should_fail")
        .body(Body::empty())
        .unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let mut res = server.call(req).await;
    assert_eq!(res.status(), 400);

    let body = body::aggregate(res.take_body()).await.unwrap().to_bytes().to_vec();
    assert_eq!(&body[..], "failed with reason: missing field `msg`".as_bytes());
}

#[tokio::test]
async fn empty_query_string_for_struct_with_no_required_fields() {
    let server = get_server();
    let req = hyper::Request::get("/optional").body(Body::empty()).unwrap();
    let req = Request::new(Arc::new(()), req, vec![]);
    let res = server.call(req).await;
    assert_eq!(res.status(), 200);
}
