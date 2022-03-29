//#![deny(warnings)]

use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};
use tokio::sync::{Mutex, broadcast, RwLock};

use regex::RegexSet;
use std::sync::Arc;

use hyper_staticfile::Static;
use std::path::Path;

use websocket_codec::Message;
type Notifier = broadcast::Sender<Message>;

mod proxy;
mod server;
mod uplink;
mod ca;
mod db;
use uplink::{make_client, HTTPClient};
use ca::CA;
use db::DB;

pub struct Cfg {
    fileserver: Static,
    broadcast: Notifier,
    settings: RwLock<Settings>,
    client: HTTPClient,
}
struct Settings {
    ca: CA,
    dont_intercept: RegexSet,
    db: DB
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
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
    
    let dont_intercept = RegexSet::new(&[
        r".+\.google\..+",
        r".+\.github\.com(:[0-9]+)?",
        r".+\.docs\.rs(:[0-9]+)?",
    ]).unwrap();

    // Create the TLS acceptor.
    let db = match DB::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("could not setup a PKI:");
            //eprintln!("{:?}", e);
            return;
        }
    };
    let settings = RwLock::new(Settings {
        ca,
        dont_intercept,
        db
    });
    let client = make_client();
    let cfg = Arc::new(Cfg {
        fileserver,
        broadcast,
        settings,
        client
    });

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

async fn proxy(req: Request<Body>, cfg: Arc<Cfg>)
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
        proxy::process_connect_req(req, cfg).await
    } else {
        if req.uri().authority().is_none() {
            //Web UI
            // Received an HTTP request like:
            // ```
            // GET / HTTP/1.1
            return server::render_webui(req, cfg).await;
        }
        // Received an HTTP request like:
        // ```
        // GET www.domain.com:443/ HTTP/1.1
        // Host: www.domain.com:443
        proxy::process_http_req(req, cfg).await
    }
}