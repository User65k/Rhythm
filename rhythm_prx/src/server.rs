use futures_util::SinkExt;
use hyper::body::Bytes;
use hyper::{header, Body, Request, Response};

use hyper_staticfile::Static;
use tokio_util::codec::{Decoder, Framed};
use websocket_codec::{ClientRequest, Message, MessageCodec, Opcode};

use crate::{Cfg, Notifier};
pub type AsyncClient = Framed<hyper::upgrade::Upgraded, MessageCodec>;
use rhythm_proto::APICall;
use serde::{de::Error as SerdeErr, Deserialize, Deserializer};
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

pub async fn render_webui(
    req: Request<Body>,
    cfg: Arc<Cfg>,
) -> Result<Response<Body>, hyper::Error> {
    match req.uri().path() {
        "/events" => {
            let mut res = Response::new(Body::empty());

            let ws_accept = if let Ok(req) = ClientRequest::parse(|name| {
                let h = req.headers().get(name)?;
                h.to_str().ok()
            }) {
                req.ws_accept()
            } else {
                *res.status_mut() = hyper::StatusCode::BAD_REQUEST;
                return Ok(res);
            };

            tokio::task::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        let client = MessageCodec::server().framed(upgraded);
                        websocket(client, cfg).await;
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });

            *res.status_mut() = hyper::StatusCode::SWITCHING_PROTOCOLS;

            let headers = res.headers_mut();
            headers.insert(
                header::UPGRADE,
                hyper::header::HeaderValue::from_static("websocket"),
            );
            headers.insert(
                header::CONNECTION,
                hyper::header::HeaderValue::from_static("Upgrade"),
            );
            headers.insert(
                header::SEC_WEBSOCKET_ACCEPT,
                hyper::header::HeaderValue::from_str(&ws_accept).unwrap(),
            );
            Ok(res)
        }
        "/" => {
            let resp = Response::builder()
                .status(hyper::StatusCode::MOVED_PERMANENTLY)
                .header(header::LOCATION, "/index.html")
                .body(Body::empty())
                .unwrap();
            Ok(resp)
        }
        "/favicon.ico" | "/main.js" | "/main.wasm" | "/index.html" | "/style.css" => {
            match cfg.fileserver.clone().serve(req).await {
                Ok(mut r) => {
                    r.headers_mut().insert(
                        header::CACHE_CONTROL,
                        hyper::header::HeaderValue::from_static("no-cache"),
                    );
                    Ok(r)
                }
                Err(e) => {
                    eprintln!("server io error: {}", e);
                    let mut resp = Response::new(Body::empty());
                    *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                    Ok(resp)
                }
            }
        }
        "/api" => api(req, cfg).await.or_else(|(c, s)| {
            let mut resp = Response::new(Body::from(s));
            *resp.status_mut() = c;
            Ok(resp)
        }),
        _ => {
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = hyper::StatusCode::NOT_FOUND;
            Ok(resp)
        }
    }
}

async fn api(
    req: Request<Body>,
    cfg: Arc<Cfg>,
) -> Result<Response<Body>, (hyper::StatusCode, String)> {
    let op = if let Some(q) = req.uri().query() {
        match serde_urlencoded::from_str::<APICall>(q) {
            Ok(op) => op,
            Err(e) => {
                return Err((hyper::StatusCode::BAD_REQUEST, e.to_string()));
            }
        }
    } else {
        return Err((hyper::StatusCode::BAD_REQUEST, "no query".to_string()));
    };
    println!("API call: {:?}", op);

    //200 OK
    let mut resp = Response::new(Body::empty());

    match op {
        APICall::Brief { id } => {
            //ID 	Time 	Method 	Host 	        Path 	Code+Reason 	RTT 	Size 	Tags
            *resp.body_mut() = Body::from("[0,0,\"GET\",\"example.com\",\"???\",200,1,2000,[]]");
        },
        APICall::Headers { id } => {
            match cfg.settings.read().await.db.get_req_resp(id) {
                Ok(None) => return Err((hyper::StatusCode::NOT_FOUND, "ID not found in DB".to_string())),
                Err(e) => return Err((hyper::StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {:?}", e))),
                Ok(Some((req, resp))) => {

                },
            }
        },
        APICall::ReqBody { id } => {
            match cfg.settings.read().await.db.get_req_body(id) {
                Ok(None) => return Err((hyper::StatusCode::NOT_FOUND, "ID not found in DB".to_string())),
                Err(e) => return Err((hyper::StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {:?}", e))),
                Ok(Some(b)) => {
                    
                },
            }
        },
        APICall::RespBody { id } => {
            match cfg.settings.read().await.db.get_resp_body(id) {
                Ok(None) => return Err((hyper::StatusCode::NOT_FOUND, "ID not found in DB".to_string())),
                Err(e) => return Err((hyper::StatusCode::INTERNAL_SERVER_ERROR, format!("DB Error: {:?}", e))),
                Ok(Some(b)) => {

                },
            }
        },
    }

    Ok(resp)
}

async fn websocket(mut stream_mut: AsyncClient, cfg: Arc<Cfg>) {
    println!("WS!");

    let mut rx = cfg.broadcast.subscribe();

    while let Ok(item) = rx.recv().await {
        if stream_mut.send(item).await.is_err() {
            return;
        }
    }
    stream_mut.send(Message::close(None)).await;
}
