use hyper::server::conn::{AddrStream, Http};
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Uri};

use http::uri::Authority;
use tokio::{net::TcpStream, sync::Mutex};

use futures_util::future::try_join;

use crate::{
    ca::CA,
    db::DB,
    uplink::{make_tcp_con, HTTPClient},
    Cfg, Notifier, Settings,
};
use std::io;
use std::sync::{Arc, RwLockReadGuard};

use log::{debug, error, info};
use rhythm_proto::WSNotify;
use std::error::Error;
use websocket_codec::Message;
use tokio_native_tls::{native_tls::TlsAcceptor as NTlsAcc, TlsAcceptor};

async fn http_mitm(
    req: Request<Body>,
    client: HTTPClient,
    broadcast: Notifier,
    db: DB,
) -> Result<Response<Body>, hyper::Error> {
    //let _a = broadcast.send(req.uri().to_string()).is_ok();

    //store request in DB
    let (req_parts, body) = req.into_parts();
    let body = hyper::body::to_bytes(body).await?;
    let req_id = db.store_req(&req_parts, &body);

    if let Ok(id) = &req_id {
        let i = WSNotify::NewReq {
            id: *id,
            method: req_parts.method.to_string(),
            uri: req_parts.uri.to_string(),
        };
        if let Ok(data) = i.as_u8() {
            let _a = broadcast.send(Message::binary(data)).is_ok();
        }
    }

    let mut req = Request::builder()
        .method(req_parts.method.clone())
        .uri(req_parts.uri.clone())
        .version(req_parts.version)
        .body(body.into())
        .unwrap();
    *req.headers_mut() = req_parts.headers.clone();
    //let req = Request::from_parts(req_parts, body.into());
    info!("plain req: {:?}", req.uri());

    //TODO break + alter

    //forward request
    let rep = client.request(req).await.unwrap_or_else(|err| {
        let e = format!("<html><body><h1>Rhythm</h1> {}</body></html>", err);
        let mut resp = Response::new(Body::from(e));
        *resp.status_mut() = hyper::StatusCode::BAD_GATEWAY;

        if let Some(cause) = err.into_cause() {
            if let Some(io_e) = cause.downcast_ref::<io::Error>() {
                if io_e.kind() == io::ErrorKind::TimedOut {
                    *resp.status_mut() = hyper::StatusCode::GATEWAY_TIMEOUT;
                }
            }
            let mut e: &dyn Error = &*cause;
            loop {
                error!("{}", e);
                e = match e.source() {
                    Some(e) => {
                        error!("caused by:");
                        e
                    }
                    None => break,
                }
            }
        }
        /*
        (500, INTERNAL_SERVER_ERROR, "Internal Server Error");
        (511, NETWORK_AUTHENTICATION_REQUIRED, "Network Authentication Required");
        */
        resp
    });

    //store response in DB
    let (parts, body) = rep.into_parts();

    if parts.status == hyper::StatusCode::SWITCHING_PROTOCOLS {
        return upgrade_proxy_req(parts, req_parts, req_id).await;
    }

    let body = hyper::body::to_bytes(body).await?;
    match req_id {
        Ok(req_id) => {
            if let Err(e) = db.store_resp(req_id, &parts, &body) {
                error!("{:?}", e);
            }

            let i = WSNotify::NewResp {
                id: req_id,
                status: parts.status.as_u16(),
            };
            if let Ok(data) = i.as_u8() {
                let _a = broadcast.send(Message::binary(data)).is_ok();
            }
        }
        Err(e) => error!("{:?}", e),
    }

    let rep = Response::from_parts(parts, body.into());
    info!("plain rep: {:?}", rep.status());

    //return response to browser
    Ok(rep)
}

