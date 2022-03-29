/**
I heard you like proxies, so I put proxies in your proxies

proxychains style:
InnerProxy > ... > OuterProxy > (HTTP) > HTTPS > Timeout
in rust:
TimeoutConnector<HttpsConnector<Uplink>>
*/

use hyper::{service::Service, Uri};

use std::{collections::HashMap, io};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

//use hyper::client::connect::dns::{self, GaiResolver};
use tokio_socks::{TargetAddr, tcp::Socks5Stream, IntoTargetAddr};

use hyper::{Client, Body};
use hyper_tls::HttpsConnector;
use hyper_timeout::TimeoutConnector;
use std::time::Duration;

use tokio::net::TcpStream;
use std::net::ToSocketAddrs;

pub type HTTPClient = Client<TimeoutConnector<HttpsConnector<Uplink>>, Body>;

#[derive(Debug, Clone)]
pub enum ProxyType {
    Socks5,
    HTTP
}

#[derive(Debug, Clone)]
pub struct ProxyEntry {
    ep: String,
    kind: ProxyType,
    user: String,
    pass: String
}

#[derive(Debug, Clone)]
pub struct Uplink {
    proxies: Vec<ProxyEntry>,
    hosts: HashMap<String, String>
}
//static mut PROXIES: Vec<ProxyEntry> = Vec::new();
//static mut HOSTS: HashMap<String, String> = HashMap::new();

impl Uplink {
    fn new(proxies: Vec<ProxyEntry>) -> Uplink {
        Uplink {
            proxies,
            hosts: HashMap::new()
        }
    }
    async fn proxy_to(mut socket: TcpStream, next_addr: TargetAddr<'_>, proxy: &ProxyEntry) -> Result<TcpStream, io::Error> {
        match &proxy.kind {
            ProxyType::Socks5 => {
                let socks_socket = if proxy.user.is_empty() {
                    Socks5Stream::connect_with_socket(socket, next_addr).await
                }else{
                    Socks5Stream::connect_with_password_and_socket(socket, next_addr, &proxy.user, &proxy.pass).await
                }.map_err(|se|io::Error::new(io::ErrorKind::ConnectionAborted, format!("{}",se)))?;
                Ok(socks_socket.into_inner())
            },
            ProxyType::HTTP => {
                let (host, port) = match next_addr {
                    TargetAddr::Ip(sa) => {
                        (sa.ip().to_string(), sa.port())
                    },
                    TargetAddr::Domain(cs, p) => {
                        (String::from(cs) ,p)
                    }
                };
                let buf = format!(
                    "CONNECT {0}:{1} HTTP/1.1\r\n\
                     Host: {0}:{1}\r\n\
                     {2}\
                     \r\n",
                    host,
                    port,
                    ""  //TODO Auth
                ).into_bytes();
                /*
                Proxy-Connection: keep-alive
                Connection: keep-alive
                */
                socket.write(&buf).await?;
                let mut buffer = [0; 40];
                let r = socket.read(&mut buffer).await?;

                let mut read = &buffer[..r];
                if r > 12{
                   if read.starts_with(b"HTTP/1.1 200") || read.starts_with(b"HTTP/1.0 200") {
                        loop {
                            if read.ends_with(b"\r\n\r\n") {
                                return Ok(socket);
                            }
                            // else read more
                            let r = socket.read(&mut buffer).await?;
                            if r==0 {
                                break;
                            }
                            read = &buffer[..r];
                        }
                    }
                }
                Err(io::Error::new(io::ErrorKind::InvalidData, format!("{}",host)))
            }
        }
    }
    async fn setup_stream(proxies: &Vec<ProxyEntry>, hosts: &HashMap<String, String>, uri: Uri) -> Result<TcpStream, io::Error> {
        let l = proxies.len();

        if l==0 {
            let sa = get_endpoint(uri, hosts)?
                .to_socket_addrs()?
                .next().ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionAborted, "invalid address"))?;
            return Ok(TcpStream::connect(sa).await?);
        }

        //TODO who does DNS? (we or target proxy)
        //TODO if there is only one HTTP proxy and we use HTTP (no S) it should be used without CONNECT?

        //at least one proxy
        let mut last_socket = TcpStream::connect(proxies.get(0).unwrap().ep.clone()).await?; //safe: l>0
        for i in 0..l-1 { //more than one
            let current = proxies.get(i).unwrap(); //safe: i<=l-2
            let next = proxies.get(i+1).unwrap(); //safe: i<=l-2
            let target = next.ep.clone().into_target_addr()
                .map_err(|se|io::Error::new(io::ErrorKind::ConnectionAborted, format!("{}",se)))?;
            last_socket = Uplink::proxy_to(last_socket, target, current).await?;
        }
        Ok(Uplink::proxy_to(last_socket, get_endpoint(uri, hosts)?, proxies.get(l-1).unwrap()).await?) //safe: l>0
    }
}

