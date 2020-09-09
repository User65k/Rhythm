use hyper::upgrade::Upgraded;
use hyper::{Body, Request, Response, header};

use hyper_staticfile::Static;

use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        handshake::server::create_response as create_ws_response,
        protocol::Role,
        Message,
        error::Error as WSErr
    }};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use crate::Notifier;

pub async fn render_webui(req: Request<Body>, fileserver: Static, broadcast: Notifier) -> Result<Response<Body>, hyper::Error> {
    match req.uri().path() {
        "/events" => {
            let (parts, body) = req.into_parts();
            match create_ws_response(&Request::from_parts(parts, ())){
                Ok(resp) => {
                    tokio::task::spawn(async move {
                        match body.on_upgrade().await {
                            Ok(upgraded) => {
                                if let Err(e) = websocket(upgraded, broadcast).await {
                                    eprintln!("server io error: {}", e)
                                };
                            }
                            Err(e) => eprintln!("upgrade error: {}", e),
                        }
                    });
                    Ok(resp.map(|_|Body::empty()))
                },
                Err(e) => {
                    let e = format!("{}",e);
                    let mut resp = Response::new(Body::from(e));
                    *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
                    Ok(resp)
                }
            }
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

async fn websocket(upgraded: Upgraded, broadcast: Notifier) -> Result<(), WSErr> {
    println!("WS!");
    let mut ws = WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;
    let a = ws.next().await;
    println!("{:?}",a);

    let mut rx = broadcast.subscribe();

    while let Ok(b) = rx.recv().await {
        ws.send(Message::text(b)).await?;
    }

    Ok(())
}
