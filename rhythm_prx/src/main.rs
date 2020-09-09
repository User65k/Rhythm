//#![deny(warnings)]

use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use tokio::sync::broadcast;

use regex::RegexSet;
use std::sync::Arc;

use hyper_staticfile::Static;
use std::path::Path;

type Notifier = broadcast::Sender<String>;

mod proxy;
mod server;
mod uplink;
mod ca;
//mod db;
use uplink::{make_client, HTTPClient};
use ca::CA;
//use db::DB;

#[derive(Clone)]
struct Cfg {
    ca: CA,
    dont_intercept: Arc<RegexSet>,
    fileserver: Static,
    broadcast: Notifier,
    client: HTTPClient,
    //db: Arc<DB>
}

#[tokio::main]
async fn main() {
    let fileserver = Static::new(Path::new("./"));
    let (broadcast, _rx) = broadcast::channel(1);

    // Create the TLS acceptor.
    let ca = match CA::new() {
        Ok(ca) => ca,
        Err(e) => {
            eprintln!("could not setup a PKI:");
            eprintln!("{}", e);
            return;
        }
    };
    
    let dont_intercept = Arc::new(RegexSet::new(&[
        r".+\.google\..+",
        r".+\.github\.com(:[0-9]+)?",
        r".+\.docs\.rs(:[0-9]+)?",
    ]).unwrap());
    let client = make_client();
    //let db = Arc::new(DB::new());
    let cfg = Cfg {
        ca,
        dont_intercept,
        fileserver,
        broadcast,
        client,
        //db
    };

    let make_service = make_service_fn(move |_| {
        let cfg = cfg.clone();

        async move { Ok::<_, Infallible>(service_fn(move |req| proxy(req, cfg.clone()))) }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    let server = Server::bind(&addr).serve(make_service);
    println!("Listening on http://{}", addr);

    if let Err(e) = /*proxy::transparent_prxy().await { */ server.await {
        eprintln!("server error: {}", e);
    }
}

async fn proxy(req: Request<Body>, cfg: Cfg)
 -> Result<Response<Body>, hyper::Error>
{
    //println!("req: {:?}", req);
    //req: Request { method: CONNECT, uri: doc.rust-lang.org:443, version: HTTP/1.1, headers: {"user-agent": "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:78.0) Gecko/20100101 Firefox/78.0", "proxy-connection": "keep-alive", "connection": "keep-alive", "host": "doc.rust-lang.org:443"}, body: Body(Empty) }
    //req: Request { method: GET, uri: http://drak.li/, version: HTTP/1.1, headers: {"host": "drak.li", "user-agent": "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:78.0) Gecko/20100101 Firefox/78.0", "accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8", "accept-language": "en-US,en;q=0.5", "accept-encoding": "gzip, deflate", "connection": "keep-alive", "upgrade-insecure-requests": "1", "dnt": "1"}, body: Body(Empty) }

    if Method::CONNECT == req.method() {
        // Received an HTTP request like:
        // ```
        // CONNECT www.domain.com:443 HTTP/1.1
        // Host: www.domain.com:443
        // Proxy-Connection: Keep-Alive
        // ```
        proxy::process_connect_req(cfg.ca, req, cfg.dont_intercept, cfg.broadcast, cfg.client).await
    } else {
        if req.uri().authority().is_none() {
            //Web UI
            // Received an HTTP request like:
            // ```
            // GET / HTTP/1.1
            return server::render_webui(req, cfg.fileserver, cfg.broadcast).await;
        }
        // Received an HTTP request like:
        // ```
        // GET www.domain.com:443/ HTTP/1.1
        // Host: www.domain.com:443
        proxy::process_http_req(req, cfg.broadcast, cfg.client).await
    }
}