impl Service<Uri> for Uplink
{
    type Response = TcpStream;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        /*
        Hypers HttpConnector: waits for DNS resolver to be ready
        */
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        //TODO match uri Foxyproxy style
        let p = self.proxies.clone();
        let h = self.hosts.clone();
        let fut = async move {
            Uplink::setup_stream(&p, &h, uri).await
        };
        Box::pin(fut)
    }
}

fn get_endpoint(dst: Uri, hosts: &HashMap<String, String>) -> Result<TargetAddr<'static>,io::Error> {
    let host = match dst.host() {
        Some(s) => {
            match hosts.get(s) {
                Some(r) => r,//replace host for different DNS
                None => s,
            }
        },
        None => {
            return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "URI has no host"));
        }
    };
    let port = match dst.port() {
        Some(port) => port.as_u16(),
        None => {
            if dst.scheme() == Some(&http::uri::Scheme::HTTPS) {
                443
            } else {
                if dst.scheme().is_none() {
                    return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "URI has neither port nor scheme"));
                }
            
                80
            }
        }
    };
    Ok(format!("{}:{}",host, port).into_target_addr().map_err(|se|io::Error::new(io::ErrorKind::ConnectionAborted, format!("{}",se)))?.to_owned())
}
/*
HTTPConnector opts:
if let Some(dur) = config.keep_alive_timeout {
    sock.set_keepalive(Some(dur))
        .map_err(ConnectError::m("tcp set_keepalive error"))?;
}

if let Some(size) = config.send_buffer_size {
    sock.set_send_buffer_size(size)
        .map_err(ConnectError::m("tcp set_send_buffer_size error"))?;
}

if let Some(size) = config.recv_buffer_size {
    sock.set_recv_buffer_size(size)
        .map_err(ConnectError::m("tcp set_recv_buffer_size error"))?;
}

sock.set_nodelay(config.nodelay)
    .map_err(ConnectError::m("tcp set_nodelay error"))?;
*/



pub fn make_client() -> HTTPClient {

    let mut proxies = Vec::new();/*
    proxies.push(ProxyEntry{
        ep: "127.0.0.1:1090".to_string(),
        kind: ProxyType::Socks5,
        user: "".to_string(),
        pass: "".to_string()
    });*/
    /*proxies.push(ProxyEntry{
        ep: "127.0.0.1:8081".to_string(),
        kind: ProxyType::HTTP,
        user: "".to_string(),
        pass: "".to_string()
    });*/
    let proxies = Uplink::new(proxies);

    //let proxies = Uplink::new(Vec::new());

    let mut tls_connector_builder = native_tls::TlsConnector::builder();
    tls_connector_builder.danger_accept_invalid_certs(true);
    let tls_connector = tls_connector_builder
        .build()
        .expect("TLS initialization failed");
    let https = HttpsConnector::from((proxies, tokio_native_tls::TlsConnector::from(tls_connector)));

    let timeout = Some(Duration::from_secs(5));
    let mut toc = TimeoutConnector::new(https);
    toc.set_connect_timeout(timeout);
    toc.set_read_timeout(timeout);
    toc.set_write_timeout(timeout);
    Client::builder().build(toc) //.pool_idle_timeout(Duration::from_secs(30))
}

pub async fn make_tcp_con(uri: Uri) -> Result<TcpStream, io::Error> {
    let mut proxies = Vec::new();
    /*proxies.push(ProxyEntry{
        ep: "127.0.0.1:1090".to_string(),
        kind: ProxyType::Socks5,
        user: "".to_string(),
        pass: "".to_string()
    });*/
    let hosts = HashMap::new();
    Uplink::setup_stream(&proxies, &hosts, uri).await
}

