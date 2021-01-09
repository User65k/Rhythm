use futures_util::SinkExt;
use hyper::{Body, Request, Response, header};

use hyper_staticfile::Static;
use tokio_util::codec::{Decoder, Framed};
use websocket_codec::{ClientRequest, MessageCodec, Message, Opcode};

use crate::Notifier;
pub type AsyncClient = Framed<hyper::upgrade::Upgraded, MessageCodec>;


pub async fn render_webui(req: Request<Body>, fileserver: Static, broadcast: Notifier) -> Result<Response<Body>, hyper::Error> {
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
                        websocket(client, broadcast).await;
                    }
                    Err(e) => eprintln!("upgrade error: {}", e),
                }
            });
        
            *res.status_mut() = hyper::StatusCode::SWITCHING_PROTOCOLS;
        
            let headers = res.headers_mut();
            headers.insert(header::UPGRADE, hyper::header::HeaderValue::from_static("websocket"));
            headers.insert(header::CONNECTION, hyper::header::HeaderValue::from_static("Upgrade"));
            headers.insert(header::SEC_WEBSOCKET_ACCEPT, hyper::header::HeaderValue::from_str(&ws_accept).unwrap());
            Ok(res)
        },
        "/" => {
            let resp = Response::builder()
            .status(hyper::StatusCode::MOVED_PERMANENTLY)
            .header(header::LOCATION, "/index.html")
            .body(Body::empty())
            .unwrap();
            Ok(resp)
        },
        "/favicon.ico" | "/main.js" | "/main.wasm" | "/index.html" | "/style.css" => {
            match fileserver.serve(req).await {
                Ok(r) => Ok(r),
                Err(e) => {
                    eprintln!("server io error: {}", e);
                    let mut resp = Response::new(Body::empty());
                    *resp.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                    Ok(resp) 
                }
            }
        },
        _ => {
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = hyper::StatusCode::FORBIDDEN;
            Ok(resp)    
        }
    }
}

async fn websocket(mut stream_mut: AsyncClient, broadcast: Notifier) {
    println!("WS!");

    let mut rx = broadcast.subscribe();

    while let Ok(b) = rx.recv().await {
        if stream_mut.send(Message::text(b)).await.is_err() {
            return;
        }
    }
    stream_mut.send(Message::close(None)).await;

}