async fn upgrade_proxy_req(
    resp: hyper::http::response::Parts,
    req: hyper::http::request::Parts,
    req_id: Result<u64, crate::db::DBErr>,
) -> Result<Response<Body>, hyper::Error> {
    let mut rep2ret = Response::builder()
        .status(resp.status)
        .version(resp.version)
        .body(Body::empty())
        .unwrap();
    *rep2ret.headers_mut() = resp.headers.clone();

    let rep = Response::from_parts(resp, Body::empty());
    let mut req = Request::from_parts(req, Body::empty());
    //consume server aw
    match hyper::upgrade::on(rep).await {
        Ok(server) => {
            tokio::task::spawn(async move {
                match hyper::upgrade::on(&mut req).await {
                    Ok(client) => {
                        info!("client upgrade");

                        let _amounts = {
                            let (mut server_rd, mut server_wr) = tokio::io::split(server);
                            let (mut client_rd, mut client_wr) = tokio::io::split(client);

                            let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
                            let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);

                            try_join(client_to_server, server_to_client).await
                        };
                    }
                    Err(e) => error!("Error client upgrade {}", e),
                    //Error client upgrade upgrade expected but low level API in use
                }
            });
            println!("plain rep 2upgr: {:?}", rep2ret.status());

            Ok(rep2ret)
        }
        Err(err) => {
            error!("Error server upgrade {}", err);
            let e = format!("<html><body><h1>Rhythm</h1>Upgrade {}</body></html>", err);
            let mut resp = Response::new(Body::from(e));
            *resp.status_mut() = hyper::StatusCode::BAD_GATEWAY;
            Ok(resp)
        }
    }
}

async fn tls_mitm(
    tcp_stream: TcpStream,
    auth: &Authority,
    broadcast: Notifier,
    client: HTTPClient,
    settings: &Settings,
) -> Result<(), Box<dyn Error>> {
    /*
    read first 6 byte and check if it is TLS to
    support other things than HTTPS here
    like HTTP (for Websockets)*/

    let mut b1 = [0; 16];
    let n = tcp_stream.peek(&mut b1).await?;
    /*
    16  //handshake
    3   //v >= SSL 3.0
    ?   // exact version 1 == TLS1.0+
    ?   // len
    ?   //len
    1 //client hello
    */
    if n > 1 && b1[0] == 0x16 && b1[1] == 0x3 {
        //TLS1.0, TLS1.1, TLS1.2, TLS1.3

        let cert = settings.ca.get_cert_for(auth.host()).await?;
        let tls_acceptor =
            TlsAcceptor::from(NTlsAcc::builder(cert).build()?);
        let tls_stream = tls_acceptor.accept(tcp_stream).await?;

        //TODO check for protocoll (everything but TLS) again
        let auth_str = auth.as_str();

        let some_service = service_fn(move |mut req| {
            let url = if let Some(pq) = req.uri().path_and_query() {
                Uri::builder()
                    .scheme(http::uri::Scheme::HTTPS)
                    .authority(auth_str)
                    .path_and_query(pq.as_str())
            } else {
                Uri::builder()
                    .scheme(http::uri::Scheme::HTTPS)
                    .authority(auth_str)
            };
            *req.uri_mut() = url.build().unwrap();
            let client = client.clone();
            http_mitm(req, client, broadcast.clone(), settings.db.clone())
        });
        let http = Http::new();
        let conn = http.serve_connection(tls_stream, some_service);

        return Ok(conn.with_upgrades().await?);
    }
    let mut try_http = true; //OPTIONS / HTTP/1.1  ~9 ALPHA, URL, HTTP/N.N
    for (x, b) in b1.iter().enumerate() {
        if *b == b' ' || x >= n {
            break;
        }
        if !(b'A'..=b'Z').contains(b) {
            try_http = false;
            break;
        }
    }
    if try_http {
        //HTTP1.x
        let auth_str = auth.as_str();

        let some_service = service_fn(move |mut req| {
            let url = Uri::builder()
                .scheme(http::uri::Scheme::HTTP)
                .authority(auth_str);
            let url = if let Some(pq) = req.uri().path_and_query() {
                url.path_and_query(pq.as_str())
            } else {
                url
            };
            *req.uri_mut() = url.build().unwrap();
            let client = client.clone();
            http_mitm(req, client, broadcast.clone(), settings.db.clone())
        });
        let http = Http::new();
        let conn = http.serve_connection(tcp_stream, some_service);

        return Ok(conn.with_upgrades().await?);
    }
    //HTTP2 should only be used in TLS, so dont care about it here

    error!("don't know protocoll for {:?}, starts with {:?}", auth, b1);

    Ok(pass_throught(tcp_stream, auth).await?)
}