#[cfg(test)]
mod tests {
    //manual 1x socks via ssh - passed
    //manual no proxy - passed
    //manual 1x http via burp - passed
    use super::*;
    use tokio::net::TcpListener;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[tokio::test]
    async fn proxy_name_over_http() {
        let mut listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 50];
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"CONNECT test:123 HTTP/1.1\r\nHost: test:123\r\n\r\n"[..]);
            let buf = b"HTTP/1.1 200 bla\r\n\r\n";
            socket.write(&buf[..]).await.unwrap();
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"done"[..]);
        });

        let uri = "https://test:123/foo/bar?baz".parse::<Uri>().unwrap();
        let mut proxies = Vec::new();
        proxies.push(ProxyEntry{
            ep: format!("{}", addr),
            kind: ProxyType::HTTP,
            user: "".to_string(),
            pass: "".to_string()
        });
        let hosts = HashMap::new();
        let mut s = Uplink::setup_stream(&proxies, &hosts, uri).await.unwrap();
        s.write(&b"done"[..]).await.unwrap();
    }
    #[tokio::test]
    async fn proxy_ip_over_socks() {
        let mut listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 50];
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &[5, 1, 0][..]); //Socks5, one auth, no auth
            let buf = [5, 0]; //Socks5, no auth
            socket.write(&buf[..]).await.unwrap();
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &[5, 1, 0, 1, 192, 168, 1, 1, 0, 123][..]); //Socks5, Connect, IPv4, IP, Port
            let buf = [5, 0, 0, 1, 192, 168, 1, 1, 0, 123]; //Socks5, connected, reserved, IPv4, IP, Port
            socket.write(&buf[..]).await.unwrap();
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"done"[..]);
        });

        let uri = "http://192.168.1.1:123/foo/bar?baz".parse::<Uri>().unwrap();
        let mut proxies = Vec::new();
        proxies.push(ProxyEntry{
            ep: format!("{}", addr),
            kind: ProxyType::Socks5,
            user: "".to_string(),
            pass: "".to_string()
        });
        let hosts = HashMap::new();
        let mut s = Uplink::setup_stream(&proxies, &hosts, uri).await.unwrap();
        s.write(&b"done"[..]).await.unwrap();
    }
    #[tokio::test]
    async fn no_proxy() {
        let mut listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 50];
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"done"[..]);
        });

        let uri = format!("http://{}/foo/bar?baz", addr).parse::<Uri>().unwrap();
        let proxies = Vec::new();
        let hosts = HashMap::new();
        let mut s = Uplink::setup_stream(&proxies, &hosts, uri).await.unwrap();
        s.write(&b"done"[..]).await.unwrap();
    }
    #[test]
    fn ipv6() {
        let uri = "http://[::1]:123/foo/bar?baz".parse::<Uri>().unwrap();
        let hosts = HashMap::new();
        let sa = get_endpoint(uri, &hosts).unwrap();
        assert_eq!(sa, TargetAddr::Ip(std::net::SocketAddr::new(
            std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            123
        )))
    }
    #[test]
    fn custom_hosts() {
        let uri = "http://jowhat:123/foo/bar?baz".parse::<Uri>().unwrap();
        let mut hosts = HashMap::new();
        hosts.insert("jowhat".to_string(), "[::1]".to_string());
        let sa = get_endpoint(uri, &hosts).unwrap();
        assert_eq!(sa, TargetAddr::Ip(std::net::SocketAddr::new(
            std::net::IpAddr::V6(std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)),
            123
        )))
    }
    #[tokio::test]
    async fn proxy_order() {
        let mut listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0; 50];
            let buf = b"HTTP/1.1 200 bla\r\n\r\n";

            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"CONNECT 2nd:2222 HTTP/1.1\r\nHost: 2nd:2222\r\n\r\n"[..]);
            socket.write(&buf[..]).await.unwrap();
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"CONNECT 3rd:3333 HTTP/1.1\r\nHost: 3rd:3333\r\n\r\n"[..]);
            socket.write(&buf[..]).await.unwrap();
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"CONNECT test:123 HTTP/1.1\r\nHost: test:123\r\n\r\n"[..]);
            socket.write(&buf[..]).await.unwrap();
            let r = socket.read(&mut buffer).await.unwrap();
            assert_eq!(&buffer[..r], &b"done"[..]);
        });

        let uri = "https://test:123/foo/bar?baz".parse::<Uri>().unwrap();
        let mut proxies = Vec::new();
        proxies.push(ProxyEntry{
            ep: format!("{}", addr),
            kind: ProxyType::HTTP,
            user: "".to_string(),
            pass: "".to_string()
        });
        proxies.push(ProxyEntry{
            ep: "2nd:2222".to_string(),
            kind: ProxyType::HTTP,
            user: "".to_string(),
            pass: "".to_string()
        });
        proxies.push(ProxyEntry{
            ep: "3rd:3333".to_string(),
            kind: ProxyType::HTTP,
            user: "".to_string(),
            pass: "".to_string()
        });
        let hosts = HashMap::new();
        let mut s = Uplink::setup_stream(&proxies, &hosts, uri).await.unwrap();
        s.write(&b"done"[..]).await.unwrap();
    }
}