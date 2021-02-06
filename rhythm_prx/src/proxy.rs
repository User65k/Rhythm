use hyper::service::service_fn;
use hyper::{Body, Request, Response, Uri};
use hyper::server::conn::{Http, AddrStream};

use http::uri::Authority;
use tokio::{net::TcpStream, sync::Mutex};

use futures_util::future::try_join;

use crate::{Notifier, db::DB, ca::CA};
use regex::RegexSet;
use std::sync::Arc;
use std::io;
use crate::uplink::{HTTPClient, make_tcp_con};

use std::error::Error;

async fn http_mitm(req: Request<Body>, client: HTTPClient, broadcast: Notifier, db: Arc<Mutex<DB>>) -> Result<Response<Body>, hyper::Error>
{
    let _a = broadcast.send(req.uri().to_string()).is_ok();
    
    //store request in DB
    let (parts, body) = req.into_parts();
    let body = hyper::body::to_bytes(body).await?;
    let req_id = db.lock().await.store_req(&parts, &body);
    let req = Request::from_parts(parts, body.into());
    println!("plain req: {:?}", req);

    //TODO break + alter

    //forward request
    let rep = client.request(req).await.unwrap_or_else(|err| {
        let e = format!("<html><body><h1>Rhythm</h1> {}</body></html>",err);
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
                eprintln!("{}", e);
                e = match e.source() {
                    Some(e) => {eprintln!("caused by:");e},
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
    let body = hyper::body::to_bytes(body).await?;
    match req_id {
        Ok(req_id) => {
            if let Err(e) = db.lock().await.store_resp(req_id, &parts, &body) {
                eprint!("{:?}", e);
            }
        },
        Err(e) => eprint!("{:?}", e)
    }
    
    let rep = Response::from_parts(parts, body.into());
    println!("plain rep: {:?}", rep);

    //return response to browser
    Ok(rep)
}

async fn tls_mitm(mut ca: CA, tcp_stream: TcpStream, auth: &Authority, broadcast: Notifier, client: HTTPClient, db: Arc<Mutex<DB>>) -> Result<(), Box<dyn Error>> {
    /*
    read first 6 byte and check if it is TLS to
    support other things than HTTPS here
    like HTTP

    let mut b1 = [0; 6];
    let n = tcp_stream.peek(&mut b1).await.expect("what");
    /*
    16  //handshake
    3   //v >= SSL 3.0
    ?   // exact version
    ?   // len
    ?   //len
    1 //hello
    */
    println!("io2: {:?}", b1);
                
    */
    let cert = ca.get_cert_for(auth.host()).await?;
    let tls_acceptor =
        tokio_native_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build()?);
    let tls_stream = tls_acceptor.accept(tcp_stream).await?;

    let auth_str = auth.as_str();

    let some_service = service_fn(move |mut req|{
        let url = if let Some(pq) = req.uri().path_and_query() {
            Uri::builder()
            .scheme(http::uri::Scheme::HTTPS)
            .authority(auth_str)
            .path_and_query(pq.as_str())
        }else{
            Uri::builder()
            .scheme(http::uri::Scheme::HTTPS)
            .authority(auth_str)
        };
        *req.uri_mut() = url.build().unwrap();
        let client = client.clone();
        http_mitm(req, client, broadcast.clone(), db.clone())
    });
    let http = Http::new();
    let conn = http.serve_connection(tls_stream, some_service);
    
    Ok(conn.await?)
}

// Create a TCP connection to host:port, build a pass_throught between the connection and
// the upgraded connection
async fn pass_throught(tcp_stream: TcpStream, auth: &Authority) -> std::io::Result<()> {
    // Connect to remote server
    let uri = Uri::builder()
        .authority(auth.clone())
        .build()
        .unwrap();
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
    let mut listener = TcpListener::bind("127.0.0.1:3389").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        // Proxying data
        pass_throught(socket, &Authority::from_static("192.168.2.114:3389")).await?;
    }
    Ok(())
}// */

pub async fn process_http_req(req: Request<Body>, broadcast: Notifier, client: HTTPClient, db: Arc<Mutex<DB>>) -> Result<Response<Body>, hyper::Error>
{
    http_mitm(req, client, broadcast, db).await
}
pub async fn process_connect_req(
    ca: CA,
    mut req: Request<Body>,
    dont_intercept: Arc<RegexSet>,
    broadcast: Notifier,
    client: HTTPClient,
    db: Arc<Mutex<DB>>) -> Result<Response<Body>, hyper::Error>
{
    match req.uri().authority(){
        None => {
            eprintln!("CONNECT must contain an endpoint to connect to: {:?}", req.uri());
            let mut resp = Response::new(Body::from("CONNECT must contain an endpoint to connect to"));
            *resp.status_mut() = hyper::StatusCode::BAD_REQUEST;
            Ok(resp)
        },
        Some(a) => {
            let auth = a.clone();

            tokio::task::spawn(async move {
                match hyper::upgrade::on(&mut req).await {
                    Ok(upgraded) => {
                        let parts = upgraded.downcast::<AddrStream>().expect("upgrade not AddrStream");
                        //ignore parts.read_buf - its empty in case of HTTP CONNECT
                        let tcp_stream = parts.io.into_inner();

                        if dont_intercept.is_match(auth.as_str()) {
                            if let Err(e) = pass_throught(tcp_stream, &auth).await {
                                eprintln!("server io error for {}: {}", auth, e);
                            };
                        }else{
                            if let Err(e) = tls_mitm(ca, tcp_stream, &auth, broadcast, client, db).await {
                                eprintln!("server error for {}:", auth);
                                let mut e = &*e;
                                loop {
                                    eprintln!("\t{}", e);
                                    e = match e.source() {
                                        Some(e) => e,
                                        None => break,
                                    }
                                }
                                //TODO forward the error to the UI
                            };
                        }
                    },
                    Err(e) => eprintln!("upgrade error: {}", e),  //TODO forward the error to the UI
                }
            });
            Ok(Response::new(Body::empty()))
        }
    }
}