// Create a TCP connection to host:port, build a pass_throught between the connection and
// the upgraded connection
async fn pass_throught(tcp_stream: TcpStream, auth: &Authority) -> std::io::Result<()> {
    // Connect to remote server
    let uri = Uri::builder().authority(auth.clone()).build().unwrap();
    let mut server = make_tcp_con(uri).await?;

    // Proxying data
    let _amounts = {
        let (mut server_rd, mut server_wr) = server.split();
        let (mut client_rd, mut client_wr) = tokio::io::split(tcp_stream);

        let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
        let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);

        try_join(client_to_server, server_to_client).await
    };
    Ok(())
}
/*
///transparent proxy with pass_throught PoC
use tokio::net::TcpListener;
pub async fn transparent_prxy() -> std::io::Result<()> {
    let mut listener = TcpListener::bind("127.0.0.1:443").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        // Proxying data

        pass_throught(socket, &Authority::from_static("192.168.2.114:3389")).await?;
    }
    Ok(())
}*/

pub async fn process_http_req(
    req: Request<Body>,
    cfg: Arc<Cfg>,
) -> Result<Response<Body>, hyper::Error> {
    let settings = cfg.settings.read().await;
    http_mitm(
        req,
        cfg.client.clone(),
        cfg.broadcast.clone(),
        settings.db.clone(),
    )
    .await
}
pub async fn process_connect_req(
    mut req: Request<Body>,
    cfg: Arc<Cfg>,
) -> Result<Response<Body>, hyper::Error> {
    match req.uri().authority() {
        None => {
            error!(
                "CONNECT must contain an endpoint to connect to: {:?}",
                req.uri()
            );
            let mut resp =
                Response::new(Body::from("CONNECT must contain an endpoint to connect to"));
            *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
            Ok(resp)
        }
        Some(a) => {
            debug!("Connection request for {}", a.as_str());
            let auth = a.clone();

            tokio::task::spawn(async move {
                match hyper::upgrade::on(&mut req).await {
                    Ok(upgraded) => {
                        let settings = cfg.settings.read().await;
                        let parts = upgraded
                            .downcast::<AddrStream>()
                            .expect("upgrade not AddrStream");
                        //ignore parts.read_buf - its empty in case of HTTP CONNECT
                        let tcp_stream = parts.io.into_inner();

                        if settings.dont_intercept.is_match(auth.as_str()) {
                            if let Err(e) = pass_throught(tcp_stream, &auth).await {
                                error!("server io error for {}: {}", &auth, &e);
                                let _ = cfg
                                    .broadcast
                                    .send(Message::text(format!("{}: {}", &auth, &e)))
                                    .is_ok();
                            };
                        } else {
                            // intercept!
                            if let Err(e) = tls_mitm(
                                tcp_stream,
                                &auth,
                                cfg.broadcast.clone(),
                                cfg.client.clone(),
                                &settings,
                            )
                            .await
                            {
                                error!("server error for {}:", &auth);
                                let _ = cfg
                                    .broadcast
                                    .send(Message::text(format!("{}: {}", &auth, &e)))
                                    .is_ok();
                                let mut e = &*e;
                                loop {
                                    error!("\t{}", e);
                                    e = match e.source() {
                                        Some(e) => e,
                                        None => break,
                                    }
                                }
                            };
                        }
                    }
                    Err(e) => error!("upgrade error: {}", e), //TODO forward the error to the UI
                }
            });
            Ok(Response::new(Body::empty()))
        }
    }
}
